---
id: "A.6.9"
title: "U.CrossContextSamenessDisambiguation - Repairing cross-context “same / equivalent / align” via explicit Bridges (RPR‑XCTX)"
kind: "pattern"
part: "A"
status: "Stable"
source:
  path: "source/FPF-Spec.md"
  sha256: "cb4666c3c2f5482e59d6d79f39e48e2a6706d40e243c6617a348ba4a0823c371"
  start_line: 16053
  end_line: 16474
relations:
  builds_on:
    - "A.6.P"
    - "F.9"
    - "E.10.D1"
    - "A.7"
    - "F.0.1:2.3"
    - "E.10.U9"
    - "F.7"
    - "F.8"
    - "F.5"
    - "E.19"
  coordinates_with:
    - "E.17"
    - "C.3.3"
    - "A.6.6"
    - "F.7"
    - "F.8"
    - "E.10"
    - "B.3"
    - "F.5"
  specializes:
    - "A.6.P"
  uses:
    - "F.9"
---

## A.6.9 - U.CrossContextSamenessDisambiguation - Repairing cross-context “same / equivalent / align” via explicit Bridges (RPR‑XCTX)

> **Type:** Architectural (A) — A.6.P specialisation (RPR)
> **Status:** Stable
> **Normativity:** Normative
> **Placement:** A.6 cluster; immediately after A.6.8
> **Builds on:** A.6.P (RPR); F.0.1:2.3 (Explicit Bridge Principle); E.10.D1 (Context discipline); E.10.U9 (Alignment/Bridge lexical discipline); F.9 (Bridge discipline + reasoning primitives); F.7/F.8 (Concept‑Set rows & weakest‑link); F.5 (labels); A.7 (Strict Distinction: lanes + stance hygiene); E.19 (normative precision)
> **Coordinates with:** E.17 (Viewpoints / Views / Correspondences, when the prose is really about views/projections); C.3.3 (KindBridge, when the claim is about kind/classification transfer); A.6.6 (Identification/indexing, when the umbrella is really about IDs); Concept‑Set row scope rules; E.10 lexical SD (umbrella tokens); B.3 penalty conversion (if used)
> **Delta‑Class:** Δ‑3 (new normative pattern; additive; does not change existing kernel semantics)
> **Impact radius:** any document, table row, or boundary statement that asserts cross‑context sameness/compatibility/alignment between SenseCells, or collapses **A.7 lanes** / `CHR:ReferencePlane`s under umbrella “same/equivalent/…” wording
> **Mint vs reuse:** reuses `Bridge`, `BridgeKind`, `dir`, `CL`, `Loss`, `scope`; adds A.6.9‑specific Bridge‑Card qualifiers (`Γ_time`, `facetSpan`) (annotation slots; do not alter the kernel Bridge predicate); does not mint new kernel relations
> **Rationale witness:** required (in decision/publication lanes) for (i) declaring any Bridge with `scope` stronger than **Naming‑only**, and (ii) any strengthening edit (`scope` upgrade or `CL` increase). Provide the rationale as `witnessRefs` (review note, evaluation suite, decision log excerpt, etc.) and, where your process uses it, link the change to a DRR entry.

### A.6.9:1 - Problem frame

Cross‑Context prose routinely uses umbrella predicates (“same”, “equivalent”, “align”, “map”, “matches”, “corresponds”) to compress a multi‑dimensional claim into a single adjective.

In FPF terms, this is almost never a single claim. It is a *Bridge situation* that typically contains, at minimum:

* two (or more) **Contexts** (`U.BoundedContext`; each with its own idiom);
* a potentially hidden **direction** (A→B is not B→A);
* a hidden **degree of fit** (≈ vs ⊑/⊒ vs ⋂ vs ⊥, or interpretation‑only);
* near‑inevitable **loss/distortion** on transfer;
* a (usually implicit) **edition / time‑slice basis** for both endpoints and the correspondence judgement (`Γ_time`);
* a usually implicit **facet span** (`facetSpan`; “which aspects are being aligned?”) — the correspondence is often a *partial lens*, not whole‑cell sameness;
* a critical ambiguity between **lexical synonymy / translation** (“same word/label”), **referential co‑denotation** (“same referent under different IDs”), and **value‑level normalization** (“equivalent after φ‑normalization / unit conversion”).
* a critical ambiguity between **explaining** a correspondence and **licensing substitution**.

A.6.9 is the RPR specialisation that makes this structure explicit and prevents accidental “global identity” claims when the author’s intent is merely naming convenience or interpretive help.

### A.6.9:2 - Problem

When an umbrella predicate is used as if it were a single relation, readers (and downstream editors) silently choose defaults:

* **Symmetry hallucination:** “equivalent” is read as symmetric even when the intended relation is ⊑/⊒ (directional).
* **Scope creep:** “align/map” is read as substitution‑eligible, leaking into Role Assignment & Enactment or Concept‑Set row scopes.
* **Loss erasure:** “same” implies lossless transfer even when units, granularity, preconditions, or stance differ.
* **License confusion:** “explain X using Y” is mistaken for “Y can stand in for X”.
* **Implicit inversion:** later prose uses the inverse direction without an explicit redeclaration, breaking the “no silent inversion” rule.

The result is not merely imprecise wording: it changes what inferences are considered safe, and it pollutes Concept‑Set row scopes via unnoticed weakest‑link violations.

It also breaks **temporal coherence**: if the underlying canons (glossaries, schemas, code lists, ontologies) evolve, an un‑pinned “equivalent” claim silently becomes a claim about *two different editions at once*.

### A.6.9:3 - Forces

| Force                      | Pull                                            | Push                                                                      |
| -------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------- |
| Brevity                    | One word (“same”) is fast.                      | Fast words hide multi‑slot claims and create accidental licences.         |
| Practical interoperability | Teams want “one label” across artefacts.        | Shared labels are not structural sameness; they require scope discipline. |
| Direction sensitivity      | Many correspondences are one‑way.               | Natural language defaults to symmetry (“equivalent”).                     |
| Partial overlap is common  | Real systems rarely coincide perfectly.         | “Same” collapses overlap vs inclusion vs disjointness.                    |
| Evidence evolves           | Fit changes as counter‑examples are discovered. | Without change classes, people “re‑align” without recording what changed. |
| Version drift              | Canons/models are versioned and revised.        | Without `Γ_time` pinning, “equivalent” becomes temporally incoherent.     |
| Safety of reuse            | Substitution can reduce work.                   | Substitution without explicit `CL`/Loss is a latent defect.               |

### A.6.9:4 - Solution

Treat every cross‑Context umbrella‑sameness statement as an **RPR trigger** that must be rewritten into an explicit **Bridge claim** (F.9) with declared attributes.

