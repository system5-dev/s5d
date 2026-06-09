//! Flatten: port of skills/s5d/scripts/flatten.sh (jq → Rust).
//!
//! Normalizes three analyze output shapes into markdown anomaly bullets.
//! Severity parse is case-insensitive — the bash original had a bug class
//! where mixed-case severities (e.g. "HIGH") were dropped; this impl
//! always downcases before ranking.

use serde_json::Value;

use super::Severity;

/// Flatten a skill's JSON output into markdown anomaly bullets.
///
/// - `json`: raw JSON string from an analyze call.
/// - `label`: section label (e.g. "ddd-refactor").
/// - `min`: floor severity; items below are excluded.
pub fn flatten(json: &str, label: &str, min: Severity) -> anyhow::Result<String> {
    let v: Value = serde_json::from_str(json)?;
    let items = normalize_items(&v);

    let kept: Vec<_> = items
        .into_iter()
        .filter(|i| i.sev >= min)
        .collect();

    // Sort descending by severity
    let mut kept = kept;
    kept.sort_by_key(|i| std::cmp::Reverse(i.sev));

    let mut out = String::new();
    let min_str = min.to_string();

    // Header
    out.push_str(&format!(
        "## {} — {} anomaly(ies) >= {}\n",
        label,
        kept.len(),
        min_str
    ));

    if kept.is_empty() {
        out.push_str(&format!("✓ none at/above {}", min_str));
    } else {
        for item in &kept {
            // "- **[SEV]** label" + optional path + " — detail" + optional fix
            out.push_str(&format!("- **[{}]** {}", item.sev.to_string().to_uppercase(), item.label));
            if !item.path.is_empty() && item.path != "(repo)" {
                out.push_str(&format!(" (`{}`)", item.path));
            }
            let detail = item.detail.replace('\n', " ");
            out.push_str(&format!(" — {}", detail));
            if !item.fix.is_empty() {
                let fix = item.fix.replace('\n', " ");
                out.push_str(&format!("  _fix:_ {}", fix));
            }
            out.push('\n');
        }
        // Remove trailing newline to match jq output style
        if out.ends_with('\n') {
            out.pop();
        }
    }

    Ok(out)
}

// ── internal ──────────────────────────────────────────────────────────────────

struct Item {
    sev: Severity,
    label: String,
    path: String,
    detail: String,
    fix: String,
}

fn normalize_items(v: &Value) -> Vec<Item> {
    // Shape 1: findings[]
    if let Some(arr) = v.get("findings").and_then(|f| f.as_array()) {
        return arr
            .iter()
            .map(|f| Item {
                sev: Severity::from_str_lossy(
                    f.get("severity").and_then(|s| s.as_str()).unwrap_or("info"),
                ),
                label: f
                    .get("check")
                    .and_then(|s| s.as_str())
                    .unwrap_or("?")
                    .to_string(),
                path: f
                    .get("path")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
                detail: f
                    .get("detail")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
                fix: f
                    .get("fix")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
            })
            .collect();
    }

    // Shape 2: violations[]
    if let Some(arr) = v.get("violations").and_then(|f| f.as_array()) {
        return arr
            .iter()
            .map(|f| Item {
                sev: Severity::from_str_lossy(
                    f.get("severity").and_then(|s| s.as_str()).unwrap_or("info"),
                ),
                label: f
                    .get("kind")
                    .and_then(|s| s.as_str())
                    .unwrap_or("?")
                    .to_string(),
                path: f
                    .get("path")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
                detail: f
                    .get("detail")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
                fix: String::new(),
            })
            .collect();
    }

    // Shape 3: SARIF runs[].results[]
    if let Some(runs) = v.get("runs").and_then(|r| r.as_array()) {
        let mut items = Vec::new();
        for run in runs {
            if let Some(results) = run.get("results").and_then(|r| r.as_array()) {
                for result in results {
                    let level = result
                        .get("level")
                        .and_then(|l| l.as_str())
                        .unwrap_or("warning");
                    let sev = sarif_level_to_severity(level);
                    let label = result
                        .get("ruleId")
                        .and_then(|s| s.as_str())
                        .unwrap_or("?")
                        .to_string();
                    let path = result
                        .pointer("/locations/0/physicalLocation/artifactLocation/uri")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    let detail = result
                        .pointer("/message/text")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    items.push(Item { sev, label, path, detail, fix: String::new() });
                }
            }
        }
        return items;
    }

    Vec::new()
}

fn sarif_level_to_severity(level: &str) -> Severity {
    match level {
        "error" => Severity::High,
        "warning" => Severity::Medium,
        "note" => Severity::Low,
        "none" => Severity::Info,
        _ => Severity::Medium,
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_case_severities_kept() {
        let json = r#"{"findings":[
            {"check":"a","severity":"HIGH","path":"x.ts","detail":"d1","fix":"f1"},
            {"check":"b","severity":"medium","path":"y.ts","detail":"d2","fix":"f2"},
            {"check":"c","severity":"low","path":"z.ts","detail":"d3","fix":"f3"}
        ]}"#;

        let out = flatten(json, "test", Severity::Medium).unwrap();
        // header says 2
        assert!(out.contains("2 anomaly(ies) >= medium"), "expected 2 kept: {}", out);
        // 2 bullet lines
        let bullets = out.lines().filter(|l| l.starts_with("- **[")).count();
        assert_eq!(bullets, 2, "expected 2 bullet lines: {}", out);
    }

    #[test]
    fn floor_high_leaves_one() {
        let json = r#"{"findings":[
            {"check":"a","severity":"HIGH","path":"x.ts","detail":"d1","fix":"f1"},
            {"check":"b","severity":"medium","path":"y.ts","detail":"d2","fix":"f2"},
            {"check":"c","severity":"low","path":"z.ts","detail":"d3","fix":"f3"}
        ]}"#;

        let out = flatten(json, "test", Severity::High).unwrap();
        assert!(out.contains("1 anomaly(ies) >= high"), "expected 1: {}", out);
        let bullets = out.lines().filter(|l| l.starts_with("- **[")).count();
        assert_eq!(bullets, 1);
    }

    #[test]
    fn empty_findings_shows_check_mark() {
        let json = r#"{"findings":[]}"#;
        let out = flatten(json, "test", Severity::Medium).unwrap();
        assert!(out.contains("✓ none at/above medium"), "expected none: {}", out);
    }

    #[test]
    fn sarif_error_maps_to_high() {
        let json = r#"{"runs":[{"results":[
            {"ruleId":"rule1","level":"error","locations":[{"physicalLocation":{"artifactLocation":{"uri":"src/x.ts"}}}],"message":{"text":"bad"}}
        ]}]}"#;

        let out = flatten(json, "sarif-test", Severity::High).unwrap();
        assert!(out.contains("[HIGH]"), "SARIF error must map to HIGH: {}", out);
    }

    #[test]
    fn header_format_correct() {
        let json = r#"{"findings":[
            {"check":"a","severity":"high","path":"x.ts","detail":"detail text","fix":"fix text"}
        ]}"#;
        let out = flatten(json, "my-label", Severity::Medium).unwrap();
        assert!(out.starts_with("## my-label — 1 anomaly(ies) >= medium"));
    }
}
