---
name: s5d-feature
description: "S5D feature lifecycle — spec, preview, approve, gates, import, reflect. Build features with verification and traceability."
argument-hint: "[feature description or spec ID]"
---

> **DEPRECATED.** This skill is superseded by the unified `/s5d` flow. Use `/s5d` instead.


# S5D Feature

Requires: `/fpf` for reasoning vocabulary.

Feature lifecycle from spec to production. Entry from a Decision winner (Step 6) or direct request. Produces a frozen spec, gated import, and a reflection record.

---

## Step 7. Implement

**INPUT:** Decision winner from Step 6.

**OUTPUT:** Feature spec JSON before writing YAML:
```json
{
  "id": "feat.payment-processing",
  "tier": "standard",
  "context": "## Problem\n...\n\n## Goal\n...",
  "artifacts": {
    "domains": [{ "id": "...", "product": "...", "classification": "core", "name": "...", "description": "..." }],
    "capabilities": [{ "id": "...", "domain": "...", "name": "VerbNoun" }],
    "entities": [{ "id": "...", "domain": "...", "name": "...", "description": "..." }],
    "features": [{ "id": "feat.payment-processing", "product": "...", "name": "..." }],
    "use_cases": [{ "id": "...", "name": "...", "feature": "feat.payment-processing" }],
    "components": [{ "id": "...", "domain": "...", "container": "...", "feature": "feat.payment-processing", "name": "...", "paths": ["..."] }],
    "containers": [{ "id": "...", "name": "...", "system": "..." }]
  },
  "links": {
    "feature_to_domain": [{ "feature": "feat.payment-processing", "domain": "payment" }],
    "edges": [{ "from": "...", "to": "...", "archetype": "..." }],
    "component_to_capability": [{ "component": "...", "capability": "..." }],
    "component_to_entity": [{ "component": "...", "entity": "..." }],
    "component_to_container": [{ "component": "...", "container": "..." }]
  },
  "verification": {
    "scenarios": [
      { "title": "Happy path", "given": "...", "when": "...", "then": "..." }
    ],
    "invariants": ["..."],
    "boundaries": ["OUT_OF_SCOPE: ..."]
  },
  "gates": ["schema", "graph", "lint"]
}
```

**VALIDATE:**
- `spec.id` == `artifacts.features[0].id` (single-feature invariant)
- All edge targets reference declared domains
- All component domains reference declared domains
- All component containers reference declared containers
- Capability names are VerbNoun

**CLI (MANDATORY):**
1. Write YAML to `.s5d/packages/<id>__<date>.s5d.yaml`
2. `s5d validate <spec>` — must pass
3. `s5d graph-check <spec>` — must pass
4. If either fails → fix schema, re-generate, do NOT skip

Agent MUST run validate and graph-check. Do not skip. Show output in PRESENT block.

**PRESENT:**
```
┌─ S5D | IMPLEMENT (SPEC) | 7 ────────────────────────────────┐
│                                                               │
│  Spec: <spec ID>                                             │
│  Tier: <standard | high | lightweight>                       │
│  From decision: <decision ID or "standalone">                │
│                                                               │
│  Artifacts:                                                   │
│    Domains:      <count> (<core/supporting/generic>)         │
│    Capabilities: <count>                                      │
│    Entities:     <count>                                      │
│    Components:   <count> (<new/modified>)                    │
│    Use cases:    <count>                                      │
│                                                               │
│  Verification:                                                │
│    Scenarios:  <count>                                        │
│    Invariants: <count>                                        │
│    Boundaries: <count>                                        │
│                                                               │
│  Gates: <gate list>                                          │
│                                                               │
│  CLI results:                                                 │
│    validate:    ✓ passed | ✗ <error>                         │
│    graph-check: ✓ passed | ✗ <error>                         │
│                                                               │
│  Explanation:                                                 │
│  <What the spec covers. Key design choices made.             │
│   How it connects to the parent decision (if any).>          │
│                                                               │
│  → Next: Preview → Approve → Gates → Import                  │
└───────────────────────────────────────────────────────────────┘
```