This specialisation follows the A.6.P RPR envelope: it (i) defines a **trigger rule**, (ii) fixes the **stable lens** (Bridge Card), (iii) fixes a **minimal contract skeleton**, (iv) provides a **disambiguation guide**, and (v) standardises **change narration** for this class of ambiguity.

#### A.6.9:4.0 - Trigger rule (normative)

An occurrence SHALL be treated as an A.6.9 trigger when **either** (i) `CtxA ≠ CtxB`, **or** (ii) the statement collapses **A.7 lanes** (`Object | Description | Carrier`) or `CHR:ReferencePlane`s under an umbrella sameness predicate, and the prose (or a table row comment) uses any of the following as if they were a single relation:

* **Umbrella predicates**: “same”, “identical”, “equivalent”, “align”, “map”, “match”, “correspond(s)”, and close variants.
* **Reuse‑intent shorthands** that often smuggle licences: “treat as”, “reuse”, “share”, “unify”, “canonical”, “single source of truth”, “synced”, “normalized”, “one‑to‑one”, “same ID”, “mirrors”.
* **Endpoint umbrellas** in the presence of a cross‑context sameness claim (e.g., “the system/service/model/table/class”) — this is simultaneously an endpoint‑identity problem and a Bridge problem.

**ID/reference caveat.** Tokens like “same ID”, “same key”, “one‑to‑one”, “synced”, or “mirrors” often denote an **identification/indexing** claim or an **operational mapping artefact** rather than a sense‑level correspondence. If an ID claim is being used as a proxy for meaning (“same ID ⇒ same thing/role”), split it into (i) an explicit identification/indexing claim (A.6.6) and (ii) any Bridge claim about meaning (this pattern). Keep code/ETL facts as `witnessRefs`; they do not determine `kind/CL/Loss/scope` by themselves.

**Multilingual caveat.** In non-English prose, treat local-language equivalents
of umbrella tokens such as "equivalent", "corresponds", or "this is the same
thing" as the same trigger class.

**Lane/plane‑only caveat.** If `CtxA = CtxB` and the trigger is solely a lane/plane collapse, repair lane/plane typing first (A.7 / declared `Φ_plane`). You MAY satisfy this pattern by re‑typing endpoints + adding an explicit non‑licensing marker; do not invent a Bridge unless you actually need an auditable cross‑Context licence record.

When triggered, the author SHALL do exactly one of:

1. **Rewrite into an explicit Bridge** (BridgeId or inline Bridge Card) with the required slots (`kind/dir/CL/Loss/scope` at minimum), or
2. **Rewrite into an Explanation‑only form**: either declare an **Explanation‑only Bridge** (`scope=Explanation‑only`) or keep the statement as Plain explanatory prose with an explicit **non‑licensing marker** (“no Bridge licence; do not substitute; do not justify rows”). In either form, it MUST NOT be used to justify Concept‑Set rows, cross‑Context reuse, or substitution.

The repair has three moves:

**Terminology discipline (Tech register).**
* In this spec, **Context** means `U.BoundedContext` (E.10.D1 / D.CTX).
* Use **lane** for the A.7 split (**Object | Description | Carrier**).
* **CHR:ReferencePlane** is reserved for world/concept/episteme crossings; do **not** use it as a synonym for lane.

0. **Resolve endpoints as SenseCells (and pin editions where relevant).** If the surface text uses pronominal/metonymic bundles (“the system”, “the model”, “it”, “this class”, “that table”, “the service”), treat this as an endpoint‑identity problem first: enumerate candidates and select the intended `σ@Ctx` endpoints (Candidate‑Set Note, A.6.P:4.0b). Also check **lane** and **stance/time tags**: ensure each candidate sits on the intended A.7 lane (**Object | Description | Carrier**) and record any time‑stance tags on the relevant artefacts/sources (e.g., `DesignRunTag = design | run`) that affect substitution safety. Do not treat `DesignRunTag` as a separate Context; it is a time tag on artefacts/sources. If the only crossing is design↔run, route via an Interpretation Bridge (`kind=⇄ᴅʀ`, `scope=Explanation‑only`) unless you have a separately justified substitution Bridge within a fixed lane. If the triggering token is an identifier/key/code, repair it as a Carrier‑lane identification/indexing claim first (A.6.6), and only then decide whether there is also a sense‑level Bridge claim. If the ambiguity is actually a **CHR:ReferencePlane** mix (e.g., “a database column” vs “a real‑world attribute”), treat that as a ReferencePlane issue: restate endpoints on a single `CHR:ReferencePlane`, or route the crossing through a declared `Φ_plane` policy before attempting any substitution licence. In decision/publication lanes, endpoint ambiguity is fail‑closed: if the intended endpoints cannot be resolved from local cues and `witnessRefs`, keep the sentence as Plain explanatory prose (or an Explanation‑only Bridge) and do not use it to justify cross‑Context reuse, Concept‑Set rows, or substitution.
   * **Modularity note:** if the endpoint token itself is a known umbrella term (e.g., “service”), apply the relevant endpoint‑disambiguation RPR first (e.g., A.6.8 for “service”), then return here for the cross‑context sameness predicate.
   * **View/projection note:** if the prose is primarily about **views/projections/correspondences** rather than sameness licences, coordinate with E.17 (multi‑view describing). You may still need a Bridge for naming/substitution licences, but do not let “is a view of” silently become “is the same as”.
   * **Edition / canon pinning (Γ_time).** If either endpoint’s meaning is fixed by a versioned canon (glossary, schema, code list, ontology, model release), record the specific editions (or “as‑of” date) used to make the correspondence judgement, and carry that as `Γ_time` on the Bridge Card. If you cannot state `Γ_time` in decision/publication lanes, fail‑closed: keep the prose Explanation‑only and do not justify rows or substitution.
   * **Ontology category sanity (Kinds vs instances vs values).** Before declaring `kind/dir/CL/scope`, check that the endpoints live at compatible ontological strata (e.g., *Kind/classification* vs *instance* vs *measurement value*). If the “equivalence” is really a kind/classification transfer, coordinate with **C.3.3 KindBridge**; if it is a value‑normalization claim, treat it as a Measurement‑family bridge and make the normalization channel explicit in `Loss` (and/or `witnessRefs`).

1. **Replace the umbrella predicate with a Bridge reference** (or an inline Bridge Card).
2. **Choose the Bridge’s kind, direction, licence scope, `CL`, and Loss notes explicitly**, instead of letting readers infer them.
3. **Separate “interpretation” from “licence”** by using the Bridge scope rules: Explanation‑only vs Naming‑only vs Substitution‑eligible.

