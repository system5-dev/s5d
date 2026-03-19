---
name: s5d-trace
description: "Codebase traceability, architecture health, and impact analysis. Index code, trace spec-to-code bindings, measure coupling, detect drift."
argument-hint: "[path or spec to analyze]"
---

> **DEPRECATED.** This skill is superseded by the unified `/s5d` flow. Use `/s5d` instead.


# S5D Trace

Standalone — works with any codebase. No dependency on other S5D skills.

Bidirectional traceability between spec artifacts and source code. Spec owns meaning, code owns reality, trace links bind them. Architecture health metrics (coupling, cycles, WLNK) work independently of the full S5D spec lifecycle.

---

## Trace mode

Bidirectional traceability between spec artifacts and source code.

**Source hierarchy (priority order):**
1. `annotated` (0.95) — explicit `@s5d:` comments in code
2. `inferred` (0.7) — symbol name matches capability/entity name in codebase index
3. `inferred` (0.5) — file path matches component paths

**Annotation format** (optional — for disambiguation, not mandatory):
```rust
// @s5d:domain billing
// @s5d:capability AuthorizePayment
pub fn authorize_payment() { ... }
```

Supported comment styles: `//`, `#`, `/* */`, `--`, `*`. Valid artifact kinds: `domain`, `capability`, `entity`, `component`, `container`, `usecase`.

**Workflow:**
1. `s5d codebase index .` — build/update code index
2. `s5d trace build <spec>` — resolve bindings → stored in `.s5d/codebase.db`
3. `s5d trace check <spec>` — report coverage, find unmatched artifacts

Trace is separate from drift-check. `drift-check` compares spec fingerprint vs ledger state. `trace check` compares spec artifacts vs code bindings.

---

## Blast radius

`s5d trace blast-radius <spec>` — impact analysis from trace links. Shows:
- **Domains touched** — domains with at least one traced capability/entity/component
- **Capabilities/components affected** — spec artifacts with code bindings
- **Cross-domain edges** — from `spec.links.edges`, with archetypes
- **Files/symbols** — deduplicated count of code locations
- **Weakest link (WLNK)** — priority: UnmatchedArtifact > CrossDomainWithoutTrace > LowConfidenceMatch(<0.6)

---

## Codebase index

```
s5d codebase index .                        # full index: files, symbols, chunks → .s5d/codebase.db
s5d codebase update .                       # incremental update via git diff
s5d codebase search "<query>" [--top-k 10]  # full-text + semantic search
```

Index stored in `.s5d/codebase.db` (SQLite). Parses via tree-sitter: Rust, Python, TypeScript, Go, Java, C++. Extracts: file metadata, code chunks, symbols (functions, classes, structs) with line numbers. Used by `analyze` for higher-accuracy entity extraction and by `trace build` for symbol matching.

---

## Traceability

```
s5d trace build <spec>                      # scan code + @s5d: annotations → trace links
s5d trace check <spec>                      # coverage report: matched / unmatched artifacts
s5d trace report [--spec <id>]              # list all trace links
s5d trace blast-radius <spec>               # impact analysis: domains, files, WLNK
```

**Trace sources (priority order):**
1. `annotated` (confidence 0.95) — `// @s5d:capability AuthorizePayment` comments in code
2. `inferred` (confidence 0.7) — symbol name matches capability/entity name in codebase index
3. `path` (confidence 0.5) — file path matches component.paths field

**blast-radius** shows: domains touched, capabilities/components affected, files/symbols count, cross-domain edges, weakest link (WLNK = UnmatchedArtifact > CrossDomainWithoutTrace > LowConfidenceMatch < 0.6).

---

## Architecture health

```
s5d check <spec>                            # health score, cycles, coupling — exit 0 if healthy
s5d check <spec> --threshold 80             # CI gate: exit 1 if aggregate score < 80
s5d check <spec> --compare                  # compare against saved baseline, show degradation
s5d check <spec> --format json              # machine-readable output
s5d metrics <spec>                          # per-domain table: Ca, Ce, I, Score, Violations
s5d metrics <spec> --format json            # JSON output
```