If this decision does not lead to implementation (pure strategy, org change) — WAIVER required:
```
WAIVER: step.implement
Reason: [why no implementation artifact]
Condition to lift: [what would require a spec]
Approved by: [name — or "self" if solo]
```

---

## Feature mode

Entry: winning hypothesis from Decision, or direct user request to build.

**Direct entry** (no prior Decision): use `--hypothesis-id` to link to an existing decision hypothesis, or omit for standalone features. Standalone features have no DRR — document the rationale in the spec's `context` field. Stepping stone is not applicable for direct entry.

1. Check if `.mcp.json` exists BEFORE running init.
2. `s5d init` — ensure `.s5d/` exists + registers `.mcp.json` (idempotent)
3. **If `.mcp.json` did NOT exist before init → STOP.** Tell user to restart agent session, then re-run. Do NOT proceed.
4. `s5d analyze . --product <p>` — scan codebase (skip only if recent analysis exists)
5. What are we building? (from which decision/winner)
4. What constraints are already fixed?
5. Spec (with Verification section) → Preview → Approve → Test Contract → Execute → Verify (gates) → Reflect

**Test Contract** (after Approve, before Execute): the approved spec's Verification section translated into executable tests or explicit test cases in the repository. Required for High tier. Recommended for Standard. Tests first, code to pass them second.

The test contract is the output of "Approve" and the input of "Execute". S5D currently does not require a separate test-contract file under `.s5d/`; the contract lives in the approved spec and the resulting tests.
Use the recorded role map when execution starts. The multi-agent team is instantiated from the decision/spec roles, not improvised ad hoc mid-flight.

If blocked mid-build → spawn child Decision. Transition is explicit.

Exit: spec applied, gates passed, reflection recorded.
Output artifact: frozen `.s5d.yaml` + living `.record.yaml`.

```
s5d new feat.<id> --tier <tier> --product <p>
s5d validate <spec>
s5d preview <spec>
s5d approve <spec> --reviewer <name>
s5d run-gates <spec>       # PASS or WAIVER per gate — no silent SKIP. Evidence saved to .s5d/evidence/
s5d import <spec> [--verified-by "<name>"]   # verifier ≠ approver trust separation
s5d reflect <spec> --summary "<what happened>" --heuristic "<rule>" --issue "<desc|cause|fix|severity>"
s5d learn report                             # aggregate all reflections
s5d learn feed <spec>                        # pull relevant heuristics for new spec
```

### CLI Enforcement (Feature lifecycle)

**Every CLI command in the lifecycle MUST be executed via Bash tool.** The agent must not claim a step is done without running the command and checking the exit code.

| Step | CLI required? | Tool call |
|------|--------------|-----------|
| Create spec | Yes | `s5d new feat.<id>` |
| Validate | Yes | `s5d validate <spec>` |
| Graph check | Yes | `s5d graph-check <spec>` |
| Preview | Yes | `s5d preview <spec>` |
| Approve | Yes (human name) | `s5d approve <spec> --reviewer <name>` |
| Gates | Yes | `s5d run-gates <spec>` |
| Import | Yes | `s5d import <spec>` |
| Reflect | Yes | `s5d reflect <spec>` |

**Enforcement rule:** Display PRESENT block only AFTER the CLI command has been executed and verified. If CLI fails — diagnose and fix, do not skip.

### PRESENT blocks (Feature lifecycle)

After each lifecycle step, display a structured block:

**Preview:**
```
┌─ S5D | PREVIEW ──────────────────────────────────────────────┐
│  Spec: <spec ID>                                             │
│  Diff: create <n> / update <n> / link <n> / delete <n>      │
│  diff_sha256: <hash>                                         │
│  CLI: ✓ s5d preview                                          │
│  Explanation: <what will change in the alias table>          │
│  → Next: Approve                                             │
└───────────────────────────────────────────────────────────────┘
```