This is a pattern specialisation of A.6.P: it provides the stable lens, contract skeleton, change‑class lexicon, and a disambiguation guide tailored to cross‑Context “sameness”.

#### A.6.9:4.1 - Stable lens

**Stable lens (QRR):** the **Bridge Card** (F.9) used as a qualified relation record for cross‑Context sameness claims.

A conforming cross‑Context claim is expressed as a Bridge declaration:

```
⊢ Bridge(σA@CtxA, σB@CtxB) : ⟨senseFamily, kind, dir, CL, Loss, scope⟩
```

**A.6.9 qualifiers (pattern‑level; Bridge‑Card annotations).** A.6.9 additionally requires:
* `Γ_time` — edition/as‑of basis for the correspondence judgement (MUST in decision/publication lanes),
* `facetSpan` — the facet‑preservation span when the correspondence is not whole‑cell.
These live on the Bridge Card as qualifiers; they do **not** change the kernel Bridge predicate signature.

This record is a **conceptual judgement and licensed‑use record** (a thought‑format), not an ETL pipeline, API guarantee, or a “mapping implementation”. Operational mapping artefacts (aligner models, lookup tables, transformation code) belong in `witnessRefs` and do not erase `Loss` or relax `scope` by themselves.

**Non‑inheritance note.** A Bridge relates two local senses; it does **not** make `CtxA` a sub‑Context of `CtxB` (or vice versa), and it does not create “global identity” between Contexts.

**Kernel restraint reminder.** Bridges translate between local senses; they do **not** justify minting a new global `U.Type` by “sameness”. If the desired outcome is a new shared type/kind, route to the type‑minting discipline (A.11) and keep Bridges as translators.

**Direction note (avoid a common misread).** `dir = A↔B` expresses *symmetry of the correspondence* (e.g., for `kind∈{≈,⋂,⊥}` or for `kind=⇄ᴅʀ`), not “two substitution licences for free”. **Role Assignment & Enactment substitution is always directional** and must be stated as such (A→B). `scope=Type‑structure` is structural reuse, not substitution.

**Memory hook:** if the Bridge Card does not fit on one screen, you are describing the Contexts, not the Bridge.

#### A.6.9:4.2 - Explicit contract skeleton

A.6.9 fixes the minimal slot set that must be made explicit whenever a cross‑Context (or cross‑lane / cross‑plane) “same/equivalent/align/map/…” assertion appears.
| Slot                 |               Required | Meaning / constraints                                                                                                                  |
| -------------------- | ---------------------: | -------------------------------------------------------------------------------------------------------------------------------------- |
| `BridgeId`           |          Yes (if cited) | Required whenever the Bridge is referenced from multiple places, used to justify row scope, or used as a licence in decision/publication lanes. Inline cards MAY omit an id for a single‑use didactic gloss. **When present, the id is a registry reference** (per the F.9 registry‑reference note): check existence / edition pinning, not signature export. |
| `σA@CtxA`, `σB@CtxB` |                    Yes | Endpoints are **SenseCells** (not strings, not “the systems”).                                                                         |
| `senseFamily`        |                    Yes | Use a named family (F.9). For substitution‑capable Bridges, this MUST be a single family (Role / Status / Measurement / Type‑structure / …). If the correspondence crosses families, use an **Interpretation** kind (`⇄ᴅʀ / →ᴍᴇᵃ / →ᴅᵉᵒ`) and record the channel explicitly (e.g., `Method ⇄ᴅʀ Execution`, `Measurement →ᴍᴇᵃ Requirement/Clause`, `Deontic →ᴅᵉᵒ Execution`), keeping `scope=Explanation‑only`. |
| `kind`               |                    Yes | One of the F.9 kinds: `≈ / ⊑ / ⊒ / ⋂ / ⊥ / ⇄ᴅʀ / →ᴍᴇᵃ / →ᴅᵉᵒ`. Use `⊑/⊒` only for defensible inclusion. If you can name a counter‑case that violates inclusion for these endpoints, you do **not** have `⊑/⊒` — use `⋂` or refine endpoints (SenseCell split). |
| `dir`                |                    Yes | Always explicit (F.9). Use `A→B` for any **substitution** claim (Role Assignment & Enactment‑eligible), even when `kind=≈`. Use `A↔B` only to express a symmetric correspondence (or Type‑structure reuse); it does **not** imply bidirectional substitution. **No implicit inversion.** **Inclusion sanity:** when `kind∈{⊑,⊒}`, ensure `dir` matches the intended safe reading (substitution, when allowed, goes **from narrower to broader**); if needed, swap endpoints or declare the inverse Bridge explicitly rather than relying on prose. |
| `Γ_time`             | Yes (in decision/publication lanes); otherwise Should | **Edition / time‑slice basis** for the Bridge judgement. Pin (or reference) the editions of the canons that fix the endpoints’ meanings (glossary/schema/code list/ontology/model release), or state an “as‑of” date for both sides. If endpoint notation already pins editions unambiguously, you MAY set `Γ_time = =endpointPins`. If the correspondence is intentionally *rolling*, say so explicitly and attach an update policy + witnesses; rolling claims MUST NOT justify substitution unless a specific edition pair is pinned for the decision being justified. |
| `CL`                 |                    Yes | Integer `0–3` with label (`0 Opposed`, `1 Comparable`, `2 Translatable`, `3 Near‑identity`) and a one‑line “why”. For `CL=3`, the “why” MUST cite matched invariants (see below). |
| `Loss`               |                    Yes | **Non‑empty Loss Notes** stating what fails to carry (units, scope, granularity, preconditions, stance). `Loss: none` is permitted **only** when `CL=3` and matched invariants are cited; for `kind=⊥`, use `Loss: n/a (incompatibility claim)` (F.9). |
| `facetSpan`          | Yes (if not whole‑cell); otherwise May | The **facet span** of the correspondence (what is being aligned / preserved): e.g., `{label}`, `{identifier semantics}`, `{membership}`, `{value after unit normalization}`, `{role qualifiers}`, `{status lattice}`. If the bridge is facet‑limited, either (a) refine endpoints into facet SenseCells (preferred), or (b) declare `facetSpan` explicitly and keep `scope` capped appropriately. |
| `counterExample`     |           Yes (if CL≤2) | The crispest case where the next‑stronger reading would mislead (substitution, row scope, or type reuse). For `CL=3`, state “no known counterexamples under invariants” (and cite the invariant set). |
| `invariants`         |           Yes (if CL=3) | A short list of the invariants that justify `CL=3` (domain + measurement + stance constraints as applicable), with pointers (`witnessRefs`) to where they are checked or argued. |
| `scope`              |                    Yes | Allowed use (F.9): `Explanation‑only / Naming‑only / Role Assignment & Enactment‑eligible / Type‑structure`. This is a **maximum licence** for how the Bridge may be used in reasoning and tables. Do not confuse it with **Claim scope (G)** from USM (A.2.6), and do not encode “sometimes substitution” by mixing scopes—narrow endpoints instead (see below). |
| `witnessRefs`        | Should (MUST in decision/publication lanes for any Bridge used beyond Explanation‑only) | Evidence artefacts / witness set (rules, tests, audits, empirical evaluations, review notes, alignment reports). `witnessRefs` are how readers distinguish “declared” from “demonstrated”. |
| `didacticHook`       |                    May | A single sentence that teaches the safe reading.                                                                                       |