Computes per-domain coupling metrics from edge graph:
- **Ca** (Afferent Coupling) — count of edges pointing TO this domain
- **Ce** (Efferent Coupling) — count of edges pointing FROM this domain
- **I** (Instability) — Ce / (Ca + Ce). 0.0 = fully stable, 1.0 = fully unstable
- **Cycles** — Tarjan SCC on domain edges, finds ALL cycles
- **Layering** — checks Core > Supporting > Generic dependency direction

Health score 0-100 per domain (start at 100, deduct penalties):
- Cycle participation: −15
- Layering violation: −10
- High instability (I > 0.8 AND Ce > 3): −5
- Hub domain (Ca > 5): −3
- Scattered dependencies (Ce > 5): −3

**Aggregate = min(domain scores)** following WLNK principle.

`--compare` loads previous snapshot from `.s5d/records/{spec_id}.health.yaml`, computes degradation:
- delta > 5: improved
- delta in [-5, 5]: stable
- delta in [-15, -5): degraded
- delta < -15: critical

Snapshot auto-saved on each `check` run.

---

## Monitoring

```
s5d status                                  # project overview: spec counts, lifecycle states
s5d drift-check [<spec>]                    # fingerprint comparison: synced vs drifted
s5d reconcile [<spec>]                      # re-import to fix drift
s5d report                                  # REPORT.md with adoption metrics
```

---

## suggest-domains

Coupling-based domain boundary suggestion from codebase index. Groups files into modules by directory depth, builds module-level dependency graph from import edges, computes Ca/Ce/Instability/Cohesion per module, classifies by PageRank centrality (core/supporting/generic), and identifies merge candidates (modules with high mutual coupling).

```
s5d codebase suggest-domains <path> [--depth 1] [--min-files 2] [--format text|json]
```

**Metrics per domain:**
- **Ca** (afferent) — how many other modules depend on this one
- **Ce** (efferent) — how many modules this one depends on
- **I** (instability) — Ce/(Ca+Ce), 0=stable, 1=unstable
- **Coh** (cohesion) — internal_edges/total_edges, higher=better boundary
- **Conf** (confidence) — composite score: how good a domain boundary this is

**Merge candidates:** modules where mutual coupling > 30% of the smaller module's dependencies. Suggests these should be one domain.

**Prerequisite:** `s5d codebase index .` must be run first.

---

## generate-rules

Derives executable architecture constraints from an S5D spec. Reads domain classifications and declared edges, produces rules that can be checked by linters or CI.

```
s5d generate-rules <spec> [--format yaml|json]
```

**Rule types generated:**
- **Layering** — lower-level domains (generic) must not depend on higher-level (core). Based on `domain.classification`: core(3) > supporting(2) > generic(1)
- **Allowed edges** — each domain may only depend on domains explicitly declared in `spec.links.edges`. Any other cross-domain dependency is a warning
- **No cycles** — circular dependencies between domains are forbidden

**Output:** YAML (default) or JSON. Rules include id, severity (error/warning), description, and type-specific fields.

---

## drift-check and reconcile

### drift-check
Compares current state fingerprint vs last import's fingerprint. If different → Drifted. Can check all specs if no arg given.

### reconcile
Re-imports to fix drift. Bypasses diff_sha256 check (allows alias state changes). Still checks spec hasn't been edited since approval. Appends ledger entry: action="reconcile".

---

## render

Generates diagrams. Views: `domain` (domain map), `components` (deployment), `decision` (decision tree). Formats: `mermaid` (default), `html` (interactive D3.js, domain view only). `-o <file>` or stdout.

```
s5d render <spec> --view domain --format html
s5d render <spec> --view components --format mermaid
s5d render <spec> --view decision
```

