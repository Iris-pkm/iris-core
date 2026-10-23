use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct Vault(PathBuf);

impl Vault {
    fn new(yaml: Option<&str>) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "iris-computed-check-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(path.join(".iris")).unwrap();
        if let Some(yaml) = yaml {
            fs::write(path.join(".iris/computed.yaml"), yaml).unwrap();
        }
        Self(path)
    }

    fn check(&self, json: bool) -> Output {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_iris"));
        cmd.args(["-C", self.0.to_str().unwrap(), "computed", "check"]);
        if json {
            cmd.arg("--json");
        }
        cmd.output().unwrap()
    }
}

impl Drop for Vault {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn valid_definitions_list_name_and_kind() {
    let vault = Vault::new(Some(
        "note:\n  - name: score\n    kind: formula\n    expr: '1 + 2'\n",
    ));
    let text = vault.check(false);
    assert!(text.status.success());
    assert_eq!(
        String::from_utf8(text.stdout).unwrap(),
        "note.score: formula\n"
    );
    let json = vault.check(true);
    assert!(json.status.success());
    let values: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(
        values,
        serde_json::json!([{ "type": "note", "name": "score", "kind": "formula" }])
    );
}

#[test]
fn missing_definitions_file_is_valid_and_empty() {
    let vault = Vault::new(None);
    let output = vault.check(true);
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "[]");
}

#[test]
fn cycle_fails_with_property_path() {
    let vault = Vault::new(Some(
        "note:\n  - name: a\n    kind: formula\n    expr: a + 1\n",
    ));
    let output = vault.check(false);
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("note.a -> note.a"));
}

#[test]
fn unknown_field_fails_with_location() {
    let vault = Vault::new(Some(
        "note:\n  - name: score\n    kind: formula\n    expr: '1'\n    surprise: 2\n",
    ));
    let output = vault.check(false);
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(
        error.contains("surprise") && error.contains("line"),
        "{error}"
    );
}