**Hard separation:** “shared label” is `Naming‑only`; “can replace in decisions/enactment” is `Role Assignment & Enactment‑eligible` and requires the substitution conditions; “can be treated as the same class/type for structural inference” is `Type‑structure` and requires near‑identity under invariants.

**Two “scopes” warning.** `scope` is a **licence scope** (how the Bridge may be used). The *facet span* of the correspondence (“which aspects are aligned?”) MUST be carried either by endpoint refinement (preferred) or by an explicit `span` + consistent `Loss`. Do not overload `scope` to mean facet span.
**Naming note.** Use `facetSpan` for facet limitation to avoid confusion with other “span” operators/vocabulary elsewhere in the spec.

**Kind/scope admissibility (concept‑level; non‑deontic).**

The following constraints are stated as *admissibility conditions* (E.19): they define when a Bridge Card is well‑formed for a claimed licence.

* **INV‑XCTX‑KS‑0 (Kind/CL sanity).** If `kind=⊥`, then `CL=0`. If `CL=3`, then `kind=≈` and `invariants` are stated.
* **INV‑XCTX‑KS‑1 (Overlap caps scope).** If `kind=⋂`, then `scope ∈ {Explanation‑only, Naming‑only}`.
* **INV‑XCTX‑KS‑2 (Disjoint embargo).** If `kind=⊥`, then `scope = Explanation‑only`, and the Bridge cannot support Concept‑Set rows or substitution (F.9:13.4).
* **INV‑XCTX‑KS‑3 (Interpretation embargo).** If `kind∈{⇄ᴅʀ, →ᴍᴇᵃ, →ᴅᵉᵒ}`, then `scope = Explanation‑only`, and the Bridge cannot support Concept‑Set rows or substitution (F.9:13.5).
* **INV‑XCTX‑KS‑4 (Role Assignment & Enactment substitution).** If `scope = Role Assignment & Enactment‑eligible`, then `kind∈{≈,⊑,⊒}`, `dir = A→B`, `CL≥2`, the Bridge is senseFamily‑preserving, endpoints are stance‑compatible, Loss notes are non‑empty, and a counter‑example is stated (F.9:13.2, F.9:13.8, F.9:16.1).
* **INV‑XCTX‑KS‑5 (Type‑structure reuse).** If `scope = Type‑structure`, then `senseFamily = Type‑structure`, `kind=≈`, `dir=A↔B`, `CL=3`, and matched invariants are stated (Type‑structure is only supported by near‑identity; see F.9:6.1 and F.9:16.1).
* **INV‑XCTX‑KS‑6 (Inclusion honesty).** `kind∈{⊑,⊒}` implies: the Bridge does not cite any membership counter‑case that violates inclusion for the stated endpoints. If such a counter‑case exists, then (for these endpoints) `kind=⋂`, or the endpoints are refined (SenseCell split) before any inclusion kind is stated.

**No “conditional scope” in one Bridge.** Authors SHALL NOT encode two licences in one Bridge (e.g., “Naming‑only generally; substitution in workflow X”). Instead, refine endpoints into the guarded subset SenseCells (SenseCell split) and declare a **separate** Bridge for the refined endpoints (new id or new edition), keeping the broad Bridge at the weaker scope.

#### A.6.9:4.3 - Change‑class lexicon

A.6.9 forbids “re‑align / re‑map / now equivalent” as a change description. Changes are narrated using the **A.6.P change classes**; the Bridge‑specific verbs below are narrative shorthands that map to A.6.P:4.4 (`declareRelation`, `withdrawRelation`, `retargetParticipant`, `reviseByValue`, `rescope`, `retime`, `refreshWitnesses`).
Authors SHALL NOT use umbrella verbs (“re‑align”, “re‑map”, “now equivalent”, …) as change narration. Narrate changes using the change‑class lexicon below (mapped to A.6.P:4.4).

1. `declareBridge(BridgeId, σA@CtxA, σB@CtxB, …slots…)`
2. `withdrawBridge(BridgeId)`
3. `retargetEndpoint(BridgeId, σA→σA', σB→σB')` (edition pinning or SenseCell split/merge)
4. `retime(BridgeId, Γ_time→Γ_time')` (maps to A.6.P `retime(newΓ_time)`; semantic; edition‑fenced in decision/publication lanes)
5. `changeBridgeKind(BridgeId, kind→kind')` (maps to A.6.P `changeRelationKind`)
6. `adjustCL(BridgeId, CL→CL')` (raise/lower, with at least one new invariant or counter‑example)
7. `rescope(BridgeId, scope→scope')` (Naming‑only → Role Assignment & Enactment‑eligible / Type‑structure is a strengthening; requires DRR and MUST be unconditional for the same endpoints)
8. `reviseLossNotes(BridgeId, Loss→Loss')`
9. `reviseFacetSpan(BridgeId, facetSpan→facetSpan')` (maps to A.6.P `reviseByValue`; semantic; edition‑fenced in decision/publication lanes)
10. `refreshWitnesses(BridgeId, witnessRefs→witnessRefs')` (adding one witness is a special case: set‑union + re‑publish)

**Edition fence (decision/publication lanes).** Any semantic edit to a Bridge’s slots (endpoints, kind, dir, CL, scope, invariants) SHALL be published as a **new Bridge edition** (with an explicit supersedes/withdraws note) rather than rewriting a prior edition in place. This preserves auditability and prevents “silent strengthening” through edits.

Semantic edits include changes to `Γ_time` or declared `facetSpan` (because they change what editions/aspects the correspondence judgement is about).

**Workflow/guard‑scoped strengthening is not a plain `rescope`.** If the stronger licence holds only after filtering/guards (e.g., “human users only”), represent that by **refining endpoints** (SenseCell split) and declaring a Bridge for the refined endpoints (new id or new edition), rather than upgrading the broad Bridge’s scope.