**Approve:**
```
┌─ S5D | APPROVED ─────────────────────────────────────────────┐
│  Spec: <spec ID>                                             │
│  Reviewer: <name>                                            │
│  spec_sha256: <hash>                                         │
│  CLI: ✓ s5d approve                                          │
│  → Next: Run gates                                           │
└───────────────────────────────────────────────────────────────┘
```

**Gates:**
```
┌─ S5D | GATES ────────────────────────────────────────────────┐
│  Spec: <spec ID>                                             │
│  Results:                                                     │
│    schema: ✓ passed | ✗ failed | ⚠ waived                   │
│    lint:   ✓ passed | ✗ failed | ⚠ waived                   │
│    test:   ✓ passed | ✗ failed | ⚠ waived                   │
│  CLI: ✓ s5d run-gates                                        │
│  Explanation: <any failures and how resolved>                │
│  → Next: Import                                              │
└───────────────────────────────────────────────────────────────┘
```

**Import:**
```
┌─ S5D | IMPORTED ─────────────────────────────────────────────┐
│  Spec: <spec ID>                                             │
│  Status: Applied                                             │
│  Verified by: <name>                                         │
│  CLI: ✓ s5d import                                           │
│  Explanation: <spec applied to alias table and ledger>       │
│  → Next: Implement code, then Reflect                        │
└───────────────────────────────────────────────────────────────┘
```

**Reflect:**
```
┌─ S5D | REFLECTED ────────────────────────────────────────────┐
│  Spec: <spec ID>                                             │
│  Status: Operated (terminal)                                 │
│  Summary: <what happened>                                    │
│  Heuristics: <reusable rules captured>                       │
│  Issues: <structured issues, if any>                         │
│  CLI: ✓ s5d reflect                                          │
│  → Lifecycle complete                                        │
└───────────────────────────────────────────────────────────────┘
```

---

## Signoff mode

When asked "is this ready/safe/done?":
1. Run gates → collect evidence → verdict → human signs off or rejects
2. If gate failed: fix OR record WAIVER with justification + condition to lift
3. SKIP without WAIVER = BLOCK. Not a pass. Not acceptable.

Gate result: PASS | BLOCK (must fix) | WAIVER (explicit justification + condition)
Default gates by tier: Lightweight = schema. Standard = schema + graph + lint. High = schema + graph + lint + test + contract.

---

## Drift mode

When applied spec diverges from codebase:
- Detect → Classify (accidental vs intentional) → Reconcile or reopen as Feature
- `s5d drift-check <spec>` → `s5d reconcile <spec>`

**Resolution paths:**
- **Accidental drift** (alias table corrupted, external edit): `reconcile` restores spec state.
- **Intentional drift** (code evolved beyond spec): create a **new** feature spec (`new --tier standard`) that captures the actual architecture, then `import` it. The old spec remains as-is (historical record). Do NOT edit the old spec post-approval — create a successor instead.

---

## Learn phase (feedback loop)

After gates pass → agent calls `reflect` with `--heuristic` to capture reusable rules. Before starting new work → agent calls `learn feed` to pull relevant heuristics from past specs. Periodically → `learn report` shows systemic patterns.

