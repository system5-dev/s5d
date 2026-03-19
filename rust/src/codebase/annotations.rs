use regex::Regex;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AnnotationHit {
    pub file_path: String,
    pub line: u32,
    pub artifact_kind: String,
    pub artifact_id: String,
}

/// Scan a single source file for @s5d: annotations in comments.
/// Supports: // @s5d:kind id, # @s5d:kind id, /* @s5d:kind id */
pub fn scan_file(path: &Path) -> Vec<AnnotationHit> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let file_path = path.to_string_lossy().into_owned();
    let re = Regex::new(r"@s5d:(\w+)\s+(\S+)").unwrap();

    let mut hits = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let is_comment = trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
            || trimmed.starts_with("--");
        if !is_comment {
            continue;
        }
        for cap in re.captures_iter(line) {
            let kind = cap[1].to_string();
            let id = cap[2].to_string();
            if matches!(
                kind.as_str(),
                "domain" | "capability" | "entity" | "component" | "container" | "usecase"
            ) {
                hits.push(AnnotationHit {
                    file_path: file_path.clone(),
                    line: i as u32 + 1,
                    artifact_kind: kind,
                    artifact_id: id,
                });
            }
        }
    }
    hits
}

/// Scan all source files under a directory for @s5d: annotations.
pub fn scan_directory(root: &Path) -> Vec<AnnotationHit> {
    let mut all = Vec::new();
    scan_recursive(root, &mut all);
    all
}

fn scan_recursive(dir: &Path, out: &mut Vec<AnnotationHit>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(name, "target" | ".git" | "node_modules" | ".s5d" | "vendor") {
                continue;
            }
            scan_recursive(&path, out);
        } else if super::parser::detect_lang(&path).is_some() {
            let hits = scan_file(&path);
            out.extend(hits);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_scan_rust_annotations() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.rs");
        let mut f = std::fs::File::create(&file).unwrap();
        writeln!(f, "// @s5d:domain billing").unwrap();
        writeln!(f, "// @s5d:capability AuthorizePayment").unwrap();
        writeln!(f, "pub fn authorize_payment() {{}}").unwrap();
        writeln!(f, "// @s5d:entity Invoice").unwrap();
        drop(f);

        let hits = scan_file(&file);
        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].artifact_kind, "domain");
        assert_eq!(hits[0].artifact_id, "billing");
        assert_eq!(hits[1].artifact_kind, "capability");
        assert_eq!(hits[1].artifact_id, "AuthorizePayment");
        assert_eq!(hits[2].artifact_kind, "entity");
        assert_eq!(hits[2].artifact_id, "Invoice");
    }

    #[test]
    fn test_scan_python_annotations() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.py");
        let mut f = std::fs::File::create(&file).unwrap();
        writeln!(f, "# @s5d:domain payments").unwrap();
        writeln!(f, "# @s5d:capability ProcessRefund").unwrap();
        writeln!(f, "def process_refund(): pass").unwrap();
        drop(f);

        let hits = scan_file(&file);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].artifact_kind, "domain");
        assert_eq!(hits[0].artifact_id, "payments");
    }

    #[test]
    fn test_ignores_non_comment_annotations() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.rs");
        let mut f = std::fs::File::create(&file).unwrap();
        writeln!(f, r#"let s = "@s5d:domain fake";"#).unwrap();
        writeln!(f, "// @s5d:domain real").unwrap();
        drop(f);

        let hits = scan_file(&file);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].artifact_id, "real");
    }
}