**Direction inversion is not an edit.** If the inverse relation is needed, declare a *new* Bridge (new `BridgeId`) with its own `dir`, `kind`, `CL`, and Loss; optionally withdraw the old one.

#### A.6.9:4.4 - Lexical guardrails and name selection

**Umbrella tokens (red‑flag triggers):** “same”, “identical”, “equivalent”, “align”, “map”, “match”, “correspond(s)”, and close variants.

These are only in‑scope here when used as **cross‑Context predicates** (`CtxA ≠ CtxB`) or when the prose collapses **A.7 lanes** / `CHR:ReferencePlane`s under an umbrella sameness predicate. For that case:
* In **Tech register** (normative / decision‑carrying prose), authors SHALL NOT use umbrella tokens as standalone cross‑Context predicates. Use a Bridge reference and a licence‑revealing verb instead (“share a label”, “substitutes for”, “explain in terms of”).
* In **Plain didactic** or quoted legacy prose, umbrella tokens MAY appear, but only if the paragraph also includes an explicit Bridge reference (BridgeId or inline Bridge Card) so readers are not forced to infer `kind/dir/CL/Loss/scope`.

Instead, choose a phrase that reveals the intended licence:

| Intended meaning                | Use this (canonical)                                                               | Avoid                                             |
| ------------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------- |
| Interpretation only             | “Explain σB in terms of σA under an *Interpretation Bridge* (kind∈{⇄ᴅʀ,→ᴍᴇᵃ,→ᴅᵉᵒ}, scope=Explanation‑only).” | “σA is the same as σB.” |
| Naming convenience              | “Share a label under a *Naming‑only* Bridge (scope=Naming‑only; kind∈{⋂,⊑,⊒} (and **≈ only when you state why substitution is still forbidden); CL≥1; Loss + counterexample).” | “σA corresponds to σB (so we can treat them as…)” |
| Safe substitution (directional) | “Licence substitution A↠B under a *Substitution Bridge* (kind∈{≈,⊑,⊒}, dir A→B, CL≥2, same senseFamily + stance; Loss + counterexample; scope=Role Assignment & Enactment‑eligible).” | “σA and σB are equivalent.” |
| Type‑structure reuse (strong)   | “Declare a *Type‑structure* Bridge (senseFamily=Type‑structure; kind=≈; dir A↔B; CL=3; invariants; scope=Type‑structure).” | “They are literally the same class everywhere.” |
| Disjoint / contrast             | “Declare kind=⊥ with scope=Explanation‑only (contrast only).”                       | “Almost the same” / “basically equivalent”        |

**Name selection rule:** if the author wants “the same name”, choose *Naming‑only* and keep the verb “share a label”; if the author wants “can be substituted”, use *Substitution* and keep the verb “substitutes for” with explicit direction.

#### A.6.9:4.5 - RPR Disambiguation Guide (XCTX)

Use this table when you encounter umbrella‑sameness wording.

| Trigger in text                    | Candidate Bridges (default first)                                                                 | Discriminating questions / tests                                                                 | Canonical rewrite                                                                 | Routing hooks                                              |
| ---------------------------------- | -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| “A is the same as B” (CtxA ≠ CtxB) | Explanation‑only (interpretation) → Naming‑only (⋂/⊑/⊒/≈) → Substitution (≈/⊑/⊒, CL≥2)            | Is this a licence or a teaching gloss? What direction is safe? What is lost? What is the counter‑example? | `Bridge(σA@CtxA, σB@CtxB): ⟨kind=?, dir=?, CL=?, Loss=?, scope=?⟩`                | E (witness), D (naming), A (admissibility if substitution) |
| “Align A and B”                    | Naming‑only with overlap (⋂)                                                                        | Do we only need a shared label, or do we need safe substitution/type reuse?                       | `Bridge(σA,σB): kind=⋂, dir=A↔B, CL=1, Loss + counterExample, scope=Naming‑only`   | D (labeling), E (counterexample)                           |
| “Map A to B”                       | (i) semantic Bridge (this pattern) vs (ii) operational artefact (ETL/transform/lookup)             | Is “map” about a thinking move (licence) or about code/execution? What is the substitution direction (if any) vs code direction? | `Bridge(σA,σB): dir A→B, kind chosen for that direction, Loss bullets + counterExample` | E (artefact), A (if substitution proposed)                 |
| “Same ID / same key / 1‑to‑1”      | Identification/indexing claim (A.6.6) ± semantic Bridge                                              | Is the claim about **Carrier‑lane equality** (identifier scheme), or about **sense/meaning**? What is the reference scheme? Are collisions/aliases possible? | First: repair as an identification/indexing relation (A.6.6). Then (only if needed): declare a Bridge for meaning with explicit `kind/dir/CL/Loss/scope`. | A.6.6 (Carrier), E (reference scheme), A.6.9 (meaning)     |
| “B is a view/projection of A”      | Explanation‑only or Naming‑only by default; substitution only after explicit guards/refined endpoints | Is this a view/correspondence statement (E.17), or a reuse licence? Does projection drop constraints/fields/stance? | `Bridge(σA,σB): kind=⊑ (if A is narrower), dir A→B (if substitution is intended), Loss states dropped structure/constraints, scope capped unless proven` | E.17 (views), E (artefact), A (if substitution proposed)   |
| “A matches B” / “corresponds to”   | Naming‑only overlap (⋂)                                                                             | Is it overlap (⋂) or inclusion (⊑/⊒)? What breaks under substitution?                              | `kind=⋂, scope=Naming‑only, CL=1 (or CL=2 if translatable), Loss + counterExample` | D, E                                                       |
| “Equivalent”                       | ≈ only under explicit invariants; otherwise overlap/inclusion                                       | Equivalent in what **senseFamily** and under what invariants? Any counter‑examples?               | Prefer `⋂ + Naming‑only`, or specify `≈` with invariants & CL                       | L (invariant claim), E                                     |

Updates:

* For “Align A and B”, default to `kind=⋂`, `scope=Naming‑only`, `dir=A↔B`, `CL=1`, with explicit Loss + counterexample. Use `kind=≈` only when you can state the equivalence criterion; invariants are mandatory for `CL=3` (and recommended whenever you use `≈`). Use `scope=Type‑structure` only when `kind=≈` and `CL=3` with matched invariants (INV‑XCTX‑KS‑5).
* For “Map A to B”, first decide whether “map” denotes (i) a semantic Bridge claim (this pattern) or (ii) an operational transformation artefact (ETL, id translation, schema mapping). If (ii), keep the artefact as `witnessRefs` and still declare the Bridge kind/dir/Loss separately; do not let “there exists a map” collapse into substitution.

