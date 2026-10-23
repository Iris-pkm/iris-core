//! Computed properties — `formula` and `rollup` (ADR-041).
//!
//! A computed property is a **derived value**: it is evaluated on read from
//! canonical data and is never written to a node file (same principle as
//! derived relation inverses, ADR-017). Only its *definition* is canonical, in
//! `.iris/computed.yaml` (`SCHEMA_SPEC.md` §6).
//!
//! * **Formula** — one expression over the same node's raw frontmatter fields
//!   (raw, not the typed `Node`, so custom fields work) and its other computed
//!   properties.
//! * **Rollup** — a one-hop aggregate over related nodes (a canonical relation
//!   or its derived inverse). Rollups never chain.
//!
//! The expression language is small, closed and total: no loops, recursion,
//! user functions, I/O, regex or randomness, so every expression terminates.
//! Hard limits: 1 KiB of source, 256 AST nodes, depth 16. There is no `now()`;
//! `today` is a parameter, like `views::today`.
//!
//! Errors are values (`Value::Error`), not failures: a bad formula, bad data
//! or a missing field never blocks a canonical write (ADR-021), a cache
//! rebuild, or opening the vault. Evaluation is read-only — nothing here calls
//! a mutating `Engine` method or writes a file.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::Deserialize;
use serde_yaml::{Mapping, Value as Yaml};

use crate::cache::Cache;
use crate::connections::canonical_for_inverse;
use crate::error::{IrisError, IrisResult};
use crate::vault::Vault;

const MAX_EXPR_BYTES: usize = 1024;
const MAX_AST_NODES: usize = 256;
const MAX_PARSE_DEPTH: usize = 16;
/// Runtime guard on computed-property → computed-property nesting. Load-time
/// cycle rejection should make this unreachable; it is the backstop.
const MAX_EVAL_DEPTH: usize = 8;

// ---------------------------------------------------------------------------
// Values
// ---------------------------------------------------------------------------

/// Why a cell is an error. Errors propagate through dependent expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    DivideByZero,
    /// An operand had the wrong type (e.g. `"a" + 1`).
    Type,
    /// A null / missing field was used where a value is required.
    Null,
    /// A result was not a finite number.
    Number,
    /// A computed property depends on itself.
    Cycle,
    /// Computed properties nested deeper than the runtime guard.
    Depth,
    /// A rollup read another rollup (rollups never chain).
    Chained,
    /// The cache or vault could not be read for this value.
    Unavailable,
}

impl ErrorCode {
    /// Stable snake_case name, used on every surface (CLI, MCP, FFI).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DivideByZero => "divide_by_zero",
            Self::Type => "type",
            Self::Null => "null",
            Self::Number => "number",
            Self::Cycle => "cycle",
            Self::Depth => "depth",
            Self::Chained => "chained",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    Text(String),
    Date(NaiveDate),
    DateTime(DateTime<Utc>),
    List(Vec<Value>),
    Error(ErrorCode),
}

impl Value {
    /// Short type name: `null bool number text date datetime list error`.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "bool",
            Self::Number(_) => "number",
            Self::Text(_) => "text",
            Self::Date(_) => "date",
            Self::DateTime(_) => "datetime",
            Self::List(_) => "list",
            Self::Error(_) => "error",
        }
    }

    /// JSON form for the CLI and MCP. Dates are `YYYY-MM-DD`, datetimes RFC 3339,
    /// whole numbers print as integers, and an error is `{"error": "<code>"}`.
    pub fn to_json(&self) -> serde_json::Value {
        use serde_json::json;
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(b) => json!(b),
            Self::Number(n) if n.fract() == 0.0 && n.abs() < 1e15 => json!(*n as i64),
            Self::Number(n) => json!(n),
            Self::Text(s) => json!(s),
            Self::Date(d) => json!(d.to_string()),
            Self::DateTime(dt) => json!(dt.to_rfc3339()),
            Self::List(items) => {
                serde_json::Value::Array(items.iter().map(Self::to_json).collect())
            }
            Self::Error(c) => json!({ "error": c.as_str() }),
        }
    }

    /// Human-readable text (CLI default output, FFI display string).
    pub fn display(&self) -> String {
        match self {
            Self::Null => "null".into(),
            Self::Text(s) => s.clone(),
            Self::List(items) => {
                let parts: Vec<String> = items.iter().map(Self::display).collect();
                format!("[{}]", parts.join(", "))
            }
            Self::Error(c) => format!("#error({})", c.as_str()),
            // Display only: 4 decimals, trailing zeros trimmed (1/3 → 0.3333).
            // `to_json` keeps full precision for machine consumers.
            Self::Number(n) if n.fract() != 0.0 => {
                let s = format!("{n:.4}");
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            }
            other => match other.to_json() {
                serde_json::Value::String(s) => s,
                j => j.to_string(),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Expression language: lexer, parser, AST
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    Op(&'static str),
    LParen,
    RParen,
    Comma,
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            c if c.is_whitespace() => i += 1,
            '(' => {
                toks.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                toks.push(Tok::RParen);
                i += 1;
            }
            ',' => {
                toks.push(Tok::Comma);
                i += 1;
            }
            '+' | '-' | '*' | '/' | '%' => {
                toks.push(Tok::Op(match c {
                    '+' => "+",
                    '-' => "-",
                    '*' => "*",
                    '/' => "/",
                    _ => "%",
                }));
                i += 1;
            }
            '=' | '!' | '<' | '>' => {
                let next_eq = chars.get(i + 1) == Some(&'=');
                let op = match (c, next_eq) {
                    ('=', true) => "==",
                    ('!', true) => "!=",
                    ('<', true) => "<=",
                    ('>', true) => ">=",
                    ('<', false) => "<",
                    ('>', false) => ">",
                    _ => return Err(format!("unexpected `{c}` (use `==` / `!=`)")),
                };
                toks.push(Tok::Op(op));
                i += if op.len() == 2 { 2 } else { 1 };
            }
            '"' => {
                let mut s = String::new();
                i += 1;
                loop {
                    match chars.get(i) {
                        None => return Err("unterminated string".into()),
                        Some('"') => {
                            i += 1;
                            break;
                        }
                        Some('\\') => match chars.get(i + 1) {
                            Some('"') => {
                                s.push('"');
                                i += 2;
                            }
                            Some('\\') => {
                                s.push('\\');
                                i += 2;
                            }
                            _ => return Err("bad escape in string (only \\\" and \\\\)".into()),
                        },
                        Some(&ch) => {
                            s.push(ch);
                            i += 1;
                        }
                    }
                }
                toks.push(Tok::Str(s));
            }
            c if c.is_ascii_digit() => {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                if chars.get(i) == Some(&'.')
                    && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit())
                {
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                let text: String = chars[start..i].iter().collect();
                let n: f64 = text.parse().map_err(|_| format!("bad number `{text}`"))?;
                if !n.is_finite() {
                    return Err(format!(
                        "number out of range `{}...`",
                        &text[..8.min(text.len())]
                    ));
                }
                toks.push(Tok::Num(n));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                toks.push(Tok::Ident(chars[start..i].iter().collect()));
            }
            other => return Err(format!("unexpected character `{other}`")),
        }
    }
    Ok(toks)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Debug, Clone)]
