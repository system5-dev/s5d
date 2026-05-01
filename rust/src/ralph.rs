use crate::{Spec, Workflow, WorkflowPhase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RalphPreset {
    Generic,
    Init,
    Bugfix,
}

impl RalphPreset {
    pub fn parse(raw: &str) -> anyhow::Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "generic" => Ok(Self::Generic),
            "init" => Ok(Self::Init),
            "bugfix" => Ok(Self::Bugfix),
            other => anyhow::bail!(
                "invalid Ralph preset '{}' (expected: generic, init, bugfix)",
                other
            ),
        }
    }

    pub fn resolve(raw: Option<&str>, phase_id: &str) -> anyhow::Result<Self> {
        match raw.map(str::trim).filter(|value| !value.is_empty()) {
            Some(raw) => Self::parse(raw),
            None => Ok(Self::infer_for_phase(phase_id)),
        }
    }

    pub fn infer_for_phase(phase_id: &str) -> Self {
        match phase_id {
            "prototype" => Self::Init,
            _ => Self::Generic,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::Init => "init",
            Self::Bugfix => "bugfix",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Generic => "ralph-generic",
            Self::Init => "ralph-init",
            Self::Bugfix => "ralph-bugfix",
        }
    }
}

pub fn build_ralph_task_package(
    spec: &Spec,
    workflow: &Workflow,
    phase: &WorkflowPhase,
    preset: RalphPreset,
) -> anyhow::Result<String> {
    let mut out = String::new();
    out.push_str("RALPH TASK PACKAGE\n");
    out.push_str(&format!("Spec: {}\n", spec.id));
    out.push_str(&format!("Phase: {} ({})\n", phase.title, phase.id));
    out.push_str("Engine: ralph\n");
    out.push_str(&format!("Preset: {}\n", preset.label()));
    if let Some(ref mode) = workflow.mode {
        out.push_str(&format!("Workflow mode: {}\n", mode));
    }
    if let Some(ref target) = workflow.target_architecture {
        out.push_str(&format!("\nTarget Architecture:\n{}\n", target.summary));
        if !target.invariants.is_empty() {
            push_bullets(&mut out, "Invariants", &target.invariants);
        }
    }
    if let Some(ref strategy) = workflow.delivery_strategy {
        out.push_str(&format!("\nDelivery Strategy:\n{}\n", strategy.summary));
        if let Some(ref rationale) = strategy.rationale {
            out.push_str(&format!("Rationale: {}\n", rationale));
        }
    }

    if let Some(ref outline) = workflow.structure_outline {
        out.push_str(&format!("\nStructure Outline:\n{}\n", outline.summary));
        if !outline.signatures.is_empty() {
            push_bullets(&mut out, "Signatures", &outline.signatures);
        }
        if !outline.types.is_empty() {
            push_bullets(&mut out, "Types", &outline.types);
        }
    }

    out.push_str(&format!("\nScope:\n{}\n", phase.scope));
    if !phase.roles.is_empty() {
        let roles = phase
            .roles
            .iter()
            .map(|role| {
                let binding = workflow
                    .role_map
                    .get(role)
                    .map(String::as_str)
                    .unwrap_or("unassigned");
                format!("{} -> {}", role, binding)
            })
            .collect::<Vec<_>>();
        push_bullets(&mut out, "Roles", &roles);
    }
    if !phase.acceptance.is_empty() {
        push_bullets(&mut out, "Acceptance", &phase.acceptance);
    }
    if !phase.rollback.is_empty() {
        push_bullets(&mut out, "Rollback", &phase.rollback);
    }
    if let Some(ref exec_mode) = workflow.execution_mode {
        if !exec_mode.stop_conditions.is_empty() {
            push_bullets(&mut out, "Stop Conditions", &exec_mode.stop_conditions);
        }
        if let Some(max_iterations) = exec_mode.max_iterations {
            out.push_str(&format!("Iteration cap: {}\n", max_iterations));
        }
    }

    push_bullets(&mut out, "Required Inputs", preset_required_inputs(preset));
    push_bullets(&mut out, "Execution Runbook", preset_runbook(preset));
    push_bullets(&mut out, "Done Definition", preset_done_definition(preset));

    let mut evidence_package = vec![
        "Diff scoped to the active phase only".to_string(),
        "Notes that explain non-obvious trade-offs and reviewer handoff".to_string(),
        "Evidence that acceptance criteria are satisfied".to_string(),
    ];
    if !spec.gates.is_empty() {
        evidence_package.push("Gate evidence for declared checks".into());
        for gate in &spec.gates {
            evidence_package.push(format!("gate: {}", gate.kind));
        }
    }
    if matches!(phase.id.as_str(), "measure") || workflow.mode.as_deref() == Some("operate") {
        evidence_package.push("Telemetry wiring proof for the measurement surface".into());
    }
    evidence_package.extend(
        preset_evidence_requirements(preset)
            .iter()
            .map(|item| item.to_string()),
    );
    push_bullets(&mut out, "Required Evidence Package", &evidence_package);

    let mut escalations = vec![
        "Target architecture must change".to_string(),
        "Phase scope must expand".to_string(),
        "New roles or dependencies are required".to_string(),
        "A stop condition fires before acceptance".to_string(),
        "Evidence package cannot support a human phase acceptance".to_string(),
    ];
    escalations.extend(
        preset_escalations(preset)
            .iter()
            .map(|item| item.to_string()),
    );
    push_bullets(&mut out, "Escalate Immediately If", &escalations);

    Ok(out)
}

