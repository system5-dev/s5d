# domain-refactor — anti-pattern playbook

**Last verified:** 2026-05-28

Each entry: what the pattern looks like, why it hurts, the standard refactor.
The skill's `analyze.sh` detects some of these mechanically; others need
agent judgement against the architecture-map and test coverage.

---

## God-component

**Shape:** one file > 30 KB with ≥3 exports, often owning multiple capabilities.

**Pain:** every change risks breaking unrelated capabilities. Code review
becomes futile. Test coverage degrades because tests are too coarse to
exercise each path. Merge conflicts cluster on the file.

**Refactor:** decompose along **capability seams** (from architecture-map §2).
One file per capability, public API preserved via a barrel re-export
(only if external consumers can't be updated in the same commit).

**Worked example (large shared `utils/http-errors.ts`, 62 KB, 24 exports):**
1. Identify capabilities — error type definitions vs. response builders vs.
   logging shims vs. status-code helpers.
2. Verify each capability has a clear consumer (`grep -r "import.*http-errors"`).
3. Write or augment tests covering each capability (it's classified `unsafe`
   in analyze.sh — coverage <30% — so tests are prerequisite).
4. Per-capability: `apply.sh move-component utils/http-errors.ts
   lib/http/<capability>.ts`. Each move is one commit.
5. After all caps moved: barrel re-export from the original path if external
   imports still exist; otherwise delete.

---

## Domain leak

**Shape:** component in a generic/supporting domain (e.g. `notifications`,
`analytics`, `auth-identity`) imports a core-domain entity directly (`Policy`,
`Apply`, etc.) instead of receiving a projection.

**Pain:** core-domain entities change shape; generic code must follow.
Couples reuse-friendly modules to product-specific data.

**Refactor:** introduce a projection type owned by the consumer (the generic
domain), populated by the core-domain caller. The contract becomes:
"consumer declares what it needs; producer adapts the entity."

```ts
// Before
// lib/email.ts (notifications domain) — leaks Policy entity
import { Policy } from '@/prisma/types';
export async function sendBindConfirmation(policy: Policy) { ... }

// After
// lib/email/types.ts (notifications domain) — owns projection
export interface BindConfirmationEmail {
  to: string;
  policyNumber: string;
  premium: string;       // formatted, not Decimal
  coverageDates: string; // formatted, not Date[]
}
// lib/email/templates/bind-confirmation.ts
export async function sendBindConfirmation(input: BindConfirmationEmail) { ... }

// app/api/.../route.ts (core caller) — adapts entity → projection
import { sendBindConfirmation } from '@/lib/email/templates/bind-confirmation';
await sendBindConfirmation({
  to: policy.user.email,
  policyNumber: policy.policyNumber,
  premium: formatMoney(policy.premium),
  coverageDates: formatDateRange(policy.coverageStartDate, policy.coverageEndDate),
});
```

The architecture-map §3 already says "Cross-domain entity access uses
projections/contracts" — this anti-pattern is a direct violation of that rule.

---

## Missing contract on a cross-domain edge

**Shape:** §7 Edges in the map lists `domain-A → domain-B` but the contract
column is blank, `n/a`, or `unknown`. In code: domain-B imports domain-A's
internal types or modules directly.

**Pain:** internal changes in A silently break B. No type-checked boundary.

**Refactor:** define a shared contract owned by **the downstream consumer**
(B owns "what it needs from A"). Land the contract file first; then refactor
the call site to use it.

Common contract shapes:
- TypeScript interface + Zod schema (for runtime+compile validation).
- OpenAPI / TypeSpec definition for HTTP edges.
- Protobuf for gRPC edges.
- Shared event schema (Avro / JSON Schema) for queue edges.

---

## Drift-missing (file gone, map stale)

**Shape:** map row lists a path that no longer exists. Often happens after
a rename / move that didn't update the map.

**Pain:** trace.sh / orphan detection unreliable. Future agents have no idea
where the capability moved.

**Refactor:** in one commit, either restore the file (intent was to keep)
or remove the row (intent was to delete). Brace-expansion shorthand in
the map (`{x,y}.ts`) should be expanded to one row per file.

---

## Capability duplication

**Shape:** §2 Capabilities lists the same id (e.g. `apply.collect-form`)
twice in two rows. Usually a copy-paste error.

**Pain:** ambiguity about which row is canonical. Cascades to incorrect
traces and inconsistent feature spec linkage.

**Refactor:** rename one of the duplicates (often: split into two distinct
capabilities with clearer scopes), or merge into one if they really are
the same.

---

## Orphan (substantial file, not in map)

**Shape:** ≥200-line file under `app/`, `components/`, `lib/`, etc. that no
map row mentions.

**Pain:** when refactoring an adjacent module, the agent cannot tell which
capability/domain this orphan belongs to. Cross-domain edges through it
are invisible.

**Refactor:** EITHER promote the file to a map row (add to §5 with its
domain, feature, container, capabilities) OR confirm it's purely
internal/leaf and document that decision in §5's preamble.

**Note:** the architecture map §5 is intentionally a SELECTION ("large or
boundary-bearing components"), not exhaustive. Most orphans are correctly
unregistered. Use the orphan list as a prompt: "should this be registered?"

---

## Step-numbering chaos (consumer-app pattern, generalisable)

**Shape:** UI step files named `Step1*.tsx`, `Step2*.tsx`, …, `Step16*.tsx`,
with duplicate numbers across variants (Step9 + Step9, Step10 empty file,
Step12 god-component 98 KB).

**Pain:** numbering becomes a hidden coupling between UI files and the
applicationStore.ts `currentStep`. Renumbering a step requires touching
≥5 files; deleting a step requires careful audit.

**Refactor:** convert numbered steps to **named** capabilities. Map keeps
the numeric order as an explicit field, files use semantic names
(`steps/EventType.tsx`, `steps/Attendance.tsx`, etc.). Renumbering becomes
a map edit, not a file rename.

This is a multi-slice refactor: one slice per Step → semantic-name rename,
with the store update + map update + e2e selector update in each commit.

---

## Mismatched money representations

**Shape:** `Policy.premium` is `Decimal(10,2)` in Prisma, `QuotePreview.*Cents`
is integer cents in the same schema. Conversion lives in scattered places.

**Pain:** drift between the two representations is silent until a rounding
edge case bites. Auditability of monetary trails is poor.

**Refactor:** pick ONE representation (typically cents-integer for
compute, formatted-string for display) and add a typed adapter at the
boundary. Land the adapter; then convert call sites slice-by-slice; then
drop the dead representation.

---

## How analyze.sh decides "domain-leak" (placeholder)

v1 does NOT detect domain-leak automatically — it's listed in the JSON
output for symmetry but always returns 0. To detect it properly you need
a static analysis pass that:
1. Reads the architecture-map §1 to know each module's domain.
2. Reads §3 to know which entity belongs to which domain.
3. Parses imports / requires per file.
4. Flags any import where consumer-domain ≠ producer-domain AND the imported
   symbol is a core-domain entity (not a projection).

A future v1.1 could add this via `ts-morph` / `tree-sitter`. For now, the
playbook above tells the agent how to spot it manually.

---

## Re-verification checklist (every 3-6 months)

1. Has the team converged on `tsc` strict mode, removing the need for
   runtime-validation contracts?
2. Has the architecture map's §7 edges grown beyond table readability?
   Consider migrating to a graph file (`.s5d/discovery/graph.json` already
   exists from `s5d discover sync`).
3. Has Prisma 7+ introduced a native projection type system that obviates
   manual contract-writing?
4. Does the team have a static-analysis tool wired up (ts-morph,
   dependency-cruiser) that can replace the bash-grep heuristics in
   `analyze.sh`?
