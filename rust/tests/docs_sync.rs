use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn read_repo_file(rel: &str) -> String {
    fs::read_to_string(repo_root().join(rel)).unwrap()
}

#[test]
fn skill_references_all_reference_docs() {
    let skill = read_repo_file("skills/s5d/SKILL.md");
    for doc in ["metamodel.md", "session-protocol.md"] {
        assert!(skill.contains(doc), "SKILL.md must reference: {}", doc);
    }
}

#[test]
fn reference_docs_exist() {
    for doc in ["skills/s5d/metamodel.md", "skills/s5d/session-protocol.md"] {
        assert!(
            repo_root().join(doc).exists(),
            "reference doc missing: {}",
            doc
        );
    }
}

#[test]
fn metamodel_has_core_invariants() {
    let meta = read_repo_file("skills/s5d/metamodel.md");
    for term in ["Domain", "Capability", "Entity", "Component", "Edge"] {
        assert!(
            meta.contains(term),
            "metamodel.md must define artifact: {}",
            term
        );
    }
}

#[test]
fn session_protocol_has_core_concepts() {
    let session = read_repo_file("skills/s5d/session-protocol.md");
    for term in ["WAL", "spec://", "REVIEW", "Effectiveness Metrics"] {
        assert!(
            session.contains(term),
            "session-protocol.md must contain: {}",
            term
        );
    }
}

#[test]
fn session_protocol_documents_effectiveness_measurement() {
    let session = read_repo_file("skills/s5d/session-protocol.md");
    for term in [
        "Judgment",
        "Efficiency",
        "Intent preservation",
        "A/B Protocol",
        "Replay Protocol",
        "Code-only",
        "Spec-anchored",
    ] {
        assert!(
            session.contains(term),
            "session-protocol.md must document effectiveness measurement term: {}",
            term
        );
    }
}

#[test]
fn docs_state_existing_codebase_scope() {
    let readme = read_repo_file("README.md");
    assert!(readme.contains("existing repository") || readme.contains("repository"));
}

#[test]
fn workflow_shell_commands_are_documented() {
    let readme = read_repo_file("README.md");
    let skill = read_repo_file("skills/s5d/SKILL.md");

    for term in [
        "s5d phase start",
        "s5d phase run",
        "s5d execute loop",
        "--verdict",
        "--measurement-window",
        "--telemetry",
        "--mode",
        ".s5d/tasks/",
        "ralph-init",
        "ralph-bugfix",
        ".s5d/runs/",
    ] {
        assert!(
            readme.contains(term) || skill.contains(term),
            "workflow-shell docs must mention: {}",
            term
        );
    }
}

#[test]
fn skill_defines_cli_conductor_contract() {
    let skill = read_repo_file("skills/s5d/SKILL.md");

    for term in [
        "human-facing conductor",
        "not a second state machine",
        ".s5d/config.yaml",
        "approved: true",
        "must not",
        "call Claude/Codex/Gemini directly",
        "Engine completion is only evidence",
        "s5d add-hypothesis",
    ] {
        assert!(
            skill.contains(term),
            "S5D skill must preserve CLI-conductor contract term: {}",
            term
        );
    }
}