const HORIZONTAL_PATTERNS: &[&str] = &[
    "all database",
    "all api",
    "all frontend",
    "all ui",
    "entire model",
    "entire schema",
    "all migrations",
    "all endpoints",
    "all components",
    "whole data layer",
    "whole service layer",
];

pub fn check_vertical_slicing(phase: &WorkflowPhase) -> Option<String> {
    let scope_lower = phase.scope.to_ascii_lowercase();
    for pattern in HORIZONTAL_PATTERNS {
        if scope_lower.contains(pattern) {
            return Some(format!(
                "phase '{}' scope looks horizontal (matches '{}') — \
                 consider vertical slices with end-to-end verification points",
                phase.id, pattern
            ));
        }
    }
    None
}

fn push_bullets(out: &mut String, title: &str, bullets: &[impl AsRef<str>]) {
    if bullets.is_empty() {
        return;
    }
    out.push('\n');
    out.push_str(title);
    out.push_str(":\n");
    for bullet in bullets {
        out.push_str("- ");
        out.push_str(bullet.as_ref());
        out.push('\n');
    }
}

fn preset_required_inputs(preset: RalphPreset) -> &'static [&'static str] {
    match preset {
        RalphPreset::Generic => &["Approved spec, active phase, and the current repository state"],
        RalphPreset::Init => &[
            "Repository root with docs, code, and tests available",
            "Documented setup path or CI hints for the dev environment",
        ],
        RalphPreset::Bugfix => &[
            "Clear bug statement: expected vs actual behavior",
            "Reproduction path, failing scenario, or issue text",
            "Scope small enough to stay inside the active phase boundary",
        ],
    }
}

fn preset_runbook(preset: RalphPreset) -> &'static [&'static str] {
    match preset {
        RalphPreset::Generic => &[
            "Work only inside the approved phase scope",
            "Preserve target architecture and stop on architecture drift",
            "Produce an evidence package instead of claiming the work is done",
        ],
        RalphPreset::Init => &[
            "Scan repository layout, entrypoints, and documented boundaries",
            "Read user-facing docs and representative tests before inferring from code only",
            "Prepare the documented dev environment without inventing a new bootstrap flow",
            "Run the current test suite and record blockers without fixing unrelated failures",
            "Produce a short warm-up report with repo map, test commands, behavior coverage, and risks",
        ],
        RalphPreset::Bugfix => &[
            "Understand the affected area and the existing behavioral contract",
            "Write or update a regression test first",
            "Run the targeted test and confirm it fails for the expected reason",
            "Apply the smallest reasonable fix that stays inside the current scope",
            "Re-run the targeted test until green, then run the full suite",
            "Summarize root cause, fix, changed tests, and residual risk",
        ],
    }
}

fn preset_done_definition(preset: RalphPreset) -> &'static [&'static str] {
    match preset {
        RalphPreset::Generic => {
            &["Acceptance criteria are met and the reviewer can inspect the evidence package"]
        }
        RalphPreset::Init => &[
            "Repository context is warmed: docs, tests, and entrypoints mapped",
            "Documented environment path is proven or blocked with explicit evidence",
            "A concise warm-up report exists and is ready for the owner/reviewer",
        ],
        RalphPreset::Bugfix => &[
            "A regression test reproduces the bug and now passes",
            "The smallest in-scope fix is applied and the full suite result is recorded",
            "Root cause and rollback notes are explicit enough for human review",
        ],
    }
}

fn preset_evidence_requirements(preset: RalphPreset) -> &'static [&'static str] {
    match preset {
        RalphPreset::Generic => &[],
        RalphPreset::Init => &[
            "Repo map: key packages, boundaries, and entrypoints",
            "Environment bootstrap commands actually used",
            "Current test-suite outcome with blockers called out explicitly",
            "Warm-up report covering behavior from tests, gaps, and risks",
        ],
        RalphPreset::Bugfix => &[
            "Regression test diff that captures the bug",
            "Proof the regression test failed before the fix",
            "Proof targeted tests and the relevant suite are green after the fix",
            "Root cause summary and rollback note",
        ],
    }
}

