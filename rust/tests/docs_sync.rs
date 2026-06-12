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
    for doc in [
        "metamodel.md",
        "session-protocol.md",
        "references/shape-layer.md",
        "references/adversarial-review.md",
    ] {
        assert!(skill.contains(doc), "SKILL.md must reference: {}", doc);
    }
}

#[test]
fn reference_docs_exist() {
    for doc in [
        "skills/s5d/metamodel.md",
        "skills/s5d/session-protocol.md",
        "skills/s5d/references/shape-layer.md",
        "skills/s5d/references/adversarial-review.md",
    ] {
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
fn agent_run_commands_are_documented() {
    let readme = read_repo_file("README.md");
    let skill = read_repo_file("skills/s5d/SKILL.md");

    for term in [
        "s5d run start",
        "s5d run exec",
        "s5d run task",
        "--verdict",
        "--measurement-window",
        "--telemetry",
        "--mode",
        ".s5d/tasks/",
        "ralph-init",
        "ralph-bugfix",
        ".s5d/runs/",
        "s5d run harness start",
        "s5d run harness status",
        "s5d run harness exec",
        ".s5d/harness/",
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
        "harness journal state",
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

#[test]
fn flow_vocabulary_is_canonical_and_consistent() {
    // One operational flow string, byte-identical across every doc that states it.
    // Guards the drift that prompted this test: metamodel.md once cited a
    // "Bootstrap → Frame → … → Build" flow as "defined in SKILL.md" while the
    // loaded skill used a different lifecycle. Four vocabularies, one lifecycle.
    const CANONICAL_FLOW: &str =
        "Route → Shape → Discover → Target → Decide → Spec → Run → Verify → Ship → Learn";

    for doc in ["skills/s5d/SKILL.md", "S5D.md", "skills/s5d/metamodel.md"] {
        let text = read_repo_file(doc);
        assert!(
            text.contains(CANONICAL_FLOW),
            "{doc} must state the canonical flow `{CANONICAL_FLOW}`"
        );
    }

    // Retired stage vocabularies must not reappear in any loaded doc.
    for doc in ["skills/s5d/SKILL.md", "S5D.md", "skills/s5d/metamodel.md"] {
        let text = read_repo_file(doc);
        for banned in ["Bootstrap → Frame", "→ Build →"] {
            assert!(
                !text.contains(banned),
                "{doc} still contains retired flow vocabulary `{banned}`"
            );
        }
    }
}

#[test]
fn waiver_semantics_are_consistent_across_runtime_docs() {
    // The High-tier "no waivers" vs Target/Decide auto-waiver resolution must
    // appear in BOTH runtime entry docs (Claude/Codex SKILL.md and the Gemini
    // context S5D.md). Otherwise an agent on one runtime hits the contradiction
    // a behavioral probe surfaced (auto-waiver Target+Decide vs "No waivers
    // allowed") while the other runtime does not.
    for doc in ["skills/s5d/SKILL.md", "S5D.md"] {
        let text = read_repo_file(doc);
        assert!(
            text.contains("assurance gates"),
            "{doc} must carry the High-tier no-waivers vs auto-waiver resolution"
        );
    }
}