enum Expr {
    Lit(Value),
    Field(String),
    Not(Box<Expr>),
    Neg(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

/// `(min, max)` argument count for each built-in function, or `None` if
/// the name is not one. The whole function set is closed (ADR-041).
fn arity(name: &str) -> Option<(usize, usize)> {
    Some(match name {
        "abs" | "floor" | "ceil" | "len" | "lower" | "upper" | "year" | "month" => (1, 1),
        "round" => (1, 2),
        "min" | "max" | "concat" => (2, 8),
        "contains" | "days_between" | "add_days" => (2, 2),
        "today" => (0, 0),
        "coalesce" => (1, 8),
        "if" => (3, 3),
        _ => return None,
    })
}

const KEYWORDS: [&str; 6] = ["true", "false", "null", "and", "or", "not"];

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
    nodes: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn bump(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn node(&mut self, e: Expr) -> Result<Expr, String> {
        self.nodes += 1;
        if self.nodes > MAX_AST_NODES {
            return Err(format!("expression too large (max {MAX_AST_NODES} nodes)"));
        }
        Ok(e)
    }

    /// Binding power of the infix operator at the cursor, if any.
    fn infix(&self) -> Option<(BinOp, u8)> {
        Some(match self.peek()? {
            Tok::Ident(w) if w == "or" => (BinOp::Or, 1),
            Tok::Ident(w) if w == "and" => (BinOp::And, 2),
            Tok::Op("==") => (BinOp::Eq, 4),
            Tok::Op("!=") => (BinOp::Ne, 4),
            Tok::Op("<") => (BinOp::Lt, 4),
            Tok::Op("<=") => (BinOp::Le, 4),
            Tok::Op(">") => (BinOp::Gt, 4),
            Tok::Op(">=") => (BinOp::Ge, 4),
            Tok::Op("+") => (BinOp::Add, 5),
            Tok::Op("-") => (BinOp::Sub, 5),
            Tok::Op("*") => (BinOp::Mul, 6),
            Tok::Op("/") => (BinOp::Div, 6),
            Tok::Op("%") => (BinOp::Rem, 6),
            _ => return None,
        })
    }

    fn expr(&mut self, min_bp: u8, depth: usize) -> Result<Expr, String> {
        if depth > MAX_PARSE_DEPTH {
            return Err(format!(
                "expression nested too deeply (max {MAX_PARSE_DEPTH})"
            ));
        }
        let mut lhs = match self.bump() {
            None => return Err("unexpected end of expression".into()),
            Some(Tok::Num(n)) => self.node(Expr::Lit(Value::Number(n)))?,
            Some(Tok::Str(s)) => self.node(Expr::Lit(Value::Text(s)))?,
            Some(Tok::LParen) => {
                let inner = self.expr(0, depth + 1)?;
                match self.bump() {
                    Some(Tok::RParen) => inner,
                    _ => return Err("missing `)`".into()),
                }
            }
            Some(Tok::Op("-")) => {
                let inner = self.expr(7, depth + 1)?;
                self.node(Expr::Neg(Box::new(inner)))?
            }
            Some(Tok::Ident(w)) => match w.as_str() {
                "true" => self.node(Expr::Lit(Value::Bool(true)))?,
                "false" => self.node(Expr::Lit(Value::Bool(false)))?,
                "null" => self.node(Expr::Lit(Value::Null))?,
                "not" => {
                    let inner = self.expr(3, depth + 1)?;
                    self.node(Expr::Not(Box::new(inner)))?
                }
                "and" | "or" => return Err(format!("`{w}` needs a left operand")),
                _ if self.peek() == Some(&Tok::LParen) => {
                    self.bump();
                    let Some((min, max)) = arity(&w) else {
                        return Err(format!("unknown function `{w}`"));
                    };
                    let mut args = Vec::new();
                    if self.peek() != Some(&Tok::RParen) {
                        loop {
                            args.push(self.expr(0, depth + 1)?);
                            match self.bump() {
                                Some(Tok::Comma) => continue,
                                Some(Tok::RParen) => break,
                                _ => return Err(format!("bad argument list for `{w}`")),
                            }
                        }
                    } else {
                        self.bump();
                    }
                    if args.len() < min || args.len() > max {
                        return Err(format!(
                            "`{w}` takes {} argument(s), got {}",
                            if min == max {
                                min.to_string()
                            } else {
                                format!("{min}..{max}")
                            },
                            args.len()
                        ));
                    }
                    self.node(Expr::Call(w, args))?
                }
                _ => self.node(Expr::Field(w))?,
            },
            Some(t) => return Err(format!("unexpected token {t:?}")),
        };
        while let Some((op, bp)) = self.infix() {
            if bp < min_bp {
                break;
            }
            self.bump();
            let rhs = self.expr(bp + 1, depth + 1)?;
            lhs = self.node(Expr::Bin(op, Box::new(lhs), Box::new(rhs)))?;
        }
        Ok(lhs)
    }
}

fn parse_expr(src: &str) -> Result<Expr, String> {
    if src.len() > MAX_EXPR_BYTES {
        return Err(format!("expression too long (max {MAX_EXPR_BYTES} bytes)"));
    }
    let toks = lex(src)?;
    if toks.is_empty() {
        return Err("empty expression".into());
    }
    let mut p = Parser {
        toks,
        pos: 0,
        nodes: 0,
    };
    let e = p.expr(0, 0)?;
    if p.pos < p.toks.len() {
        return Err(format!("unexpected trailing {:?}", p.toks[p.pos]));
    }
    Ok(e)
}

/// Every field/computed-property name an expression reads.
fn idents(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Lit(_) => {}
        Expr::Field(n) => out.push(n.clone()),
        Expr::Not(a) | Expr::Neg(a) => idents(a, out),
        Expr::Bin(_, a, b) => {
            idents(a, out);
            idents(b, out);
        }
        Expr::Call(_, args) => args.iter().for_each(|a| idents(a, out)),
    }
}

// ---------------------------------------------------------------------------
// Definitions: `.iris/computed.yaml`
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RollupFn {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    Earliest,
    Latest,
    Any,
    All,
    Percent,
    Unique,
}

impl RollupFn {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "count" => Self::Count,
            "sum" => Self::Sum,
            "avg" => Self::Avg,
            "min" => Self::Min,
            "max" => Self::Max,
            "earliest" => Self::Earliest,
            "latest" => Self::Latest,
            "any" => Self::Any,
            "all" => Self::All,
            "percent" => Self::Percent,
            "unique" => Self::Unique,
            _ => return None,
        })
    }

    /// `count`/`percent` use the `where` predicate alone; the rest aggregate a property.
    fn needs_property(self) -> bool {
        !matches!(self, Self::Count | Self::Percent)
    }
}

#[derive(Debug, Clone)]
struct Rollup {
    relation: String,
    incoming: bool,
    where_: Option<Expr>,
    property: Option<String>,
    func: RollupFn,
}

#[derive(Debug, Clone)]
enum Kind {
    Formula(Expr),
    Rollup(Rollup),
}

#[derive(Debug, Clone)]
pub struct Computed {
    pub name: String,
    kind: Kind,
}

impl Computed {
    pub fn is_rollup(&self) -> bool {
        matches!(self.kind, Kind::Rollup(_))
    }
}

#[derive(Debug, Clone, Default)]
pub struct Definitions {
    by_type: BTreeMap<String, Vec<Computed>>,
}

impl Definitions {
    pub fn is_empty(&self) -> bool {
        self.by_type.is_empty()
    }

    pub fn for_type(&self, node_type: &str) -> &[Computed] {
        self.by_type.get(node_type).map_or(&[], Vec::as_slice)
    }

    pub fn entries(&self) -> impl Iterator<Item = (&str, &Computed)> {
        self.by_type
            .iter()
            .flat_map(|(ty, values)| values.iter().map(move |value| (ty.as_str(), value)))
    }