fn preset_escalations(preset: RalphPreset) -> &'static [&'static str] {
    match preset {
        RalphPreset::Generic => &[],
        RalphPreset::Init => &[
            "The documented setup path is missing or requires secrets/admin access",
            "Tests fail before scoped work begins and the failures are unrelated",
            "Repository structure or ownership is too unclear to produce a reliable report",
        ],
        RalphPreset::Bugfix => &[
            "The bug cannot be reproduced with a stable failing test",
            "The fix needs schema, architecture, or dependency changes outside scope",
            "Full-suite regressions suggest the bug is a wider system problem",
        ],
    }
}

#[cfg(test)]
mod tests {
    use crate::{generate_spec, Capability, RalphPreset, StructureOutline, Tier};

    use super::{build_ralph_task_package, check_vertical_slicing};

    #[test]
    fn bugfix_preset_package_contains_regression_first_contract() {
        let mut spec = generate_spec("feat.bugfix-loop", Tier::Lightweight, "Prod");
        spec.artifacts
            .as_mut()
            .unwrap()
            .capabilities
            .push(Capability {
                id: "cap.fix-bug".into(),
                domain: "dom.workflow".into(),
                name: "FixBug".into(),
                description: None,
                since: None,
            });

        let workflow = spec.workflow.as_mut().unwrap();
        workflow.execution_mode.as_mut().unwrap().engine = "ralph".into();
        let build_phase = workflow
            .phases
            .iter_mut()
            .find(|phase| phase.id == "build")
            .unwrap();
        build_phase.scope = "Fix the retry counter off-by-one regression".into();

        let rendered = {
            let workflow = spec.workflow.as_ref().unwrap();
            let phase = workflow
                .phases
                .iter()
                .find(|phase| phase.id == "build")
                .unwrap();
            build_ralph_task_package(&spec, workflow, phase, RalphPreset::Bugfix).unwrap()
        };

        assert!(rendered.contains("Preset: ralph-bugfix"), "{}", rendered);
        assert!(
            rendered.contains("Write or update a regression test first"),
            "{}",
            rendered
        );
        assert!(
            rendered.contains("Proof the regression test failed before the fix"),
            "{}",
            rendered
        );
        assert!(
            rendered.contains("The bug cannot be reproduced with a stable failing test"),
            "{}",
            rendered
        );
    }

    #[test]
    fn structure_outline_renders_in_package() {
        let mut spec = generate_spec("feat.outline-test", Tier::Lightweight, "Prod");
        let workflow = spec.workflow.as_mut().unwrap();
        workflow.structure_outline = Some(StructureOutline {
            summary: "Add retry logic to HTTP client".into(),
            signatures: vec!["fn retry_with_backoff(req: Request, max: u32) -> Response".into()],
            types: vec!["RetryPolicy { max_attempts: u32, backoff_ms: u64 }".into()],
        });

        let rendered = {
            let workflow = spec.workflow.as_ref().unwrap();
            let phase = &workflow.phases[1];
            build_ralph_task_package(&spec, workflow, phase, RalphPreset::Generic).unwrap()
        };

        assert!(rendered.contains("Structure Outline:"), "{}", rendered);
        assert!(rendered.contains("Add retry logic"), "{}", rendered);
        assert!(rendered.contains("retry_with_backoff"), "{}", rendered);
        assert!(rendered.contains("RetryPolicy"), "{}", rendered);
    }

    #[test]
    fn no_outline_omits_section() {
        let spec = generate_spec("feat.no-outline", Tier::Lightweight, "Prod");
        let workflow = spec.workflow.as_ref().unwrap();
        let phase = &workflow.phases[1];
        let rendered =
            build_ralph_task_package(&spec, workflow, phase, RalphPreset::Generic).unwrap();

        assert!(!rendered.contains("Structure Outline:"), "{}", rendered);
    }

    #[test]
    fn horizontal_scope_triggers_warning() {
        let phase = crate::WorkflowPhase {
            id: "db-layer".into(),
            title: "Database Layer".into(),
            scope: "Create all database migrations and models".into(),
            roles: vec![],
            acceptance: vec![],
            rollback: vec![],
        };
        let warning = check_vertical_slicing(&phase);
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("horizontal"));
    }

    #[test]
    fn vertical_scope_no_warning() {
        let phase = crate::WorkflowPhase {
            id: "build".into(),
            title: "Build".into(),
            scope: "Implement retry logic end-to-end: model, service, and UI indicator".into(),
            roles: vec![],
            acceptance: vec![],
            rollback: vec![],
        };
        assert!(check_vertical_slicing(&phase).is_none());
    }
}
