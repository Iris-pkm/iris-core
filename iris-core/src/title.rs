//! Display-title resolution (ADR-042); never changes canonical content.
use std::path::Path;

pub fn resolve_title(title: Option<&str>, body: &str, path: &Path) -> String {
    if let Some(title) = title.map(str::trim).filter(|s| !s.is_empty()) {
        return title.to_owned();
    }
    let mut fence: Option<(u8, usize)> = None;
    for line in body.lines() {
        let trimmed = line.trim_start_matches(' ');
        if line.len() - trimmed.len() > 3 {
            continue;
        }
        let line = trimmed;
        let bytes = line.as_bytes();
        if let Some((marker, count)) = fence {
            let run = bytes.iter().take_while(|&&b| b == marker).count();
            if run >= count && line[run..].trim().is_empty() {
                fence = None;
            }
            continue;
        }
        if let Some(&marker @ (b'`' | b'~')) = bytes.first() {
            let count = bytes.iter().take_while(|&&b| b == marker).count();
            if count >= 3 && (marker != b'`' || !line[count..].contains('`')) {
                fence = Some((marker, count));
                continue;
            }
        }
        if let Some(text) = line
            .strip_prefix('#')
            .filter(|s| s.is_empty() || s.starts_with([' ', '\t']))
        {
            let text = text.trim();
            let without_hashes = text.trim_end_matches('#');
            let text = if without_hashes.ends_with([' ', '\t']) || without_hashes.is_empty() {
                without_hashes.trim_end()
            } else {
                text
            };
            return text.to_owned();
        }
    }
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn precedence_and_markdown_edges() {
        for (title, body, expected) in [
            (Some(" Explicit "), "# Heading", "Explicit"),
            (Some(" \t"), "# Heading ###", "Heading"),
            (
                None,
                "```md\n# code\n```\n~~~\n# code\n~~~\n## H2\n# **Real**",
                "**Real**",
            ),
            (None, "## H2", "filename"),
            (None, "# C#", "C#"),
            (None, "    # indented code\n# Actual", "Actual"),
            (
                None,
                "````\n# code\n```\n# still code\n````\n# Actual",
                "Actual",
            ),
            (None, "```invalid`info\n# Actual", "Actual"),
        ] {
            assert_eq!(
                resolve_title(title, body, Path::new("notes/filename.md")),
                expected
            );
        }
    }
}