    fn get(&self, node_type: &str, name: &str) -> Option<&Computed> {
        self.for_type(node_type).iter().find(|c| c.name == name)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDef {
    name: String,
    kind: String,
    expr: Option<String>,
    relation: Option<String>,
    direction: Option<String>,
    #[serde(rename = "where")]
    where_: Option<String>,
    property: Option<String>,
    #[serde(rename = "fn")]
    func: Option<String>,
}

fn bad(type_name: &str, name: &str, msg: impl std::fmt::Display) -> IrisError {
    IrisError::Validation(format!("computed property `{type_name}.{name}`: {msg}"))
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !KEYWORDS.contains(&s)
        && arity(s).is_none()
}

/// Read `.iris/computed.yaml` under `vault_root`. A missing file is an empty
/// set of definitions, not an error.
pub fn load(vault_root: &Path) -> IrisResult<Definitions> {
    let path = vault_root.join(".iris").join("computed.yaml");
    match std::fs::read_to_string(&path) {
        Ok(text) => parse_definitions(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Definitions::default()),
        Err(e) => Err(IrisError::Io(e)),
    }
}

/// Parse and validate definitions: syntax, function names and arity, limits,
/// rollup field combinations, and an acyclic dependency graph.
pub fn parse_definitions(yaml: &str) -> IrisResult<Definitions> {
    if yaml.trim().is_empty() {
        return Ok(Definitions::default());
    }
    let raw: BTreeMap<String, Vec<RawDef>> = serde_yaml::from_str(yaml)
        .map_err(|e| IrisError::Validation(format!("computed.yaml: {e}")))?;
    let mut defs = Definitions::default();
    for (ty, list) in raw {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for r in list {
            if !is_ident(&r.name) {
                return Err(bad(
                    &ty,
                    &r.name,
                    "name must be an identifier (letters, digits, `_`), not a keyword or function",
                ));
            }
            if !seen.insert(r.name.clone()) {
                return Err(bad(&ty, &r.name, "defined twice for this type"));
            }
            let parse = |what: &str, src: &str| {
                parse_expr(src).map_err(|e| bad(&ty, &r.name, format!("{what}: {e}")))
            };
            let kind = match r.kind.as_str() {
                "formula" => {
                    if r.relation.is_some()
                        || r.direction.is_some()
                        || r.where_.is_some()
                        || r.property.is_some()
                        || r.func.is_some()
                    {
                        return Err(bad(&ty, &r.name, "a formula takes only `expr`"));
                    }
                    let src = r
                        .expr
                        .as_deref()
                        .ok_or_else(|| bad(&ty, &r.name, "formula needs `expr`"))?;
                    Kind::Formula(parse("expr", src)?)
                }
                "rollup" => {
                    if r.expr.is_some() {
                        return Err(bad(&ty, &r.name, "a rollup takes no `expr`"));
                    }
                    let relation = r
                        .relation
                        .clone()
                        .filter(|s| !s.trim().is_empty())
                        .ok_or_else(|| bad(&ty, &r.name, "rollup needs `relation`"))?;
                    if let Some(canonical) = canonical_for_inverse(&relation) {
                        return Err(bad(
                            &ty,
                            &r.name,
                            format!("`{relation}` is a derived inverse; use `relation: {canonical}` with `direction: in`"),
                        ));
                    }
                    let incoming = match r.direction.as_deref().unwrap_or("out") {
                        "out" => false,
                        "in" => true,
                        other => {
                            return Err(bad(
                                &ty,
                                &r.name,
                                format!("direction must be `out` or `in`, not `{other}`"),
                            ))
                        }
                    };
                    let func_name = r
                        .func
                        .as_deref()
                        .ok_or_else(|| bad(&ty, &r.name, "rollup needs `fn`"))?;
                    let func = RollupFn::parse(func_name).ok_or_else(|| {
                        bad(&ty, &r.name, format!("unknown fn `{func_name}` (count, sum, avg, min, max, earliest, latest, any, all, percent, unique)"))
                    })?;
                    if func.needs_property() != r.property.is_some() {
                        return Err(bad(
                            &ty,
                            &r.name,
                            if func.needs_property() {
                                format!("`{func_name}` needs `property`")
                            } else {
                                format!("`{func_name}` takes no `property` (it uses `where`)")
                            },
                        ));
                    }
                    if let Some(p) = &r.property {
                        if !is_ident(p) {
                            return Err(bad(
                                &ty,
                                &r.name,
                                format!("`property: {p}` must be a field or formula name"),
                            ));
                        }
                    }
                    if func == RollupFn::Percent && r.where_.is_none() {
                        return Err(bad(&ty, &r.name, "`percent` needs a `where` predicate"));
                    }
                    let where_ = match &r.where_ {
                        Some(src) => Some(parse("where", src)?),
                        None => None,
                    };
                    Kind::Rollup(Rollup {
                        relation,
                        incoming,
                        where_,
                        property: r.property.clone(),
                        func,
                    })
                }
                other => {
                    return Err(bad(
                        &ty,
                        &r.name,
                        format!("kind must be `formula` or `rollup`, not `{other}`"),
                    ))
                }
            };
            out.push(Computed { name: r.name, kind });
        }
        defs.by_type.insert(ty, out);
    }
    check_graph(&defs)?;
    Ok(defs)
}

/// Reject cycles and rollup-of-rollup. A rollup reads a *related* node whose
/// type is unknown statically, so it conservatively depends on every formula
/// of that name in any type.
fn check_graph(defs: &Definitions) -> IrisResult<()> {
    type Key = (String, String);
    let mut edges: HashMap<Key, Vec<Key>> = HashMap::new();
    let formulas_named = |name: &str| -> Vec<Key> {
        defs.by_type
            .iter()
            .flat_map(|(t, v)| v.iter().map(move |c| (t, c)))
            .filter(|(_, c)| c.name == name && !c.is_rollup())
            .map(|(t, c)| (t.clone(), c.name.clone()))
            .collect()
    };
    let rollup_named = |name: &str| {
        defs.by_type
            .values()
            .flatten()
            .any(|c| c.name == name && c.is_rollup())
    };

    for (ty, list) in &defs.by_type {
        for c in list {
            let key = (ty.clone(), c.name.clone());
            let mut deps: Vec<Key> = Vec::new();
            match &c.kind {
                Kind::Formula(e) => {
                    let mut ids = Vec::new();
                    idents(e, &mut ids);
                    deps.extend(
                        ids.into_iter()
                            .filter(|id| defs.get(ty, id).is_some())
                            .map(|id| (ty.clone(), id)),
                    );
                }
                Kind::Rollup(r) => {
                    let mut names = Vec::new();
                    if let Some(p) = &r.property {
                        let formulas = formulas_named(p);
                        if formulas.is_empty() && rollup_named(p) {
                            return Err(bad(
                                ty,
                                &c.name,
                                format!(
                                    "`property: {p}` is a rollup; rollups never chain (ADR-041)"
                                ),
                            ));
                        }
                        names.push(p.clone());
                    }
                    if let Some(w) = &r.where_ {
                        idents(w, &mut names);
                    }
                    for n in names {
                        deps.extend(formulas_named(&n));
                    }
                }
            }
            edges.insert(key, deps);
        }
    }

    // Depth-first search with an explicit path, to report the cycle.
    fn visit(
        node: &(String, String),
        edges: &HashMap<(String, String), Vec<(String, String)>>,
        path: &mut Vec<(String, String)>,
        done: &mut HashSet<(String, String)>,
    ) -> Option<Vec<(String, String)>> {
        if let Some(i) = path.iter().position(|p| p == node) {
            return Some(path[i..].to_vec());
        }
        if done.contains(node) {
            return None;
        }
        path.push(node.clone());
        for next in edges.get(node).into_iter().flatten() {
            if let Some(c) = visit(next, edges, path, done) {
                return Some(c);
            }
        }
        path.pop();
        done.insert(node.clone());
        None
    }
    let mut done = HashSet::new();
    let mut keys: Vec<_> = edges.keys().cloned().collect();
    keys.sort();
    for k in keys {
        if let Some(cycle) = visit(&k, &edges, &mut Vec::new(), &mut done) {
            let mut names: Vec<String> = cycle.iter().map(|(t, n)| format!("{t}.{n}")).collect();
            names.push(names[0].clone()); // close the loop for display
            return Err(IrisError::Validation(format!(
                "computed properties form a cycle: {}",
                names.join(" -> ")
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

struct NodeData {
    node_type: String,
    fields: Mapping,
}

impl NodeData {
    fn is_deleted_or_template(&self) -> bool {
        let truthy = |k: &str| match self.fields.get(k) {
            Some(Yaml::Null) | None => false,
            Some(Yaml::Bool(b)) => *b,
            Some(_) => true,
        };
        truthy("deleted_at") || truthy("is_template")
    }
}

struct Evaluator<'a> {
    vault: &'a Vault,
    cache: &'a Cache,
    defs: &'a Definitions,
    today: NaiveDate,
    nodes: HashMap<String, Option<Rc<NodeData>>>,
    memo: HashMap<(String, String), Value>,
    stack: Vec<(String, String)>,
}

/// Convert a raw frontmatter value. `YYYY-MM-DD` strings are dates and RFC 3339
/// strings are datetimes, so date arithmetic works on stored fields.
fn field_value(y: &Yaml) -> Value {
    match y {
        Yaml::Null => Value::Null,
        Yaml::Bool(b) => Value::Bool(*b),
        Yaml::Number(n) => n
            .as_f64()
            .map_or(Value::Error(ErrorCode::Number), Value::Number),
        Yaml::String(s) => {
            if s.len() == 10 {
                if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                    return Value::Date(d);
                }
            }
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return Value::DateTime(dt.with_timezone(&Utc));
            }
            Value::Text(s.clone())
        }
        Yaml::Sequence(items) => Value::List(items.iter().map(field_value).collect()),
        Yaml::Mapping(_) | Yaml::Tagged(_) => Value::Error(ErrorCode::Type),
    }
}

fn err_of(v: &Value) -> Option<ErrorCode> {
    match v {
        Value::Error(c) => Some(*c),
        _ => None,
    }
}

fn num(v: &Value) -> Result<f64, ErrorCode> {
    match v {
        Value::Number(n) => Ok(*n),
        Value::Null => Err(ErrorCode::Null),
        Value::Error(c) => Err(*c),
        _ => Err(ErrorCode::Type),
    }
}

fn finite(n: f64) -> Value {
    if n.is_finite() {
        Value::Number(n)
    } else {
        Value::Error(ErrorCode::Number)
    }
}

fn as_date(v: &Value) -> Result<NaiveDate, ErrorCode> {
    match v {
        Value::Date(d) => Ok(*d),
        Value::DateTime(dt) => Ok(dt.date_naive()),
        Value::Null => Err(ErrorCode::Null),
        Value::Error(c) => Err(*c),
        _ => Err(ErrorCode::Type),
    }
}

fn text_of(v: &Value) -> Result<String, ErrorCode> {
    match v {
        Value::Text(s) => Ok(s.clone()),
        Value::Number(n) if n.fract() == 0.0 && n.abs() < 1e15 => Ok(format!("{}", *n as i64)),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Date(d) => Ok(d.to_string()),
        Value::DateTime(dt) => Ok(dt.to_rfc3339()),
        Value::Null => Err(ErrorCode::Null),
        Value::Error(c) => Err(*c),
        Value::List(_) => Err(ErrorCode::Type),
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    // Mismatched types are simply unequal (not an error); `null == null` holds.
    a == b
}

fn compare(a: &Value, b: &Value) -> Result<std::cmp::Ordering, ErrorCode> {
    use Value::*;
    match (a, b) {
        (Number(x), Number(y)) => x.partial_cmp(y).ok_or(ErrorCode::Number),
        (Text(x), Text(y)) => Ok(x.cmp(y)),
        (Date(x), Date(y)) => Ok(x.cmp(y)),
        (DateTime(x), DateTime(y)) => Ok(x.cmp(y)),
        (Error(c), _) | (_, Error(c)) => Err(*c),
        (Null, _) | (_, Null) => Err(ErrorCode::Null),
        _ => Err(ErrorCode::Type),
    }
}

impl<'a> Evaluator<'a> {
    fn load(&mut self, id: &str) -> Option<Rc<NodeData>> {
        if let Some(hit) = self.nodes.get(id) {
            return hit.clone();
        }
        let loaded = self.read_node(id).map(Rc::new);
        self.nodes.insert(id.to_string(), loaded.clone());
        loaded
    }

    fn read_node(&self, id: &str) -> Option<NodeData> {
        let row = self
            .cache
            .query_nodes("SELECT * FROM nodes WHERE id = ?1", [id])
            .ok()?
            .pop()?;
        let parsed = self.vault.read_node(&row.path).ok()?;
        let fields = serde_yaml::from_str::<Mapping>(&parsed.raw_frontmatter).ok()?;
        Some(NodeData {
            node_type: row.node_type,
            fields,
        })
    }

    /// The value of `name` on node `id`: a computed property of its type if
    /// one is defined, else a raw frontmatter field (missing = `Null`).
    fn value_of(&mut self, id: &str, name: &str) -> Value {
        let Some(node) = self.load(id) else {
            return Value::Null;
        };
        let defs = self.defs;
        match defs.get(&node.node_type, name) {
            Some(def) => self.computed(id, def),
            None => node.fields.get(name).map_or(Value::Null, field_value),
        }
    }

    fn computed(&mut self, id: &str, def: &'a Computed) -> Value {
        let key = (id.to_string(), def.name.clone());
        if let Some(v) = self.memo.get(&key) {
            return v.clone();
        }
        if self.stack.contains(&key) {
            return Value::Error(ErrorCode::Cycle);
        }
        if self.stack.len() >= MAX_EVAL_DEPTH {
            return Value::Error(ErrorCode::Depth);
        }
        self.stack.push(key.clone());
        let v = match &def.kind {
            Kind::Formula(e) => self.eval(e, id),
            Kind::Rollup(r) => self.rollup(id, r),
        };
        self.stack.pop();
        self.memo.insert(key, v.clone());
        v
    }

    fn eval(&mut self, e: &Expr, id: &str) -> Value {
        match e {
            Expr::Lit(v) => v.clone(),
            Expr::Field(name) => self.value_of(id, name),
            Expr::Neg(a) => match num(&self.eval(a, id)) {
                Ok(n) => finite(-n),
                Err(c) => Value::Error(c),
            },
            Expr::Not(a) => match self.eval(a, id) {
                Value::Bool(b) => Value::Bool(!b),
                Value::Error(c) => Value::Error(c),
                Value::Null => Value::Error(ErrorCode::Null),
                _ => Value::Error(ErrorCode::Type),
            },
            Expr::Bin(BinOp::And | BinOp::Or, ..) => self.logic(e, id),
            Expr::Bin(op, a, b) => {
                let (x, y) = (self.eval(a, id), self.eval(b, id));
                if let Some(c) = err_of(&x).or_else(|| err_of(&y)) {
                    return Value::Error(c);
                }
                binary(*op, &x, &y)
            }
            Expr::Call(name, args) => self.call(name, args, id),
        }
    }

    fn logic(&mut self, e: &Expr, id: &str) -> Value {
        let Expr::Bin(op, a, b) = e else {
            return Value::Error(ErrorCode::Type);
        };
        let as_bool = |v: Value| match v {
            Value::Bool(b) => Ok(b),
            Value::Error(c) => Err(c),
            Value::Null => Err(ErrorCode::Null),
            _ => Err(ErrorCode::Type),
        };
        let left = match as_bool(self.eval(a, id)) {
            Ok(b) => b,
            Err(c) => return Value::Error(c),
        };
        // Short-circuit: the right side is not evaluated when the left decides.
        if (*op == BinOp::Or && left) || (*op == BinOp::And && !left) {
            return Value::Bool(left);
        }
        match as_bool(self.eval(b, id)) {
            Ok(r) => Value::Bool(r),
            Err(c) => Value::Error(c),
        }
    }

    fn call(&mut self, name: &str, args: &[Expr], id: &str) -> Value {
        // Lazy forms first: only the chosen branch is evaluated.
        match name {
            "if" => {
                return match self.eval(&args[0], id) {
                    Value::Bool(true) => self.eval(&args[1], id),
                    Value::Bool(false) => self.eval(&args[2], id),
                    Value::Error(c) => Value::Error(c),
                    Value::Null => Value::Error(ErrorCode::Null),
                    _ => Value::Error(ErrorCode::Type),
                };
            }
            "coalesce" => {
                for a in args {
                    match self.eval(a, id) {
                        Value::Null => continue,
                        other => return other,
                    }
                }
                return Value::Null;
            }
            _ => {}
        }
        let vals: Vec<Value> = args.iter().map(|a| self.eval(a, id)).collect();
        if let Some(c) = vals.iter().find_map(err_of) {
            return Value::Error(c);
        }
        builtin(name, &vals, self.today)
    }

    fn rollup(&mut self, id: &str, r: &Rollup) -> Value {
        let rows = match self.cache.relations_touching(id) {
            Ok(rows) => rows,
            Err(_) => return Value::Error(ErrorCode::Unavailable),
        };
        let mut related: Vec<String> = rows
            .into_iter()
            .filter(|(rel, _, _)| *rel == r.relation)
            .filter_map(|(_, src, tgt)| match (r.incoming, src == id, tgt == id) {
                (false, true, _) => Some(tgt),
                (true, _, true) => Some(src),
                _ => None,
            })
            .collect();
        related.sort();
        related.dedup();

        let mut total = 0usize;
        let mut kept: Vec<String> = Vec::new();
        for rid in related {
            let Some(node) = self.load(&rid) else {
                continue; // dangling relation: integrity.rs's job to flag
            };
            if node.is_deleted_or_template() {
                continue;
            }
            total += 1;
            let keep = match &r.where_ {
                None => true,
                Some(w) => match self.eval(w, &rid) {
                    Value::Bool(b) => b,
                    Value::Null => false,
                    Value::Error(c) => return Value::Error(c),
                    _ => return Value::Error(ErrorCode::Type),
                },
            };
            if keep {
                kept.push(rid);
            }
        }

        match r.func {
            RollupFn::Count => return Value::Number(kept.len() as f64),
            RollupFn::Percent => {
                return if total == 0 {
                    Value::Null
                } else {
                    Value::Number(kept.len() as f64 / total as f64 * 100.0)
                };
            }
            _ => {}
        }

        let prop = r.property.as_deref().unwrap_or_default();
        let mut values = Vec::with_capacity(kept.len());
        for rid in &kept {
            let Some(node) = self.load(rid) else { continue };
            // Rollups never chain: reading a related node's rollup is an error.
            if self
                .defs
                .get(&node.node_type, prop)
                .is_some_and(Computed::is_rollup)
            {
                return Value::Error(ErrorCode::Chained);
            }
            let v = self.value_of(rid, prop);
            if let Value::Error(c) = v {
                return Value::Error(c);
            }
            values.push(v);
        }
        aggregate(r.func, values)
    }
}

fn binary(op: BinOp, x: &Value, y: &Value) -> Value {
    use std::cmp::Ordering::*;
    match op {
        BinOp::Eq => Value::Bool(values_equal(x, y)),
        BinOp::Ne => Value::Bool(!values_equal(x, y)),
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => match compare(x, y) {
            Ok(o) => Value::Bool(match op {
                BinOp::Lt => o == Less,
                BinOp::Le => o != Greater,
                BinOp::Gt => o == Greater,
                _ => o != Less,
            }),
            Err(c) => Value::Error(c),
        },
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
            let (a, b) = match (num(x), num(y)) {
                (Ok(a), Ok(b)) => (a, b),
                (Err(c), _) | (_, Err(c)) => return Value::Error(c),
            };
            match op {
                BinOp::Add => finite(a + b),
                BinOp::Sub => finite(a - b),
                BinOp::Mul => finite(a * b),
                _ if b == 0.0 => Value::Error(ErrorCode::DivideByZero),
                BinOp::Div => finite(a / b),
                _ => finite(a % b),
            }
        }
        BinOp::And | BinOp::Or => Value::Error(ErrorCode::Type), // handled lazily in `logic`
    }
}

fn builtin(name: &str, v: &[Value], today: NaiveDate) -> Value {
    let r: Result<Value, ErrorCode> = (|| {
        Ok(match name {
            "today" => Value::Date(today),
            "abs" => finite(num(&v[0])?.abs()),
            "floor" => finite(num(&v[0])?.floor()),
            "ceil" => finite(num(&v[0])?.ceil()),
            "round" => {
                let digits = match v.get(1) {
                    Some(d) => num(d)?.clamp(0.0, 12.0) as i32,
                    None => 0,
                };
                let scale = 10f64.powi(digits);
                finite((num(&v[0])? * scale).round() / scale)
            }
            "min" | "max" => {
                let mut best = num(&v[0])?;
                for x in &v[1..] {
                    let n = num(x)?;
                    best = if name == "min" {
                        best.min(n)
                    } else {
                        best.max(n)
                    };
                }
                finite(best)
            }
            "len" => match &v[0] {
                Value::Text(s) => Value::Number(s.chars().count() as f64),
                Value::List(l) => Value::Number(l.len() as f64),
                Value::Null => return Err(ErrorCode::Null),
                _ => return Err(ErrorCode::Type),
            },
            "concat" => {
                let mut s = String::new();
                for x in v {
                    s.push_str(&text_of(x)?);
                }
                Value::Text(s)
            }
            "lower" => Value::Text(text_of(&v[0])?.to_lowercase()),
            "upper" => Value::Text(text_of(&v[0])?.to_uppercase()),
            "contains" => match (&v[0], &v[1]) {
                (Value::Text(h), Value::Text(n)) => Value::Bool(h.contains(n.as_str())),
                (Value::List(items), needle) => {
                    Value::Bool(items.iter().any(|i| values_equal(i, needle)))
                }
                (Value::Null, _) | (_, Value::Null) => return Err(ErrorCode::Null),
                _ => return Err(ErrorCode::Type),
            },
            "days_between" => Value::Number((as_date(&v[1])? - as_date(&v[0])?).num_days() as f64),
            "add_days" => {
                let d = as_date(&v[0])?;
                let n = num(&v[1])?;
                if n.fract() != 0.0 || n.abs() > 1e6 {
                    return Err(ErrorCode::Number);
                }
                Value::Date(
                    d.checked_add_signed(chrono::Duration::days(n as i64))
                        .ok_or(ErrorCode::Number)?,
                )
            }
            "year" => Value::Number(f64::from(as_date(&v[0])?.year())),
            "month" => Value::Number(f64::from(as_date(&v[0])?.month())),
            _ => return Err(ErrorCode::Type),
        })
    })();
    r.unwrap_or_else(Value::Error)
}

fn aggregate(func: RollupFn, values: Vec<Value>) -> Value {
    // `Null` entries (a related node lacking the property) are skipped.
    let values: Vec<Value> = values.into_iter().filter(|v| *v != Value::Null).collect();
    if values.is_empty() {
        return Value::Null; // empty set: every fn but count/percent is null
    }
    match func {
        RollupFn::Sum | RollupFn::Avg | RollupFn::Min | RollupFn::Max => {
            let mut nums = Vec::with_capacity(values.len());
            for v in &values {
                match v {
                    Value::Number(n) => nums.push(*n),
                    _ => return Value::Error(ErrorCode::Type),
                }
            }
            match func {
                RollupFn::Sum => finite(nums.iter().sum()),
                RollupFn::Avg => finite(nums.iter().sum::<f64>() / nums.len() as f64),
                RollupFn::Min => finite(nums.iter().copied().fold(f64::INFINITY, f64::min)),
                _ => finite(nums.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
            }
        }
        RollupFn::Earliest | RollupFn::Latest => {
            let mut best = values[0].clone();
            for v in &values[1..] {
                match compare(v, &best) {
                    Ok(o) => {
                        let better = if func == RollupFn::Earliest {
                            o == std::cmp::Ordering::Less
                        } else {
                            o == std::cmp::Ordering::Greater
                        };
                        if better {
                            best = v.clone();
                        }
                    }
                    Err(c) => return Value::Error(c),
                }
            }
            match best {
                Value::Date(_) | Value::DateTime(_) => best,
                _ => Value::Error(ErrorCode::Type),
            }
        }
        RollupFn::Any | RollupFn::All => {
            let mut bools = Vec::with_capacity(values.len());
            for v in &values {
                match v {
                    Value::Bool(b) => bools.push(*b),
                    _ => return Value::Error(ErrorCode::Type),
                }
            }
            Value::Bool(if func == RollupFn::Any {
                bools.iter().any(|b| *b)
            } else {
                bools.iter().all(|b| *b)
            })
        }
        RollupFn::Unique => {
            let mut out: Vec<Value> = Vec::new();
            for v in values {
                if !out.contains(&v) {
                    out.push(v);
                }
            }
            Value::List(out)
        }
        RollupFn::Count | RollupFn::Percent => Value::Error(ErrorCode::Type), // handled earlier
    }
}

/// The caller-side "today" for shells that have no date of their own: the
/// machine's local calendar date. The evaluator itself never reads the clock
/// (ADR-041); only callers do.
pub fn today_local() -> NaiveDate {
    chrono::Local::now().date_naive()
}

/// Evaluate every computed property defined for `node_id`'s type.
///
/// Read-only: it never writes a file or the cache. `today` is supplied by the
/// caller (as `views::today` does), so the result is a pure function of the
/// vault, the definitions and the date. Errors inside a property are returned
/// as `Value::Error` in the map; `Err` is only for an unknown `node_id`.
pub fn evaluate(
    vault: &Vault,
    cache: &Cache,
    defs: &Definitions,
    node_id: &str,
    today: NaiveDate,
) -> IrisResult<BTreeMap<String, Value>> {
    let mut ev = Evaluator {
        vault,
        cache,
        defs,
        today,
        nodes: HashMap::new(),
        memo: HashMap::new(),
        stack: Vec::new(),
    };
    let node = ev
        .load(node_id)
        .ok_or_else(|| IrisError::Validation(format!("no node with id {node_id}")))?;
    let mut out = BTreeMap::new();
    for def in defs.for_type(&node.node_type) {
        let v = ev.computed(node_id, def);
        out.insert(def.name.clone(), v);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "iris-computed-test-{}-{label}-{nanos}-{n}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn md(id: &str, ty: &str, extra: &str) -> String {
        format!(
            "---\nid: {id}\ntype: {ty}\ncreated: 2026-01-15T09:30:00Z\nmodified: 2026-01-15T09:30:00Z\nschema_version: 1\n{extra}---\n\nbody\n"
        )
    }

    fn task(id: &str, project: &str, extra: &str) -> String {
        md(
            id,
            "task",
            &format!("{extra}relations:\n  - type: parent_project\n    target: {project}\n"),
        )
    }

    struct Fx {
        _dir: TempDir,
        vault: Vault,
        cache: Cache,
    }

    fn fixture(label: &str, files: &[(&str, String)]) -> Fx {
        let dir = TempDir::new(label);
        let vault = Vault::create(&dir.0).unwrap();
        for (path, contents) in files {
            vault.write_node(path, contents).unwrap();
        }
        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();
        Fx {
            _dir: dir,
            vault,
            cache,
        }
    }

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn jan15() -> NaiveDate {
        d("2026-01-15")
    }

    fn eval(fx: &Fx, defs: &str, id: &str, today: NaiveDate) -> BTreeMap<String, Value> {
        let defs = parse_definitions(defs).unwrap();
        evaluate(&fx.vault, &fx.cache, &defs, id, today).unwrap()
    }

    /// Evaluate one formula `expr` on a note carrying `extra` frontmatter.
    fn formula(expr: &str, extra: &str) -> Value {
        let fx = fixture("formula", &[("n/a.md", md("A", "note", extra))]);
        let yaml = format!(
            "note:\n  - name: x\n    kind: formula\n    expr: '{}'\n",
            expr.replace('\'', "''")
        );
        eval(&fx, &yaml, "A", jan15()).remove("x").unwrap()
    }

    fn num_(n: f64) -> Value {
        Value::Number(n)
    }

    fn text(s: &str) -> Value {
        Value::Text(s.into())
    }

    // -- operators and functions -------------------------------------------

    #[test]
    fn arithmetic_precedence_and_unary() {
        assert_eq!(formula("1 + 2 * 3", ""), num_(7.0));
        assert_eq!(formula("(1 + 2) * 3", ""), num_(9.0));
        assert_eq!(formula("10 - 4 - 3", ""), num_(3.0));
        assert_eq!(formula("-2 * 3", ""), num_(-6.0));
        assert_eq!(formula("7 % 4", ""), num_(3.0));
        assert_eq!(formula("1 / 4", ""), num_(0.25));
    }

    #[test]
    fn comparisons_logic_and_short_circuit() {
        assert_eq!(formula("1 < 2 and 2 <= 2", ""), Value::Bool(true));
        assert_eq!(formula("not (1 == 2)", ""), Value::Bool(true));
        assert_eq!(formula("not 1 == 2", ""), Value::Bool(true));
        assert_eq!(formula("1 != 1 or 3 >= 3", ""), Value::Bool(true));
        assert_eq!(formula("\"a\" < \"b\"", ""), Value::Bool(true));
        // Short-circuit: the erroring right side is never evaluated.
        assert_eq!(formula("true or 1 / 0 == 1", ""), Value::Bool(true));
        assert_eq!(formula("false and 1 / 0 == 1", ""), Value::Bool(false));
        // Mismatched types are unequal, not an error.
        assert_eq!(formula("1 == \"1\"", ""), Value::Bool(false));
        assert_eq!(formula("missing == null", ""), Value::Bool(true));
    }

    #[test]
    fn if_and_coalesce_are_lazy() {
        assert_eq!(formula("if(1 < 2, \"a\", 1 / 0)", ""), text("a"));
        assert_eq!(formula("coalesce(missing, other, 3, 1 / 0)", ""), num_(3.0));
        assert_eq!(formula("coalesce(missing)", ""), Value::Null);
        assert_eq!(
            formula("if(missing, 1, 2)", ""),
            Value::Error(ErrorCode::Null)
        );
        assert_eq!(formula("if(1, 1, 2)", ""), Value::Error(ErrorCode::Type));
    }

    #[test]
    fn numeric_functions() {
        assert_eq!(formula("abs(-3)", ""), num_(3.0));
        assert_eq!(formula("round(2.5)", ""), num_(3.0));
        assert_eq!(formula("round(1.23456, 2)", ""), num_(1.23));
        assert_eq!(formula("floor(2.9)", ""), num_(2.0));
        assert_eq!(formula("ceil(2.1)", ""), num_(3.0));
        assert_eq!(formula("min(3, 1, 2)", ""), num_(1.0));
        assert_eq!(formula("max(3, 1, 2)", ""), num_(3.0));
    }

    #[test]
    fn text_and_list_functions() {
        assert_eq!(formula("len(\"héllo\")", ""), num_(5.0));
        assert_eq!(formula("len(tags)", "tags: [a, b, c]\n"), num_(3.0));
        assert_eq!(formula("lower(\"AbC\")", ""), text("abc"));
        assert_eq!(formula("upper(\"AbC\")", ""), text("ABC"));
        assert_eq!(
            formula("concat(\"n=\", 3, \" \", true)", ""),
            text("n=3 true")
        );
        assert_eq!(
            formula("contains(\"hello\", \"ell\")", ""),
            Value::Bool(true)
        );
        assert_eq!(
            formula("contains(tags, \"b\")", "tags: [a, b]\n"),
            Value::Bool(true)
        );
        assert_eq!(
            formula("contains(tags, \"z\")", "tags: [a, b]\n"),
            Value::Bool(false)
        );
    }

    #[test]
    fn date_functions_use_the_supplied_today() {
        let extra = "due_date: 2026-01-10\n";
        assert_eq!(formula("days_between(due_date, today())", extra), num_(5.0));
        assert_eq!(
            formula("days_between(today(), due_date)", extra),
            num_(-5.0)
        );
        assert_eq!(
            formula("add_days(due_date, 3)", extra),
            Value::Date(d("2026-01-13"))
        );
        assert_eq!(formula("year(due_date)", extra), num_(2026.0));
        assert_eq!(formula("month(due_date)", extra), num_(1.0));
        assert_eq!(formula("due_date < today()", extra), Value::Bool(true));
        // RFC 3339 timestamps read as datetimes; date functions take their day.
        assert_eq!(formula("year(modified)", ""), num_(2026.0));
    }

    #[test]
    fn today_is_a_parameter_not_the_clock() {
        let fx = fixture(
            "today",
            &[("n/a.md", md("A", "note", "due_date: 2026-01-10\n"))],
        );
        let yaml =
            "note:\n  - name: late\n    kind: formula\n    expr: days_between(due_date, today())\n";
        assert_eq!(eval(&fx, yaml, "A", d("2026-01-15"))["late"], num_(5.0));
        assert_eq!(eval(&fx, yaml, "A", d("2026-02-01"))["late"], num_(22.0));
    }

    #[test]
    fn computed_properties_can_reference_each_other() {
        let fx = fixture("chain", &[("n/a.md", md("A", "note", "n: 4\n"))]);
        let yaml = "note:\n  - name: double\n    kind: formula\n    expr: n * 2\n  - name: plus_one\n    kind: formula\n    expr: double + 1\n";
        let out = eval(&fx, yaml, "A", jan15());
        assert_eq!(out["double"], num_(8.0));
        assert_eq!(out["plus_one"], num_(9.0));
    }

    // -- errors are values ---------------------------------------------------

    #[test]
    fn errors_are_values_and_propagate() {
        assert_eq!(formula("1 / 0", ""), Value::Error(ErrorCode::DivideByZero));
        assert_eq!(formula("1 % 0", ""), Value::Error(ErrorCode::DivideByZero));
        assert_eq!(formula("missing + 1", ""), Value::Error(ErrorCode::Null));
        assert_eq!(formula("\"a\" + 1", ""), Value::Error(ErrorCode::Type));
        assert_eq!(formula("missing < 1", ""), Value::Error(ErrorCode::Null));
        assert_eq!(formula("\"a\" < 1", ""), Value::Error(ErrorCode::Type));
        assert_eq!(
            formula("(1 / 0) + 5", ""),
            Value::Error(ErrorCode::DivideByZero)
        );
        assert_eq!(
            formula("round(1 / 0)", ""),
            Value::Error(ErrorCode::DivideByZero)
        );
        assert_eq!(
            formula("days_between(\"x\", today())", ""),
            Value::Error(ErrorCode::Type)
        );
        // A finite literal times itself can overflow: that is an Error value, not inf.
        let big = format!("1{}", "0".repeat(200));
        assert_eq!(
            formula(&format!("{big} * {big}"), ""),
            Value::Error(ErrorCode::Number)
        );
        // ...but a literal that is itself out of range is rejected on load.
        assert!(
            load_err(&one(&format!("1{}", "0".repeat(400)))).contains("too long")
                || load_err(&one(&format!("1{}", "0".repeat(320)))).contains("out of range")
        );
    }

    #[test]
    fn one_bad_property_does_not_spoil_the_others() {
        let fx = fixture("isolate", &[("n/a.md", md("A", "note", "n: 4\n"))]);
        let yaml = "note:\n  - name: bad\n    kind: formula\n    expr: n / 0\n  - name: good\n    kind: formula\n    expr: n + 1\n";
        let out = eval(&fx, yaml, "A", jan15());
        assert_eq!(out["bad"], Value::Error(ErrorCode::DivideByZero));
        assert_eq!(out["good"], num_(5.0));
    }

    // -- definition validation: errors at load, not at read -----------------------

    fn load_err(yaml: &str) -> String {
        parse_definitions(yaml).unwrap_err().to_string()
    }

    fn one(expr: &str) -> String {
        format!(
            "note:\n  - name: x\n    kind: formula\n    expr: '{}'\n",
            expr.replace('\'', "''")
        )
    }

    #[test]
    fn bad_definitions_are_rejected_on_load() {
        assert!(load_err(&one("nope(1)")).contains("unknown function `nope`"));
        assert!(load_err(&one("abs(1, 2)")).contains("takes 1 argument"));
        assert!(load_err(&one("if(true, 1)")).contains("takes 3 argument"));
        assert!(load_err(&one("1 +")).contains("unexpected end"));
        assert!(load_err(&one("(1 + 2")).contains("missing `)`"));
        assert!(load_err(&one("1 = 2")).contains("`==`"));
        assert!(load_err(&one("\"open")).contains("unterminated"));
        assert!(load_err(&one("1 2")).contains("trailing"));
        assert!(load_err(&one("")).contains("empty"));
        assert!(load_err(&one("1 ; 2")).contains("unexpected character"));
        assert!(load_err("note:\n  - name: x\n    kind: formula\n").contains("needs `expr`"));
        assert!(
            load_err("note:\n  - name: x\n    kind: formula\n    expr: '1'\n    fn: sum\n")
                .contains("only `expr`")
        );
        assert!(load_err("note:\n  - name: x\n    kind: wat\n").contains("kind must be"));
        assert!(
            load_err("note:\n  - name: if\n    kind: formula\n    expr: '1'\n")
                .contains("identifier")
        );
        assert!(load_err("note:\n  - name: x\n    kind: formula\n    expr: '1'\n  - name: x\n    kind: formula\n    expr: '2'\n").contains("twice"));
        assert!(load_err(
            "note:\n  - name: x\n    kind: formula\n    expr: '1'\n    surprise: 1\n"
        )
        .contains("surprise"));
    }

    #[test]
    fn rollup_field_combinations_are_validated() {
        let r = |rest: &str| {
            format!("project:\n  - name: x\n    kind: rollup\n    relation: parent_project\n    direction: in\n{rest}")
        };
        assert!(load_err(&r("    fn: sum\n")).contains("needs `property`"));
        assert!(load_err(&r("    fn: count\n    property: n\n")).contains("takes no `property`"));
        assert!(load_err(&r("    fn: percent\n")).contains("needs a `where`"));
        assert!(load_err(&r("    fn: median\n    property: n\n")).contains("unknown fn"));
        assert!(
            load_err(&r("    fn: sum\n    property: n\n    expr: '1'\n")).contains("no `expr`")
        );
        assert!(load_err("project:\n  - name: x\n    kind: rollup\n    relation: project-contains\n    fn: count\n").contains("derived inverse"));
        assert!(load_err("project:\n  - name: x\n    kind: rollup\n    relation: parent_project\n    direction: sideways\n    fn: count\n").contains("direction"));
        assert!(
            load_err("project:\n  - name: x\n    kind: rollup\n    fn: count\n")
                .contains("needs `relation`")
        );
    }

    // -- limits and termination ---------------------------------------------

    #[test]
    fn size_and_depth_limits_are_enforced() {
        assert!(load_err(&one(&"1".repeat(1025))).contains("too long"));
        assert!(load_err(&one(&"(".repeat(40))).contains("nested too deeply"));
        let deep = format!("{}1{}", "(".repeat(40), ")".repeat(40));
        assert!(load_err(&one(&deep)).contains("nested too deeply"));
        let many = vec!["1"; 200].join(" + "); // 399 AST nodes
        assert!(load_err(&one(&many)).contains("too large"));
        let calls = format!("{}1{}", "abs(".repeat(30), ")".repeat(30));
        assert!(load_err(&one(&calls)).contains("nested too deeply"));
        // Just inside the limits still loads.
        let ok = format!("{}1{}", "(".repeat(10), ")".repeat(10));
        assert!(parse_definitions(&one(&ok)).is_ok());
    }

    #[test]
    fn pathological_inputs_error_without_hanging_or_panicking() {
        let nasty = [
            "((((((((((((((((((((((((((((((((",
            "))))))))",
            "1 +++ 2",
            "not not not not not not not not not not not not not not not not not not 1",
            "if(if(if(if(if(if(if(if(if(if(if(if(if(if(if(if(if(1,1,1),1,1),1,1),1,1),1,1),1,1),1,1),1,1),1,1),1,1),1,1),1,1),1,1),1,1),1,1),1,1),1,1)",
            "\"\\",
            "\"\\x\"",
            ",,,,",
            "abs(,)",
            "abs(1,,2)",
            "and",
            "or or or",
            "0.0.0",
            "1.",
            ".5",
            "é",
            "\u{0}",
            "😀",
            "- - - - - - - - - - - - - - - - - - -1",
            "a b c d",
        ];
        for src in nasty {
            // Must be a clean Ok or Err; reaching the assert at all means no hang/panic.
            let _ = parse_definitions(&one(src));
        }
        // A wide-but-legal expression evaluates (and ends) rather than looping.
        assert_eq!(formula("abs(abs(abs(abs(abs(-1)))))", ""), num_(1.0));
    }

    // -- cycles ---------------------------------------------------------------

    #[test]
    fn cycles_are_rejected_on_load() {
        let direct = "note:\n  - name: a\n    kind: formula\n    expr: a + 1\n";
        assert!(
            load_err(direct).contains("cycle: note.a -> note.a"),
            "{}",
            load_err(direct)
        );

        let mutual = "note:\n  - name: a\n    kind: formula\n    expr: b\n  - name: b\n    kind: formula\n    expr: a\n";
        let e = load_err(mutual);
        assert!(
            e.contains("cycle") && e.contains("note.a") && e.contains("note.b"),
            "{e}"
        );

        // rollup -> related formula -> rollup (on the same type, via a name).
        let via_rollup = "\
project:
  - name: total
    kind: rollup
    relation: parent_project
    direction: in
    fn: sum
    property: weight
task:
  - name: weight
    kind: formula
    expr: total + 1
";
        // task.weight reads `total`, which is not a task property -> a plain field. No cycle.
        assert!(parse_definitions(via_rollup).is_ok());

        let real_cycle = "\
project:
  - name: total
    kind: rollup
    relation: parent_project
    direction: in
    fn: sum
    property: weight
  - name: weight
    kind: formula
    expr: total + 1
";
        let e = load_err(real_cycle);
        assert!(e.contains("cycle"), "{e}");
    }

    #[test]
    fn rollups_never_chain() {
        let chained = "\
project:
  - name: a
    kind: rollup
    relation: parent_project
    direction: in
    fn: count
task:
  - name: b
    kind: rollup
    relation: parent_project
    fn: sum
    property: a
";
        assert!(load_err(chained).contains("rollups never chain"));
    }

    // -- rollups: results equal a hand-computed oracle ---------------------------------

    const PROJECT_DEFS: &str = "\
project:
  - name: tasks
    kind: rollup
    relation: parent_project
    direction: in
    fn: count
  - name: done_tasks
    kind: rollup
    relation: parent_project
    direction: in
    where: 'status == \"done\"'
    fn: count
  - name: done_pct
    kind: rollup
    relation: parent_project
    direction: in
    where: 'status == \"done\"'
    fn: percent
  - name: est_total
    kind: rollup
    relation: parent_project
    direction: in
    property: estimated_pomodoros
    fn: sum
  - name: est_avg
    kind: rollup
    relation: parent_project
    direction: in
    property: estimated_pomodoros
    fn: avg
  - name: est_min
    kind: rollup
    relation: parent_project
    direction: in
    property: estimated_pomodoros
    fn: min
  - name: est_max
    kind: rollup
    relation: parent_project
    direction: in
    property: estimated_pomodoros
    fn: max
  - name: next_due
    kind: rollup
    relation: parent_project
    direction: in
    property: due_date
    fn: earliest
  - name: last_due
    kind: rollup
    relation: parent_project
    direction: in
    property: due_date
    fn: latest
  - name: all_done
    kind: rollup
    relation: parent_project
    direction: in
    property: is_done
    fn: all
  - name: any_done
    kind: rollup
    relation: parent_project
    direction: in
    property: is_done
    fn: any
  - name: statuses
    kind: rollup
    relation: parent_project
    direction: in
    property: status
    fn: unique
  - name: open_est
    kind: rollup
    relation: parent_project
    direction: in
    where: 'status != \"done\"'
    property: estimated_pomodoros
    fn: sum
task:
  - name: is_done
    kind: formula
    expr: 'status == \"done\"'
";

    fn project_fixture() -> Fx {
        fixture(
            "rollup",
            &[
                ("p/p1.md", md("P1", "project", "project_status: active\n")),
                ("t/t1.md", task("T1", "P1", "status: done\nestimated_pomodoros: 2\ndue_date: 2026-02-01\n")),
                ("t/t2.md", task("T2", "P1", "status: todo\nestimated_pomodoros: 3\ndue_date: 2026-01-20\n")),
                ("t/t3.md", task("T3", "P1", "status: done\n")),
                // Soft-deleted: must be excluded from every rollup.
                ("t/t4.md", task("T4", "P1", "status: done\nestimated_pomodoros: 100\ndue_date: 2025-01-01\ndeleted_at: 2026-01-16T00:00:00Z\n")),
                // Belongs to another project: never counted.
                ("p/p2.md", md("P2", "project", "")),
                ("t/t5.md", task("T5", "P2", "status: done\nestimated_pomodoros: 50\n")),
            ],
        )
    }

    #[test]
    fn rollups_match_a_hand_computed_oracle() {
        let fx = project_fixture();
        let out = eval(&fx, PROJECT_DEFS, "P1", jan15());
        // Live tasks of P1: T1 (done, 2, due 02-01), T2 (todo, 3, due 01-20), T3 (done, no est).
        assert_eq!(out["tasks"], num_(3.0));
        assert_eq!(out["done_tasks"], num_(2.0));
        match &out["done_pct"] {
            Value::Number(n) => assert!((n - 200.0 / 3.0).abs() < 1e-9, "{n}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(out["est_total"], num_(5.0)); // T3's missing estimate is skipped
        assert_eq!(out["est_avg"], num_(2.5));
        assert_eq!(out["est_min"], num_(2.0));
        assert_eq!(out["est_max"], num_(3.0));
        assert_eq!(out["next_due"], Value::Date(d("2026-01-20")));
        assert_eq!(out["last_due"], Value::Date(d("2026-02-01")));
        assert_eq!(out["all_done"], Value::Bool(false));
        assert_eq!(out["any_done"], Value::Bool(true));
        assert_eq!(
            out["statuses"],
            Value::List(vec![text("done"), text("todo")])
        );
        assert_eq!(out["open_est"], num_(3.0)); // where status != done -> only T2
    }

    #[test]
    fn rollup_over_the_outgoing_direction() {
        // A task's `parent_project` points out at its project.
        let fx = project_fixture();
        let defs = "task:\n  - name: proj_type\n    kind: rollup\n    relation: parent_project\n    direction: out\n    property: type\n    fn: unique\n  - name: n_projects\n    kind: rollup\n    relation: parent_project\n    direction: out\n    fn: count\n";
        let out = eval(&fx, defs, "T1", jan15());
        assert_eq!(out["proj_type"], Value::List(vec![text("project")]));
        assert_eq!(out["n_projects"], num_(1.0));
    }

    #[test]
    fn empty_rollups_follow_the_documented_rules() {
        let fx = project_fixture();
        // P2's only task is done; a project with no tasks at all:
        let fx2 = fixture("empty", &[("p/p3.md", md("P3", "project", ""))]);
        let out = eval(&fx2, PROJECT_DEFS, "P3", jan15());
        assert_eq!(out["tasks"], num_(0.0)); // count of nothing is 0
        assert_eq!(out["done_pct"], Value::Null); // no population -> null, not 0%
        for k in [
            "est_total",
            "est_avg",
            "est_min",
            "est_max",
            "next_due",
            "last_due",
            "all_done",
            "any_done",
            "statuses",
        ] {
            assert_eq!(out[k], Value::Null, "{k}");
        }
        let _ = fx;
    }

    #[test]
    fn rollup_reads_a_related_nodes_formula() {
        let fx = project_fixture();
        let out = eval(&fx, PROJECT_DEFS, "P1", jan15());
        // `all_done`/`any_done` aggregate the task formula `is_done`.
        assert_eq!(out["any_done"], Value::Bool(true));
        assert_eq!(out["all_done"], Value::Bool(false));
    }

    #[test]
    fn a_related_nodes_error_surfaces_in_the_rollup() {
        let fx = fixture(
            "relerr",
            &[
                ("p/p1.md", md("P1", "project", "")),
                ("t/t1.md", task("T1", "P1", "n: 2\n")),
                ("t/t2.md", task("T2", "P1", "n: abc\n")),
            ],
        );
        let defs = "project:\n  - name: total\n    kind: rollup\n    relation: parent_project\n    direction: in\n    fn: sum\n    property: n\n";
        assert_eq!(
            eval(&fx, defs, "P1", jan15())["total"],
            Value::Error(ErrorCode::Type)
        );
    }

    #[test]
    fn dangling_and_missing_relations_are_ignored() {
        let fx = fixture(
            "dangling",
            &[
                ("p/p1.md", md("P1", "project", "")),
                ("t/t1.md", task("T1", "GONE", "")),
                ("t/t2.md", task("T2", "P1", "")),
            ],
        );
        let defs = "project:\n  - name: n\n    kind: rollup\n    relation: parent_project\n    direction: in\n    fn: count\n";
        assert_eq!(eval(&fx, defs, "P1", jan15())["n"], num_(1.0));
        // And `evaluate` on an id that does not exist is a clean Err.
        let parsed = parse_definitions(defs).unwrap();
        assert!(evaluate(&fx.vault, &fx.cache, &parsed, "NOPE", jan15()).is_err());
    }

    // -- purity -----------------------------------------------------------------

    fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for e in fs::read_dir(dir).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    walk(&p, out);
                } else {
                    out.insert(p.clone(), fs::read(&p).unwrap());
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(dir, &mut out);
        out
    }

    #[test]
    fn evaluation_never_changes_files_or_the_cache() {
        let fx = project_fixture();
        let before_files = snapshot(fx.vault.root());
        let before_cache = fx.cache.list_nodes().unwrap();
        let _ = eval(&fx, PROJECT_DEFS, "P1", jan15());
        let _ = eval(&fx, PROJECT_DEFS, "P1", d("2030-01-01"));
        assert_eq!(snapshot(fx.vault.root()), before_files);
        let after_cache = fx.cache.list_nodes().unwrap();
        assert_eq!(before_cache.len(), after_cache.len());
        for (a, b) in before_cache.iter().zip(&after_cache) {
            assert_eq!((&a.id, &a.path, &a.status), (&b.id, &b.path, &b.status));
        }
    }

    // -- loading from disk ------------------------------------------------------

    #[test]
    fn missing_file_is_empty_and_present_file_loads() {
        let dir = TempDir::new("load");
        assert!(load(&dir.0).unwrap().is_empty());
        fs::create_dir_all(dir.0.join(".iris")).unwrap();
        fs::write(dir.0.join(".iris/computed.yaml"), one("1 + 1")).unwrap();
        let defs = load(&dir.0).unwrap();
        assert_eq!(defs.for_type("note").len(), 1);
        assert!(defs.for_type("task").is_empty());
        fs::write(dir.0.join(".iris/computed.yaml"), "not: [valid").unwrap();
        assert!(load(&dir.0).is_err());
    }

    #[test]
    fn number_display_is_rounded_but_json_is_not() {
        assert_eq!(Value::Number(100.0 / 3.0).display(), "33.3333");
        assert_eq!(Value::Number(2.5).display(), "2.5");
        assert_eq!(Value::Number(4.0).display(), "4");
        assert_eq!(Value::Number(0.00001).display(), "0");
        assert_eq!(
            Value::Number(1.0 / 3.0).to_json().to_string(),
            "0.3333333333333333"
        );
    }

    #[test]
    fn value_json_and_display_forms_are_stable() {
        use serde_json::json;
        let cases: Vec<(Value, serde_json::Value, &str)> = vec![
            (Value::Null, json!(null), "null"),
            (Value::Bool(true), json!(true), "true"),
            (Value::Number(3.0), json!(3), "3"),
            (Value::Number(2.5), json!(2.5), "2.5"),
            (Value::Text("hi".into()), json!("hi"), "hi"),
            (
                Value::Date(d("2026-01-15")),
                json!("2026-01-15"),
                "2026-01-15",
            ),
            (
                Value::List(vec![Value::Number(1.0), Value::Text("a".into())]),
                json!([1, "a"]),
                "[1, a]",
            ),
            (
                Value::Error(ErrorCode::DivideByZero),
                json!({"error": "divide_by_zero"}),
                "#error(divide_by_zero)",
            ),
        ];
        for (v, j, shown) in cases {
            assert_eq!(v.to_json(), j, "{v:?}");
            assert_eq!(v.display(), shown, "{v:?}");
        }
        assert_eq!(Value::Date(jan15()).kind(), "date");
        assert_eq!(Value::Error(ErrorCode::Type).kind(), "error");
    }

    #[test]
    fn computed_values_are_per_type_and_missing_types_are_empty() {
        let fx = project_fixture();
        let defs = parse_definitions(PROJECT_DEFS).unwrap();
        // A task has `is_done`; a project node has none of the task defs.
        let t1 = evaluate(&fx.vault, &fx.cache, &defs, "T1", jan15()).unwrap();
        assert_eq!(t1.keys().collect::<Vec<_>>(), vec!["is_done"]);
        assert_eq!(t1["is_done"], Value::Bool(true));
        let none = parse_definitions("").unwrap_or_default();
        assert!(evaluate(&fx.vault, &fx.cache, &none, "P1", jan15())
            .unwrap()
            .is_empty());
    }
}
