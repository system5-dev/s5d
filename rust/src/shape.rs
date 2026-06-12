//! Shape-layer runtime (decision.s5d.bmad-native-runtime): intake kernel
//! ingestion, routing, adversarial-review scaffolding, and story planning.
//! Thin deterministic commands over existing surfaces — the router classifies,
//! evidence dirs hold review reports, workflow.phases hold stories. The
//! runtime validates structure; content stays with the agent.

use crate::models::*;
use crate::project::S5dProject;
use std::path::PathBuf;

/// Outcome of shaping a kernel: routing via the existing classifier plus
/// structural readiness findings.
pub struct ShapeOutcome {
    pub route: crate::RouteResult,
    pub readiness_errors: Vec<String>,
}

/// Classify a kernel through the deterministic router and check readiness.
/// The routing text concatenates the kernel's prose fields so the classifier
/// sees the same signals an agent would.
pub fn shape_kernel(kernel: &IntentKernel) -> ShapeOutcome {
    let mut description = kernel.why.clone();
    for part in kernel
        .capabilities
        .iter()
        .chain(kernel.constraints.iter())
        .chain(kernel.non_goals.iter())
    {
        description.push_str(". ");
        description.push_str(part);
    }
    if !kernel.success_signal.is_empty() {
        description.push_str(". ");
        description.push_str(&kernel.success_signal);
    }

    ShapeOutcome {
        route: crate::route(&description),
        readiness_errors: crate::validate::intent_kernel_readiness_errors(kernel),
    }
}

/// Parse a kernel from YAML (file content or inline MCP payload).
pub fn parse_kernel(yaml: &str) -> anyhow::Result<IntentKernel> {
    let kernel: IntentKernel = serde_yaml::from_str(yaml)
        .map_err(|e| anyhow::anyhow!("kernel does not parse as IntentKernel YAML: {}", e))?;
    Ok(kernel)
}

/// Story-shaped phase input for `s5d plan stories` — maps 1:1 onto
/// WorkflowPhase with implementer/reviewer roles defaulted.
#[derive(Debug, serde::Deserialize)]
pub struct StoryInput {
    pub id: String,
    pub title: String,
    pub scope: String,
    #[serde(default)]
    pub acceptance: Vec<String>,
    #[serde(default)]
    pub rollback: Vec<String>,
}

/// Append story-shaped phases to a spec's workflow. Fails when the spec has
/// no workflow (every scaffold ships one) or when a story id collides with an
/// existing phase id. Returns the appended phase ids.
pub fn apply_stories(spec: &mut Spec, stories: Vec<StoryInput>) -> anyhow::Result<Vec<String>> {
    if stories.is_empty() {
        anyhow::bail!("no stories provided");
    }
    let workflow = spec
        .workflow
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("spec has no workflow section to hold story phases"))?;

    let existing: std::collections::HashSet<&str> =
        workflow.phases.iter().map(|p| p.id.as_str()).collect();
    for story in &stories {
        if story.id.trim().is_empty()
            || story.title.trim().is_empty()
            || story.scope.trim().is_empty()
        {
            anyhow::bail!(
                "story '{}' must have non-empty id, title, and scope",
                story.id
            );
        }
        // Phase ids become filesystem path components downstream (run task
        // artifacts under .s5d/tasks/) — reject unsafe ids at planning time.
        crate::sanitize_id(&story.id)
            .map_err(|e| anyhow::anyhow!("story id '{}' is unsafe: {}", story.id, e))?;
        // A story without acceptance criteria is not a story — the validator
        // enforces non-empty phase acceptance, so fail with the real reason.
        if story.acceptance.is_empty() {
            anyhow::bail!(
                "story '{}' must declare at least one acceptance criterion",
                story.id
            );
        }
        if existing.contains(story.id.as_str()) {
            anyhow::bail!("story id collides with existing phase: {}", story.id);
        }
    }

    let mut added = Vec::new();
    for story in stories {
        added.push(story.id.clone());
        let rollback = if story.rollback.is_empty() {
            vec!["Revert this story's changes if acceptance fails".into()]
        } else {
            story.rollback
        };
        workflow.phases.push(WorkflowPhase {
            id: story.id,
            title: story.title,
            scope: story.scope,
            roles: vec!["implementer".into(), "reviewer".into()],
            acceptance: story.acceptance,
            rollback,
        });
    }
    Ok(added)
}

/// Write the 3-layer adversarial review scaffold into the spec's evidence
/// directory (`.s5d/evidence/{spec_id}/adversarial-review-N.md`) and return
/// the path plus the tier-correct binding instruction.
pub fn scaffold_adversarial_review(
    project: &S5dProject,
    spec: &Spec,
) -> anyhow::Result<(PathBuf, String)> {
    // spec.id comes from FILE CONTENT, not a sanitized CLI argument — a
    // crafted `id: ../../escape` must not become a path segment.
    crate::sanitize_id(&spec.id)?;
    let evidence_dir = project.s5d_dir().join("evidence").join(&spec.id);
    std::fs::create_dir_all(&evidence_dir)?;

    let report = format!(
        "# Adversarial review — {spec_id}\n\n\
         Layered review per skills/s5d/references/adversarial-review.md. This file is\n\
         a companion report; it becomes S5D evidence only when bound (see Binding).\n\n\
         ## Layer 1 — Blind Diff Hunter (inputs: diff only)\n\n\
         <!-- suspicious changes, hidden coupling, unsafe defaults, missing tests -->\n\n\
         ## Layer 2 — Edge Case Hunter (inputs: diff + relevant project files)\n\n\
         <!-- boundaries, nulls, permissions, concurrency, migrations, failure states, rollback paths -->\n\n\
         ## Layer 3 — Acceptance Auditor (inputs: diff + spec + intent_kernel + gate results)\n\n\
         <!-- does implemented behavior satisfy the accepted intent without quiet scope change -->\n\n\
         ## Findings\n\n\
         ```text\n\
         Severity: blocker|high|medium|low\n\
         Title: <short issue>\n\
         Evidence: <file/path/command/spec reference>\n\
         Violated constraint: <acceptance/gate/architecture/security invariant>\n\
         Impact: <what breaks or becomes risky>\n\
         Disposition: fix|required-defer|accepted-risk|not-a-bug\n\
         ```\n\n\
         ## Triage\n\n\
         - [ ] every finding has a disposition (untriaged findings = review not closed)\n\
         - [ ] blocker/high findings fixed or explicitly accepted by the human owner\n\
         - [ ] skipped layers recorded as limitations\n",
        spec_id = spec.id
    );

    // create_new closes the exists→write race: a concurrent scaffold that
    // wins the same number makes this attempt retry the next one.
    let mut n = 1;
    let dest = loop {
        let candidate = evidence_dir.join(format!("adversarial-review-{}.md", n));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                use std::io::Write;
                file.write_all(report.as_bytes())?;
                break candidate;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                n += 1;
            }
            Err(e) => return Err(e.into()),
        }
    };

    let binding = match spec.tier {
        Tier::Decision | Tier::High => format!(
            "bind via review evidence: s5d decision add-evidence <spec> --hypothesis-id <id> --evidence-type gate:review --verdict pass --content \"<summary + dispositions>\" (report: {})",
            dest.display()
        ),
        _ => format!(
            "feature tier: the report lives in the evidence dir and gate:review (if declared) consumes hypothesis evidence; record the review outcome in the spec's record via run acceptance. Report: {}",
            dest.display()
        ),
    };

    Ok((dest, binding))
}