**Default safety rule (normative):** authors SHALL NOT assign `CL≥1` (nor claim Naming‑only or substitution) unless they can state `Loss` notes and (for `CL≤2`) a `counterExample`. Otherwise, keep the statement as Explanation‑only (didactic gloss) or postpone the cross‑Context claim until evidence exists.
If the stable intent is **anti‑conflation** (“do not treat them as the same”), make that explicit as `kind=⊥` with `scope=Explanation‑only` (contrast), or—when the contrast is stable and repeatedly needed—publish a contrast row per the Concept‑Set discipline instead of relying on “not the same” prose.

When endpoint meanings are versioned, the same rule applies to `Γ_time`: if you cannot state the edition/as‑of basis, keep the claim Explanation‑only and do not justify rows or substitution.

#### A.6.9:4.6 - Mapping artefacts are not Bridges (normative clarification)

Many projects use “map” to mean an implementation artefact: a lookup table, aligner model, transformation function, or ETL step. A.6.9 treats such artefacts as **witnesses**, not as semantics. The Bridge is where you record:

* what correspondence is claimed (`kind/dir/senseFamily`);
* how strong it is (`CL`, invariants for `CL=3`);
* what breaks (`Loss`, counterexample);
* what it licenses (`scope`).

**Direction reminder.** A transformation artefact may be written `f:A→B` while the safe semantic substitution (if any) is `B↠A` (or none at all). Treat `dir` as the direction of the licensed **reading/substitution move**, not the direction of code execution.

If the artefact changes, narrate the update as `refreshWitness` / `reviseLossNotes` / `adjustCL` (editioned), not as “re‑mapped”.

#### A.6.9:4.7 - Coordination notes (keep A.6.9 modular)

* **Views / projections / correspondences:** if the core intent is multi‑view description (“this diagram is a view of that system”, “these views correspond”), route the modelling discipline to **E.17** and keep A.6.9 focused on preventing umbrella‑token licence smuggling. A.6.9 may still be used to declare any *naming/substitution* licence between view elements, but it MUST NOT replace E.17’s correspondence discipline.
* **Kinds / classifications:** if the cross‑context claim is about **kind transfer** (“Class X in A is the same kind as Class Y in B” as a classification move), consider recording the classification channel using **C.3.3 KindBridge**. Do not conflate Bridge‑CL with kind‑mapping CL^k.


### A.6.9:5 - Archetypal Grounding

#### A.6.9:5.1 - System archetype: identity “sameness” across products

**Tell (ambiguous):**
“An IAM *User* is the same as a CRM *Customer*.”

**Show A (Bridge Card repair):**

```
BridgeId: β-IAM→CRM-UserCustomer (edition-pinned)
Cells: “User”@IAM ↔ “Customer”@CRM
senseFamily: Role
kind: ⋂
dir: IAM↔CRM
CL: 2 (Translatable) — high overlap; service accounts and leads/prospects are counterexamples
Loss:
  - CRM “Customer” includes leads/prospects with no IAM account
  - IAM “User” includes service accounts and disabled identities not treated as customers
Counter-example: “svc-billing@” is a User@IAM but not a Customer@CRM
scope: Naming-only
Didactic hook: “Overlap only: share labels; do not substitute without guards/refinement.”
```

**Effect:** dashboards and prose may share labels (Naming‑only). Workflow substitution is *not* implied globally; it is gated by scope and guards.

**Show B (change narration, later evidence):**
After hard constraints are added (e.g., “human‑verified email”, “not a service account”), a team wants stronger reuse in the ticketing integration.

*Do not write:* “Now they are equivalent / now the mapping is fixed.”
*Write:*

0. Keep the broad Bridge **as‑is** (Naming‑only, overlap): it remains the correct “shared label” relation for the unguarded endpoints.
1. `refreshWitnesses(β-IAM→CRM-UserCustomer, witnessRefs→witnessRefs ∪ {TicketingIntegrationTestSuite_v3})`
2. `declareBridge(β-IAM→CRM-HumanVerifiedUser→VerifiedCustomer, HumanVerifiedUser@IAM, VerifiedCustomer@CRM, …slots…)` (new Bridge id or new edition family)
3. In that new Bridge: state `kind=⊑` (if inclusion is now true for the refined endpoints), `dir=IAM→CRM`, keep `CL=2`, restate Loss (remaining exclusions), and provide a crisp counter‑example for where substitution would still break.
4. `rescope(β-IAM→CRM-HumanVerifiedUser→VerifiedCustomer, Naming‑only → Role Assignment & Enactment‑eligible)` with DRR explaining why `CL=2` suffices for the refined endpoints.

Direction remains IAM→CRM; if the inverse is required, declare a separate Bridge with its own loss/counterexamples.

#### A.6.9:5.2 - Episteme archetype: schema/ontology alignment between knowledge graphs (class-level)

**Tell (ambiguous):**
“`Person` in KG‑A is equivalent to `Person` in KG‑B.”

**Show A (Bridge Card repair):**

```
BridgeId: β-KGA↔KGB-Person (edition-pinned)
Cells: Person@KG-A ↔ Person@KG-B
senseFamily: Type-structure
kind: ⋂
dir: A↔B
CL: 2 (Translatable) — overlap is high but invariants differ
Loss:
  - KG-A “Person” includes fictional characters; KG-B excludes them
  - KG-B requires a stable external identifier; KG-A does not
Counter-example: “Sherlock Holmes” ∈ Person@KG-A but ∉ Person@KG-B
scope: Naming-only
Didactic hook: “Shared label does not grant type-structure or substitution; the sets only overlap until invariants and membership rules are aligned.”
```

**Show B (strengthening attempt and rejection):**
A reviewer proposes Type‑structure reuse (“treat them as the same class across graphs”). Under A.6.9, this triggers a required invariant check:

* Since Type‑structure reuse requires CL=3 and matched invariants, the proposal is rejected unless the invariants are aligned and the counterexample class is eliminated (e.g., by refining `Person@KG-A` into `FictionalPerson` vs `RealPerson`).
* The correct change narrative is: `changeBridgeKind` (if kind changes), `adjustCL` only if the counterexample disappears and invariants are shown, else keep CL=2 and Naming‑only scope.

### A.6.9:6 - Bias‑Annotation

This pattern is biased toward:

* **Explicitness over fluency.** It intentionally slows down prose that would otherwise smuggle licences.
* **Safety in substitution.** It treats substitution as a high‑risk claim requiring declared direction, `CL`, and Loss notes.
* **Locality of meaning.** It assumes meanings are Context‑local unless bridged explicitly; it rejects label‑driven identity.
* **Ordinal confidence.** `CL` is treated as an ordinal safety ladder, not a probability; it is deliberately coarse.