Agent workflow (automatic, human doesn't trigger):
1. After Verify: `s5d reflect <spec> --summary "..." --heuristic "..." --worked "..." --issues "..." --issue "desc|root_cause|fix|severity"`
2. Before new spec work: `s5d learn feed <spec>` → check past lessons for same domains
3. On request: `s5d learn report` → aggregated heuristics, recurring issues, open follow-ups

---

## Autonomy — Compute Before Asking

If derivable from data — compute, don't ask. Human-in-the-loop ONLY for `approve --reviewer` (non-waivable) and genuinely ambiguous trade-offs.

Computable: domain classification, entity relations, component paths, edge archetypes, gate config (from tier), product name (from repo).

**Default to reasonable choices. Infer. Derive. Ask only when genuinely stuck.**

---

## Do NOT

- Claim "verified" without running gates
- Patch `.record.yaml` manually to change gate status — manual edits are unsupported and break process trust
- Use `--force` on import without a WAIVER record. `decide --confirmed-by` and `approve --reviewer` are non-waivable — no `--force`, no WAIVER, no workaround
- Mix Decision and Feature in one flow — separate, linked by explicit transition

---

## Workflow

### Lifecycle state machine

Every spec moves through states. Arrows = commands that trigger transitions. No skipping without WAIVER.

```
     ┌────────┐     validate        ┌──────────┐
     │Proposed│────────────────────→│ validated │ (internal, not a record status)
     └────────┘                     └─────┬────┘
                                          │
                    graph-check           │
                    check (health)        │  (can run anytime, not gating)
                                          │
                                    preview
                                          │
                                          ▼
                                   ┌───────────┐
                                   │ Previewed  │  diff_sha256 computed
                                   └──────┬────┘
                                          │
                                    approve --reviewer
                                          │
                                          ▼
                                   ┌───────────┐
                                   │ Approved   │  spec_sha256 + diff_sha256 bound
                                   └──────┬────┘
                                          │
                                    run-gates
                                          │
                                   ┌──────┴──────┐
                                   │ gate pass?  │
                                   │  or waiver? │
                                   └──────┬──────┘
                                          │
                                     import
                                          │
                                          ▼
                                   ┌───────────┐
                                   │  Applied   │  alias table + ledger updated
                                   └──────┬────┘
                                          │
                              ┌───────────┼───────────┐
                              │           │           │
                        drift-check   reflect    rollback
                              │           │           │
                              ▼           ▼           ▼
                         ┌────────┐ ┌──────────┐ ┌──────────┐
                         │Drifted │ │ Operated │ │ Approved │
                         └───┬────┘ │(terminal)│ │ (revert) │
                             │      └──────────┘ └──────────┘
                        reconcile
                             │
                             ▼
                        ┌────────┐
                        │ Synced │
                        └────────┘
```

### Blocking rules

| To reach...   | You MUST have...                                                |
|---------------|-----------------------------------------------------------------|
| Previewed     | Valid spec (validate passes)                                     |
| Approved      | Preview completed (diff_sha256 exists)                           |
| Applied       | Approval + all gates passed/waived + SHA256 chain intact         |
| Operated      | Applied + reflect called with production evidence                |

### SHA256 integrity chain

```
preview  →  diff_sha256 ──────────────┐
                                       ├──→ import verifies BOTH match
approve  →  spec_sha256 + diff_sha256 ─┘
```

If spec file changes after preview → must re-preview + re-approve.
If alias table changes after preview → diff_sha256 changes → must re-preview + re-approve.
Reconcile bypasses diff_sha256 check (for drift repair only).

---

## CLI reference — Feature path

```
s5d new feat.<id> --tier standard --product <p> [--hypothesis-id <id>]
s5d validate <spec>
s5d graph-check <spec>
s5d preview <spec>
s5d approve <spec> --reviewer <name>
s5d run-gates <spec>
s5d waiver <spec> --gate <name> --reason "<why>" --condition "<when>" --approved-by "<name>"
s5d import <spec> [--verified-by "<name>"] [--force]
s5d rollback <spec>
s5d reflect <spec> --summary "<...>" --heuristic "<rule>" --worked "<...>" --issues "<...>" --issue "<desc|cause|fix|severity>"
s5d learn report
s5d learn feed <spec>
```

---

## Command details

### init
Creates `.s5d/` directory with `packages/`, `records/`, `config.yaml`, `ledger.yaml`, `index.yaml`, `aliases.yaml`. Also creates `knowledge/` directories (L0, L1, L2, decisions). Idempotent.

### new (feature tiers)
Creates spec + record from template. Tiers: `note` (requires --rationale), `lightweight`, `standard`, `high`. Feature tiers auto-include gates by tier level. `--hypothesis-id` links the new spec to a hypothesis in a parent decision spec.

Files created: `.s5d/packages/{id}__{YYYYMMDD}.s5d.yaml` + `.s5d/records/{id}__{YYYYMMDD}.record.yaml`.

### analyze
Scans codebase and generates draft S5D spec with discovered artifacts.

**Languages:** Rust (Cargo.toml, workspace members), Python (manage.py → Django apps, pyproject.toml), TypeScript/JavaScript (package.json, detects Next.js/Express/Fastify), Go (go.mod).

**What it discovers:**
- Domains from: workspace members (Rust), Django apps with models.py (Python), monorepo dirs (`services/`, `packages/`, `apps/`, `modules/`), special dirs (`ui/`, `frontend/`, `infrastructure/`)
- Entities: `pub struct` (Rust), `class X(models.Model|BaseModel)` / `@dataclass` (Python), `export interface|class|type` (TS), `type X struct` (Go)
- Capabilities: public functions/methods from views, handlers, services, routes, endpoints files
- Components: files matching `*service*`, `*handler*`, `*controller*`, `*worker*`, `*middleware*`
- Cross-domain edges: from `use`/`import`/`from` statements between domains

**Classification heuristic:** auth/user/identity/account → Generic. notification/email/log/monitor → Supporting. Everything else → Core.

If `.s5d/codebase.db` exists, uses indexed symbols for higher accuracy; falls back to regex.

Flags: `--product <name>`, `--id <prefix>` (default: "analysis"), `-o <file>`.

### validate
Schema + semantic validation. Checks: s5d version, ID format, tier-specific requirements (note needs rationale, decision needs problem.signal and no artifacts, feature needs exactly 1 feature with id == spec.id), artifact naming conventions, domain classification values (core/supporting/generic), maturity levels (genesis/custom/product/commodity), transport types (rest/grpc/messaging/graphql/websocket), evidence formality (0-9), contract formats (openapi/json_schema/protobuf/typespec), gate configurations. Does NOT check graph structure — that's graph-check.

### graph-check
Validates structural integrity of spec graph:
- **Cycle detection:** Tarjan SCC algorithm on domain edges. Finds ALL strongly connected components (size > 1), not just the first cycle.
- **Layering:** Core (3) > Supporting (2) > Generic (1). Lower-level domain depending on higher-level = violation.
- **Binding integrity:** All IDs in feature_to_domain, use_case_to_capability, component_to_entity etc. must reference declared artifacts.
- Bidirectional archetypes (shared_kernel, partnership, separate_ways) excluded from cycle detection.

Exit code 3 on violations.

### preview
Dry-run of import. Loads alias table, applies renames, computes diff (create/update/link/delete counts), computes SHA256 of diff actions. Stores diff_sha256 in record. Sets record.status = Previewed. Idempotent — same spec yields same diff_sha256.

### approve
Records reviewer consent. Checks: record.status == Previewed, spec hasn't changed since preview (SHA256 match). Creates Approval entry binding spec_sha256 + diff_sha256 + reviewer + timestamp. Sets record.status = Approved. Supports multiple approvals.

### run-gates
Executes gate commands from `.s5d/config.yaml`. Substitutes `{package}` → spec path, `{project}` → project root. Runs with timeout (default 120s), captures stdout/stderr. Records result per gate (passed/failed/timeout/skipped). Gate kinds by tier: lightweight=schema, standard=schema+lint+test, high=schema+lint+test+security+contract. Skipped gates (no command configured) recorded as "skipped".

**Evidence capture:** Each gate execution writes full untruncated output to `.s5d/evidence/{spec_id}/{kind}_{attempt}.log`. The GateResult in `.record.yaml` stores: `exit_code` (raw exit code), `evidence_path` (relative path to log file), `command` (resolved command args), `duration_ms` (wall-clock time). The `log` field remains truncated at 10KB for readability; `evidence_path` points to the complete output for audit.

### import
Applies spec to alias table + ledger. Pre-flight: record must be Approved, all gates must be passed (or waived), spec_sha256 must match approval, diff_sha256 recomputed and must match. Transactional — failure = no alias table change. Records state_fingerprint (SHA256 of alias state) for drift detection. Sets record.status = Applied, sync_status = Synced.

**Verifier separation:** `--verified-by <name>` records who independently verified gates passed. Methodological check: verifier should differ from the last approver (trust separation — builder ≠ verifier). Solo developers override with `--force`. The `verified_by` field is stored in `.record.yaml`.

```
s5d import <spec> --verified-by "alice" [--force]
```

### rollback
Reverses last successful import. Reverts alias table, appends rollback entry to ledger. Sets record.status back to Approved.

### reflect
Records production evidence and closes lifecycle. Captures: summary, worked (what went well), issues, follow_ups, evidence (paths/URLs), heuristics (reusable rules). Sets record.status = Operated (terminal).

**Structured issues:** `--issue "description|root_cause|fix|severity"` records machine-readable failure analysis. Pipe-delimited, only description is required. Severity values: low, medium, high, critical. Stored as `structured_issues` in `.record.yaml` alongside free-text `issues`.

```
s5d reflect <spec> --summary "..." \
  --issue "Cold start 3s slower|Missing cache warmup|Add warmup to deploy|medium" \
  --issue "Memory spike on bulk import||Needs profiling|high" \
  --heuristic "Cache warmup before deploy reduces cold starts"
```

### show
Formatted display. Feature specs: tier, status, artifact counts, gate count.

### search
Full-text search across all `.s5d.yaml` files in packages/ and markdown in knowledge/ directories. Case-insensitive. Shows matching lines with context.

### report
Generates REPORT.md with metrics: total specs by tier/status, leading indicators (new specs/30d, approval rate), lagging indicators (applied rate, synced rate), anti-metrics (lightweight overuse, stale specs, drift rate), learn metrics (operated count, heuristics, open follow-ups).

### status
Project overview: total specs, in-progress count, decisions pending. Per-spec table: ID, Version, Tier, Status, Sync status.

### git-status
Git tracking state per spec: tracked?, on main?, merged?, last commit. Suggests lifecycle phase based on git state.

### rename-domain
Cascading rename of domain ID across entire spec: domain.id, all capabilities.domain, entities.domain, components.domain, edges from/to, all bindings.

### contract-check
Validates contracts: path exists, sha256 matches file, format is valid (openapi/json_schema/protobuf/typespec), binds_to is non-empty.

### bootstrap
Seeds alias table from manifest YAML. For migrating existing systems. Manifest: list of id/type/uuid/package_id entries.

---

## Configuration files

**.s5d/config.yaml** — gate runner config:
```yaml
gate_commands:
  schema: ["cargo", "build"]
  lint: ["cargo", "clippy", "--", "-D", "warnings"]
  test: ["cargo", "test", "--quiet"]
gate_runner:
  cwd: "{project}"
  timeout_s: 120
  env_deny: [AWS_SECRET_ACCESS_KEY, GITHUB_TOKEN]
```

**.s5d/aliases.yaml** — UUID → artifact ID mapping (identity table).
**.s5d/ledger.yaml** — import/reconcile/rollback history with fingerprints.
**.s5d/index.yaml** — quick-lookup feature index (id, path, status, product, version).

---

## Key implementation patterns

1. **Two-File Model:** Spec is immutable post-approval. Mutable state (decision, reflection, gates) → record file.
2. **SHA256 binding:** Approve binds (spec_sha256, diff_sha256). Import verifies both haven't changed.
3. **State fingerprint:** SHA256 of sorted alias table → used by drift-check to detect changes.
4. **WLNK:** Health score = min(domain scores). One cycle or hub ruins aggregate.
5. **Layer progression:** L0 → L1 → L2 via evidence verdicts (pass advances, fail invalidates).
6. **Tarjan SCC:** Finds ALL cycles in domain graph, not just the first.
7. **Trace confidence:** annotated=0.95, inferred=0.7, path=0.5.
8. **Terminal states:** Operated, Deprecated, Removed — no further transitions.
9. **Idempotent:** preview, reconcile, check are safe to rerun.
10. **Codebase index:** SQLite + tree-sitter. Used by analyze (entity extraction) and trace (symbol matching).