---

## CLI Enforcement & Structured Output

**Every CLI command MUST be executed via Bash tool.** Check exit code before presenting results. No claiming results without running the command.

After each operation, display a structured PRESENT block:

**Codebase index:**
```
┌─ S5D | CODEBASE INDEX ───────────────────────────────────────┐
│  Path:    <indexed path>                                     │
│  Files:   <count>                                            │
│  Symbols: <count>                                            │
│  Chunks:  <count>                                            │
│  CLI: ✓ s5d codebase index                                   │
│  Explanation: <what was indexed, languages detected>         │
└───────────────────────────────────────────────────────────────┘
```

**Trace build:**
```
┌─ S5D | TRACE BUILD ──────────────────────────────────────────┐
│  Spec: <spec ID>                                             │
│  Bindings: <annotated> annotated / <inferred> inferred / <path> path │
│  Unmatched: <count> artifacts without code bindings          │
│  CLI: ✓ s5d trace build                                      │
│  Explanation: <coverage quality, what's missing>             │
└───────────────────────────────────────────────────────────────┘
```

**Trace check:**
```
┌─ S5D | TRACE CHECK ──────────────────────────────────────────┐
│  Spec: <spec ID>                                             │
│  Coverage: <matched>/<total> (<percentage>%)                 │
│  Unmatched: <list of unmatched artifact IDs>                 │
│  CLI: ✓ s5d trace check                                      │
│  Explanation: <what to do about gaps>                        │
└───────────────────────────────────────────────────────────────┘
```

**Blast radius:**
```
┌─ S5D | BLAST RADIUS ─────────────────────────────────────────┐
│  Spec: <spec ID>                                             │
│  Domains:    <count>                                         │
│  Components: <count>                                         │
│  Files:      <count>                                         │
│  Cross-domain edges: <count>                                 │
│  WLNK: <weakest link type + detail>                         │
│  CLI: ✓ s5d trace blast-radius                               │
│  Explanation: <impact assessment, risk areas>                │
└───────────────────────────────────────────────────────────────┘
```

**Health check:**
```
┌─ S5D | HEALTH CHECK ─────────────────────────────────────────┐
│  Spec: <spec ID>                                             │
│  Score: <aggregate> (WLNK: <weakest domain>)                │
│  Cycles: <count>                                             │
│  Layering violations: <count>                                │
│  Degradation: <improved | stable | degraded | critical>      │
│  CLI: ✓ s5d check                                            │
│  Explanation: <what's healthy, what needs attention>         │
└───────────────────────────────────────────────────────────────┘
```

**Metrics:**
```
┌─ S5D | METRICS ──────────────────────────────────────────────┐
│  Spec: <spec ID>                                             │
│  ┌────────────┬────┬────┬──────┬───────┬────────────┐       │
│  │ Domain     │ Ca │ Ce │ I    │ Score │ Violations │       │
│  ├────────────┼────┼────┼──────┼───────┼────────────┤       │
│  │ <domain>   │ <> │ <> │ <.x> │ <n>   │ <list>     │       │
│  └────────────┴────┴────┴──────┴───────┴────────────┘       │
│  CLI: ✓ s5d metrics                                          │
│  Explanation: <coupling assessment, hub/instability risks>   │
└───────────────────────────────────────────────────────────────┘
```

---

## Key implementation patterns (trace-relevant)

- **Tarjan SCC:** Finds ALL cycles in domain graph, not just the first. Used by both `graph-check` and `check`.
- **Trace confidence:** annotated=0.95, inferred=0.7, path=0.5. Weighted binding resolution.
- **WLNK health:** Aggregate score = min(domain scores). One cycle or hub domain ruins the aggregate — identify it with `metrics`.
- **Codebase index:** SQLite + tree-sitter. Parses Rust, Python, TypeScript, Go, Java, C++. `analyze` uses it for entity extraction; `trace build` uses it for symbol matching.