Consequently, A.6.9 may feel “heavy” in early drafts, but it prevents latent cross‑Context defects that are costly to discover later.

### A.6.9:7 - Conformance Checklist

A document or boundary statement conforms to A.6.9 iff:

* **CC‑A.6.9‑0 (UTS/LEX trigger coverage).** The local lexicon treats umbrella‑sameness tokens as RPR triggers and points authors to Bridge‑explicit rewrites.
* **CC‑A.6.9‑1 (No standalone umbrella predicate).** Cross‑Context umbrella tokens SHALL NOT be used as standalone cross‑Context predicates unless either:
  * (a) the paragraph includes an explicit Bridge reference (BridgeId or inline Bridge Card), or
  * (b) the statement is explicitly marked as non‑licensing explanatory prose (“no Bridge licence; do not substitute; do not justify rows”).
* **CC‑A.6.9‑2 (SenseCell endpoints).** Every such claim names endpoints as `σ@Context` (edition‑pinned where relevant), not as strings or system names.
* **CC‑A.6.9‑3 (Direction explicitness).** `dir` is stated on every Bridge. If `kind` is non‑symmetric, any inverse use without redeclaration is non‑conformant.
* **CC‑A.6.9‑4 (Licence separation).** If the intent is explanation only, authors SHALL either (a) declare `scope = Explanation‑only` on a Bridge, or (b) use explicit non‑licensing prose (no Bridge licence). If the intent is naming compatibility, authors SHALL declare a Bridge with `scope = Naming‑only`. In all cases, the text SHALL NOT invite substitution unless a substitution‑eligible Bridge exists.
* **CC‑A.6.9‑5 (Substitution thresholds).** Any statement that implies substitution MUST be backed by a substitution‑eligible Bridge (`kind∈{≈,⊑,⊒}`, `CL≥2`, same `senseFamily`, stance‑compatible), with Loss notes and a counter‑example discipline.
* **CC‑A.6.9‑6 (Weakest‑link respect).** Any Concept‑Set row or composed claim that depends on multiple Bridges MUST bound its scope and `CL` by the weakest participating Bridge.
* **CC‑A.6.9‑7 (Loss visibility).** Loss notes are present and **non‑empty**. `Loss: none` is permitted only for `CL=3` with cited invariants; `Loss: n/a` is permitted for `kind=⊥`. Loss must be consistent with the allowed scope.
* **CC‑A.6.9‑8 (Change narration).** Changes to cross‑Context fit are narrated using the change‑class lexicon (declare/withdraw/adjustCL/rescope/…) rather than umbrella verbs.
* **CC‑A.6.9‑9 (Kind/scope admissibility).** Any Bridge used to justify cross‑Context sameness satisfies the admissibility constraints INV‑XCTX‑KS‑1 … INV‑XCTX‑KS‑5 (no overlap‑to‑substitution; no disjoint/interpretation rows; substitution is directional; Type‑structure only under `≈` + `CL=3` + invariants).
* **CC‑A.6.9‑10 (Registry reference hygiene).** If a BridgeId (or policy/edition id) is cited, it is treated as a **registry reference** (existence / edition pinning), not as a semantic symbol exported by signatures.
* **CC‑A.6.9‑11 (Edition basis).** In decision/publication lanes, any Bridge used to justify Naming‑only / substitution / Type‑structure SHALL state `Γ_time` (edition pins or “as‑of” basis). If `Γ_time` cannot be stated, the claim MUST remain Explanation‑only and MUST NOT justify rows or substitution.
* **CC‑A.6.9‑12 (Facet honesty).** If the correspondence holds only on a subset of facets, the author SHALL either (a) refine endpoints into the facet SenseCells (preferred) or (b) declare `facetSpan` explicitly, with `Loss` consistent with that facet span. Whole‑cell Bridges MUST NOT be used to smuggle facet‑only correspondences.

### A.6.9:8 - Common Anti‑Patterns and How to Avoid Them

| ID            | Anti‑pattern           | Example                                              | Why it breaks                                           | Remedy                                                               |
| ------------- | ---------------------- | ---------------------------------------------------- | ------------------------------------------------------- | -------------------------------------------------------------------- |
| **AP‑XCTX‑1** | Bridge‑by‑adjective    | “A is the same as B (across contexts).”              | Smuggles scope + direction + loss as implicit defaults. | Replace with Bridge Card + explicit `scope`.                         |
| **AP‑XCTX‑3** | Stealth substitution   | “We’ll just treat A like B for now.”                 | Introduces implicit licence without CL/Loss gates.      | Publish Bridge Card; if CL<2, keep `Naming‑only`.                    |
| **AP‑XCTX‑2** | Symmetry hallucination | Treating `⊑/⊒` as symmetric “equivalence”.           | Causes unsafe inverse substitution.                     | Record `kind` and `dir`. Only symmetric kinds (`≈`, `⋂`, `⊥`, `⇄ᴅʀ`) may be written as `A↔B`; inclusion kinds require direction; substitution is always directional. |
| **AP‑XCTX‑4** | Lossless fantasy       | “Equivalent” with no loss note.                      | Loss is almost always present; hiding it misleads decisions.       | State Loss notes (even if “none”), add a counter‑example (CL≤2) or invariants (CL=3); adjust CL/scope accordingly. |
| **AP‑XCTX‑5** | Silent inversion       | Later prose uses B→A without redeclaration.          | Violates direction guard; breaks auditability.          | Declare inverse Bridge (new id) or withdraw+replace.                 |
| **AP‑XCTX‑6** | Confidence laundering  | Raising CL or scope without new invariants/evidence. | Inflates trust; expands row scopes illegitimately.      | Use `adjustCL`/`rescope` with witnessRefs + DRR.                     |
| **AP‑XCTX‑7** | Chain upgrade          | Treating A↠B and B↠C as “therefore A≈C”.             | Violates weakest‑link and loss accumulation.            | Use min‑CL and accumulated Loss; avoid chaining unless justified.    |
| **AP‑XCTX‑8** | Conditional scope smuggling | “Naming‑only generally; substitution in workflow X.” | Encodes two licences in one record; leaks into row scope and downstream reasoning. | Refine endpoints (SenseCell split) and declare a separate Bridge for the guarded subset; keep broad Bridge Naming‑only. |
| **AP‑XCTX‑9** | Artefact⇒equivalence fallacy | “There is a mapping table, so they are the same.” | Confuses operational transformation with semantic licence; hides Loss and direction. | Record artefact in `witnessRefs`, keep Bridge kind/dir/Loss explicit, and keep scope capped until CL+counterexamples justify promotion. |
| **AP‑XCTX‑10** | Two‑way substitution by symmetry | “The Bridge is A↔B, so we can substitute both ways.” | `A↔B` expresses correspondence symmetry, not two substitution licences; substitution is directional and must be stated (F.9:13.2). | Declare both substitution directions explicitly (two licences / two Bridges / two editions), each with Loss + counter‑examples. |
| **AP‑XCTX‑11** | Kind/dir mismatch | `kind=⊒` but `dir=A→B` is used as if it licensed substitution. | Inverts narrower/broader; encourages unsafe “narrowing substitution” and silent information loss. | Swap endpoints (so the intended safe direction is written as `A→B` with `kind=⊑`), or declare an explicit inverse Bridge; keep scope ≤ Naming‑only until the direction is justified. |
| **AP‑XCTX‑12** | Kernel promotion by Bridge | “Since A≈B, we can mint a unified global type and treat both as instances.” | Bridges translate local senses; they do not mint global meaning or new `U.Type`s. | If you need a new shared type/kind, follow A.11; keep Bridges as translators between Context-local senses. |
| **AP‑XCTX‑13** | Edition drift / timeless equivalence | “A is equivalent to B” with no edition/as‑of basis. | Makes the claim temporally incoherent as canons evolve; readers silently compare different revisions. | Pin editions via `Γ_time`; publish Bridge edits as new editions; fail‑closed to Explanation‑only when `Γ_time` cannot be stated. |
| **AP‑XCTX‑14** | Facet‑only alignment masquerading as whole‑cell sameness | “Customer corresponds to User” (but only `email` or an external ID aligns). | Collapses a partial lens into global sameness; invites unsafe substitution and row scope creep. | Refine endpoints to the facet SenseCells, or declare `facetSpan` explicitly and keep `scope` capped (usually Naming‑only). |
| **AP‑XCTX‑15** | Lexical translation ⇒ semantic identity | “Term A is the same as term B” (just a translation / synonym). | Confuses labels with referents; erases loss and context. | Use `scope=Naming‑only` with explicit `Loss` (incl. language/canon notes) and a counter‑example; do not imply substitution. |

### A.6.9:9 - Consequences

* **Pros**

  * Removes ambiguity between explanation, naming compatibility, and substitution.
  * Makes directionality explicit; prevents accidental inverse reasoning.
  * Forces Loss disclosure early; reduces later integration surprises.
  * Provides a disciplined evolution path (change classes) when evidence changes.

* **Cons**

  * Adds visible structure to prose; authors must choose `kind/dir/CL/scope` explicitly.
  * Requires reviewers to engage with counter‑examples and loss notes.
  * Can surface uncomfortable truth: many “same” claims are only Naming‑only.

**Adoption test (PRAG).** Take any cross‑Context sentence that uses an umbrella predicate (“same/equivalent/align/map/…”). If the team cannot (a) name the two SenseCell endpoints, (b) state `dir`, (c) write at least one Loss bullet, and (d) give a crisp counter‑example (for CL≤2), then the claim is not ready to be treated as Naming‑only or substitution‑eligible. Keep it as Explanation‑only (or explicit non‑licensing prose) until evidence exists.

If the endpoints’ canons are versioned and the team cannot state `Γ_time` (edition/as‑of basis), treat that as the same kind of “evidence missing”: keep the claim Explanation‑only.

### A.6.9:10 - Rationale

Cross‑Context “sameness” is a *family of relations*, not a single predicate. Making the Bridge explicit:

* preserves the locality of meaning (SenseCells are context‑bound);
* prevents licence creep (Naming‑only does not silently become substitution);
* supports auditability (BridgeId + slots, not adjectives);
* aligns prose with the formal reasoning primitives that govern safe substitution and row scopes.

A.6.9 turns a dangerous linguistic convenience into an explicit, reviewable, evolvable claim.

### A.6.9:11 - SoTA‑Echoing

(informative; post‑2015 alignment)

| SoTA practice                                                            | Primary source (post‑2015)                                              | What A.6.9 echoes                                                   | What A.6.9 adds                                                                                               | Stance                   |
| ------------------------------------------------------------------------ | ----------------------------------------------------------------------- | ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | ------------------------ |
| Correspondences between viewpoints in architecture descriptions          | ISO/IEC/IEEE 42010:2022                                                 | Correspondences are not identity; they have intent and constraints. | Forces direction/degree/loss to be explicit via Bridge Card slots.                                            | **Adopt + specialise**   |
| Declarative constraint systems and validation shapes                     | W3C SHACL (Recommendation, 2017)                                        | Make implicit semantics checkable by explicit structure.            | Uses Bridge Cards as “shape of correspondence”: explicit slots + counterexample discipline.                   | **Adapt**                |
| Entity alignment as scored correspondences with errors (embedding‑based) | BootEA (Sun et al., 2018) and related post‑2015 KG alignment literature | Alignment is graded, not binary; error analysis matters.            | Replaces raw scores with a coarse, auditable ordinal (`CL`) + explicit Loss notes and scope licences.         | **Adapt**                |
| Entity alignment using textual encoders (transformer‑based)              | BERT‑INT (Tang et al., IJCAI 2020); Ditto (Li et al., PVLDB 2021)        | Modern matchers output scored/conditional correspondences.          | Turns “score” into an auditable licence (`CL/scope`) plus explicit error modes (`Loss` + counterexamples).    | **Adopt (conceptually)** |
| Deep learning for schema matching as a family of match types             | SMAT (Zhang et al., 2021) and post‑2020 neural/LLM schema matching lines | “Matches” are heterogeneous and directional in practice.            | Makes match type explicit as Bridge kind + direction + licence scope (separating semantics from artefacts).   | **Adapt**                |
| Human‑in‑the‑loop entity matching (thresholding + error analysis)        | “Deep Learning for Entity Matching: A Design Space Exploration” (Mudgal et al., SIGMOD 2018) and follow‑on work | Scores are not licences; practice needs thresholds, abstention, and curated error cases. | Mirrors the “explain vs name vs substitute” split: scores stay in `witnessRefs`; promotion requires Loss + counter‑examples and an explicit scope upgrade. | **Adapt** |

### A.6.9:12 - Relations

* **Specialises:** A.6.P (Relational Prose Repair) by fixing the contract skeleton for cross‑Context sameness claims.
* **Uses:** F.9 Bridge discipline (Bridge Card, `BridgeKind`, `dir`, `CL`, Loss notes, scope licences, weakest‑link).
* **Coordinates with:** E.10 lexical discipline (umbrella tokens) and F.5 label discipline (Tech/Plain labels do not imply bridges).
* **Constrains:** Any cross‑Context Concept‑Set row scope claims via weakest‑link and substitution thresholds.

### A.6.9:End
