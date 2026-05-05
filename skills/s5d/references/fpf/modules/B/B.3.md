---
id: "B.3"
title: "Trust & Assurance Calculus (F–G–R with Congruence)"
kind: "pattern"
part: "B"
status: "Stable"
source:
  path: "source/FPF-Spec.md"
  sha256: "cb4666c3c2f5482e59d6d79f39e48e2a6706d40e243c6617a348ba4a0823c371"
  start_line: 29357
  end_line: 29767
relations:
  builds_on:
    - "A.10"
    - "B.1"
    - "B.1.1"
    - "B.1.2"
    - "B.1.3"
    - "B.1.4"
    - "A.12"
    - "A.14"
    - "A.15"
    - "C.13"
  coordinates_with:
    - "C.26"
    - "C.26.1"
    - "C.26.2"
    - "C.26.3"
    - "C.16"
    - "F.9"
    - "E.14"
    - "B.3.5"
  prerequisite_for:
    - "B.3.x"
    - "D.4"
  used_by:
    - "B.4"
---

## B.3 - Trust & Assurance Calculus (F–G–R with Congruence)

> **Plain‑English headline.**
> B.3 defines how **assurance** (trust) is **computed and propagated** for both physical systems and knowledge artifacts, using a small **typed assurance tuple** (**F–G–R**: **F/R** characteristics plus **G** as scope object) and **conservative aggregation rules** that respect the Γ‑invariants and A.15 **Strict Distinction**. It treats the **Working‑Model layer** as the **publication surface** for claims, with assurance **attached downward** (Mapping - Logical - Constructive - Empirical) per E.14.

### B.3:1 - Problem frame

Every non‑trivial result in FPF—*a composed system is safe*, *a model is credible*, *a conclusion holds*—is a **claim** that rests on **composed evidence**.

* For **U.System** holons (Γ\_sys), assurance is about *capabilities and constraints* under stated conditions.
* For **U.Episteme** holons (Γ\_epist), assurance is about the *quality of support* for a statement or model.

To make such claims comparable and auditable across domains, B.3 introduces a **Trust & Assurance Calculus** that:

* uses a **small typed assurance tuple** (**F–G–R**: **F/R** characteristics plus **G** as scope object) governed by conservative propagation rules (this is **not** a state space),
* accounts for **integration quality** via **Congruence Level (CL)** along the edges of a `DependencyGraph` (B.1.1, A.14),
* and composes these values with **Γ‑flavours** while respecting the **Invariant Quintet** (IDEM, COMM/LOC or their replacements, WLNK, MONO).

B.3 is **conceptual and normative**: it defines *which assurance components must be published and how they propagate*. How you improve those components (e.g., formalize, replicate, reconcile, or lawfully widen/narrow scope) is the job of KD‑CAL actions (the knowledge‑dynamics patterns; references are descriptive, not required to read here).

**Mechanism linkage.** For law‑governed operation families (e.g., **USM/UNM**) authored as **mechanisms**, use A.6.1 — U.Mechanism to publish **OperationAlgebra/LawSet/AdmissibilityConditions** and the **Transport** clause (Bridge‑only, CL/CL^k/CL^plane). All such penalties **reduce `R/R_eff` only**; **F/G** remain invariant.

**Working‑Model handshake (alignment with E.14 - B.3.5 - C.13).**
Assurance consumes two inputs declared at the **Working‑Model** surface (CT2R‑LOG, B.3.5): the **justification stance** `validationMode ∈ {postulate, inferential, axiomatic}` and, where present, the **grounding link** `tv:groundedBy`. Structural claims that aspire to the strongest guarantees rely on **Constructive** grounding as a **Γₘ** (Compose‑CAL) narrative referenced via `tv:groundedBy`. No assurance artefact **defines** Working‑Model wording or layout (downward‑only dependence, E.14).

### B.3:2 - Problem

Without a disciplined calculus, four chronic failures appear:

1. **Trust inflation:** Averaging or summing heterogeneous “quality” tags yields aggregates that look better than their weakest parts, violating WLNK.
2. **Scale confusion:** Mixing ordinal and ratio scales (e.g., averaging “formality levels” with numeric reliabilities) produces meaningless numbers.
3. **Congruence blindness:** Integration quality (how well pieces fit) is invisible; brilliantly strong parts connected by weak mappings produce overconfident wholes.
4. **Scope drift:** Design‑time formalism and run‑time evidence are composed into a single score; dashboards then claim “assurance” for a blueprint using live data, or vice versa.


### B.3:3 - Forces

| Force                                    | Tension                                                                                                                             |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| **Conservatism vs. Synthesis**           | Avoid overclaiming (WLNK) ↔ allow real gains from better integration (raise CL) or true emergence (B.2).                            |
| **Universality vs. Domain nuance**       | One calculus for systems and epistemes ↔ physics and epistemology use different primitives; keep them comparable but not identical. |
| **Simplicity vs. Fidelity**              | Keep the assurance tuple small and typed (A.11) ↔ capture enough structure to be informative and improvable by KD‑CAL actions.           |
| **Static clarity vs. Dynamic evolution** | A score must be reproducible today ↔ tomorrow it should legitimately rise after formalization, replication, or reconciliation.      |


### B.3:4 - Solution — **Part 1: The assurance tuple and the universal aggregation skeleton**

B.3 defines **what** the assurance components are, **how** they live on nodes and edges of the dependency graph, and the **shape** of the aggregation that any Γ‑flavour must honor when producing an *assurance result*.

#### B.3:4.1 - The F–G–R assurance components (typed; `F/R` CHR, `G` USM)

We standardize **two node characteristics**, **one node scope object**, and **one edge characteristic**:

1. **Formality (F)** — *how constrained the reasoning is by explicit, proof‑grade structure.*

   * **Scale kind:** **ordinal** (levels do not admit arithmetic).
   * **Canonical levels (example):**
        `F0 Informal prose` - `F1 Structured narrative` - `F2 Formalizable schema` - `F3 Proof‑grade formalism`.
   * **Monotone direction:** higher is better (never lowers assurance when all else fixed).

2. **ClaimScope (G)** — *the declared set of `U.ContextSlice` where the result applies.*

   * **Type:** **set‑valued USM scope object** (A.2.6), **not** a CHR characteristic.
   * **Well‑typed operations:** **membership** and **set algebra** (`∈`, `⊆`, `∩`, `⋃`, `SpanUnion`, plus declared Bridge translation / widen / narrow / refit).
   * **Scalar proxy (report‑only):** if a profile needs a number for reporting, it MAY publish an explicitly declared **`CoverageMetric(G)`**; such a proxy **MUST NOT** replace `G` in norms, gates, bridge semantics, or CL routing.
3. **Reliability (R)** — *how likely the claim/behavior holds under stated conditions.*

   * **Scale kind:** **ratio** in `[0,1]` (or a conservative ordinal proxy when numeric modeling is unavailable).
   * **Monotone direction:** higher is better.

2. **Congruence Level (CL)** — *edge property: how well two parts fit* (semantic alignment, calibration, interface Standard).

   * **Scale kind:** **ordinal** with a **monotone penalty function** `Φ(CL)` where `Φ` decreases as CL increases.
   * **Canonical levels (example):**
     `CL0 weak guess` - `CL1 plausible mapping` - `CL2 validated mapping` - `CL3 verified equivalence`.
   * **Interpretation:** low CL reduces the credibility of the *integration itself* (not the parts), and therefore **penalizes** the aggregate **R**.

> **Strict Distinction (A.15).**
>
> * Assurance components live at **value/scope level**: **F/R** as characteristics, **G** as a scope object, while Γ‑flavours fold **structure/order/time**.
> * Do not smuggle assurance components into structural edges; keep **F/R/CL** explicit as CHR metadata and **G** explicit as a USM scope object.

> **Assurance shoulders (Working‑Model split).**
> **Mapping** raises **TA** (typing, fit/CL). **Logical** and **Constructive** contribute to **VA** (intended relation semantics; Γₘ extensional identity for structure). **Empirical Validation** contributes to **LA** (evidence in a bounded context). These supports attach **downward** from the Working‑Model surface (E.14).

#### B.3:4.2 - Assurance as a typed claim

B.3 speaks about **assurance of a specific typed claim** `C` over a holon `H` under context `K` and scope `S ∈ {design, run}`:

```
Assurance(H, C | K, S) = ⟨F_eff, G_eff, R_eff, Notes⟩
```

* `C` examples: *meets load L*, *argument Q holds*, *model M predicts within δ*.
* `K` binds assumptions (environment, usage, priors).
* `Notes` include the **SCR** (all sources, B.1.3), **OrderSpec/TimeWindow** where applicable (B.1.4), cutsets, and evidence citations (A.10).

This tuple gives readers an at‑a‑glance view (didactic primacy) while preserving the pieces needed for audit and improvement.

**Validation modes (declaration, normative).**
Each published Working‑Model assertion **SHALL** declare **`validationMode ∈ {postulate, inferential, axiomatic}`** (E.14).
— *postulate* → pragmatic working claim; **Empirical Validation** is **required** for audit.
— *inferential* → reasoned consequence; **Logical** assurance carries the burden.
— *axiomatic* → constructive identity; **structural** edges MUST provide a Γₘ narrative and a **`tv:groundedBy`** pointer (C.13, B.3.5).

**Design vs run (no chimeras).** Assurance tuples for **design‑time** and **run‑time** SHALL be reported **separately** and **not composed into a single score**; see the *Scope drift* hazard in §2 and the obligations in B.3.3.

#### B.3:4.3 - Where the numbers live (and do not)

* **On nodes:** each input holon contributes its local `F, G, R` according to its nature (system vs. episteme).
* **On edges:** each integration step has a `CL` (congruence of the connection).
* **Not inside Γ:** Γ consumes `D` and returns a composed holon; B.3 governs how `F, G, R, CL` **propagate** to the **Assurance** tuple for that composed holon. This keeps Γ algebra and assurance calculus **separable** and reviewable.
* **Not a state space:** `⟨F,G,R⟩` is an **assurance tuple**, not a `U.CharacteristicSpace`; do **not** draw “trajectories” in `⟨F,G,R⟩`. For episteme evolution, use **ESG** states and the **assurance‑trace** hooks (see below).

#### B.3:4.4 - Universal aggregation skeleton (domain‑neutral)

Any Γ‑flavour that claims an **Assurance** result **must** adopt the following **conservative skeleton**:

1. **Formality:**

   ```
   F_eff = min_i F_i
   ```

   *Rationale:* the least formal piece caps the formality of the whole (WLNK on F).
   *Monotone:* raising any `F_i` cannot reduce `F_eff`.

2. **ClaimScope:**

   ```
   G_eff = SpanUnion({G_i}) constrained by support
   ```

   * “SpanUnion” is a **set/coverage union** in the domain’s space.
   * **Constraint:** any region in the union **unsupported** by reliable parts is **dropped** (WLNK).
   * *Monotone:* adding supported span cannot reduce `G_eff`.

3. **Reliability (penalized by integration):**

   ```
   R_raw = min_i R_i                       // Weakest-link cap
   R_eff = max(0, R_raw − Φ(CL_min))       // Congruence penalty
   ```

   * `CL_min` is the **lowest** congruence level on any edge in the proof spine / critical integration region for the claim `C`.
   * `Φ` is **monotone decreasing** and **bounded** (never makes negative values).
   * *Monotone:* increasing any `R_i` or any `CL` cannot lower `R_eff`.

4. **SCR and Notes:**
   * The aggregate SHALL produce a SCR listing all contributing nodes and edges, with their F, G, R, CL, scopes, and evidence links (A.10).
   * The SCR SHALL additionally surface the **describedEntity** (`describe(Object→GroundingHolon)`) and the **ReferencePlane** for the claim, and present a **separable TA/VA/LA table** of evidence contributions with **valid_until/decay** marks and the **Epistemic‑Debt** per § B.3.4.
   * If order/time mattered for the claim, attach the OrderSpec or TimeWindow identifiers (B.1.4).

This skeleton is **mandatory**. Domain‑specific patterns may add **refinements** (e.g., separate epistemic “replicability” vs. “calibration”) as long as they **do not violate** WLNK or MONO and preserve scale kinds.


#### B.3:4.5 - System vs. Episteme — same shape, different readings

* **For systems (Γ\_sys):**

  * `F` reads as **engineering discipline** (from ad‑hoc procedure to verified specification).
  * `G` reads as **operational envelope coverage**.
  * `R` reads as **assured reliability** under `K` (requirements, environment, test campaigns).
  * `CL` often arises at **interfaces** (Boundary‑Inheritance Standard; B.1.2): poorly controlled interfaces reduce `R_eff`.

* **For epistemes (Γ\_epist):**

  * `F` reads as **logical/semantic formality** (from prose to proof).
  * `G` reads as **domain span** (concepts, populations, conditions).
  * `R` reads as **evidential support** (replication quality, measurement integrity).
  * `CL` measures **semantic alignment** of merged constructs (terminology mapping, ontology bridges, calibration).

> **Agentness is separate (A.13).**
> Agency metrics (Agency‑CHR) **do not enter the skeleton by default**. They may act as a **contextual overlay** (e.g., to argue why a supervisory policy can maintain `R` across disturbances), but **never** to bypass **WLNK** or the **CL penalty**. Grade shifts should be modeled as **MHT** events when they create new capabilities.


#### B.3:4.6 - Scale discipline (CHR guard‑rails)

To prevent silent misuse:

* **Ordinal scales (F, CL):** never average or subtract; only `min`/`max`, thresholds, and monotone comparisons are valid operations.
* **Coverage scales (G):** use union/intersection in a declared domain space; do not “average” sets. If a numeric proxy is used (e.g., coverage ratio), it **must** be derived from a set operation, not vice versa.
* **Ratio scales (R):** may be combined with `min`, `max`, or **explicitly justified** conservative functions; do not add R’s from different contexts without normalization of `K` (assumptions).


#### B.3:4.7 - What improves the tuple (action patterns, high‑level)

B.3 remains neutral about *how* improvement happens, but for didactic clarity:

* **Raise F:** formalize narratives (specifications, machine‑checked models).
* **Raise G:** enlarge supported span (new test regimes, new populations) with adequate evidence.
* **Raise R:** replicate, calibrate, tighten measurement error, reduce bias.
* **Raise CL:** reconcile vocabularies, align units, formalize mappings, verify interface Standards.

Each of these corresponds to recognizable **Transformer roles** and KD‑CAL moves (design‑time); their **run‑time** counterparts are covered by Γ\_time (phase evidence) and Γ\_work (cost of obtaining assurance).

### B.3:4.8 - Prohibition (normative) — F–G–R is not a CharacteristicSpace
Do not treat `⟨F,G,R⟩` as a `U.CharacteristicSpace` and do not define geometric **trajectories** over it. Use **ESG** for episteme state and the **assurance‑trace** hooks for trends in assurance tuples.

### B.3:5 Proof obligations (attach these when producing an Assurance tuple)

These obligations refine the generic Proof Kit from **B.1.1 §6** for **assurance** outputs. Each Γ‑flavour that emits an *Assurance(H, C | K, S)* tuple MUST attach the applicable obligations below.

#### B.3:5.1 - Common obligations (all Γ‑flavours)

* **ASS‑CLM (Typed claim & context).**
  State the **claim** `C` (what is being assured), the **context** `K` (assumptions, environment), and the **scope** `S ∈ {design, run}`.

* **ASS‑SCA (Scale discipline).**
  Declare the **scale kind** used for each characteristic (F ordinal, G coverage, R ratio) and confirm that all operations are **admissible** for that kind (no averaging of ordinals; G via set/coverage ops).

* **ASS‑WLNK (Weakest‑link evidence).**
  Identify the **cutset** (node or edge set) that caps F/G/R for the claim (the proof spine for epistemes, the structural or assurance bottleneck for systems).

* **ASS‑CL (Congruence path).**
  Identify the **relevant integration path(s)** and record `CL_min` used in the penalty `Φ(CL_min)`.

* **ASS‑MAN (SCR).**
  Produce a **SCR** listing all contributing nodes and edges with `(F, G, R)` and `CL` values, their **DesignRunTag**, and Evidence Graph Ref (A.10). If order or time were material, include the **OrderSpec** or **TimeWindow** identifiers from **B.1.4**.

* **ASS‑MONO (Declared monotone characteristics).**
  List the characteristics along which local improvement cannot reduce the aggregate (this supports future evolution, B.4).

#### B.3:5.2 - Γ\_sys (systems) — additional obligations

* **CORE‑BIC (Interface congruence).**
  Reference the **Boundary‑Inheritance Standard** (BIC) from **B.1.2** and record any interface mismatches; these contribute to `CL_min`.

* **CORE‑ENV (Operating envelope).**
  Specify the domain used for **G** (e.g., load–temperature region) and how coverage is computed (set union constrained by support).

#### B.3:5.3 - Γ\_epist (epistemes) — additional obligations

* **EPI‑SPN (Entailment spine).**
  Identify the **premise/lemma spine** for the claim; `R_raw = min R_i` is taken along this spine, not over arbitrary satellites.

* **EPI‑MAP (Semantic mapping congruence).**
  Point to the **vocabulary/ontology mappings** used; their verification status sets the **CL** levels on the integration edges.

#### B.3:5.4 - Γ\_ctx / Γ\_method (order‑sensitive) — additional obligations

* **CTX‑ORD (OrderSpec).**
  Attach the partial or total order `σ` and any **join‑soundness** conditions (types, pre/post‑conditions).
  (See B.1.4 for NC‑1..3 invariants; B.1.5 adds duration/capability typing.)

#### B.3:5.5 - Γ\_time (temporal) — additional obligations

* **TIME‑COV (Coverage & identity).**
  Show that `PhaseOf` intervals cover the declared window without overlap for the **same carrier**; justify any gap/overlap explicitly.

> **Note on Γ\_work.**
> Resource spending and efficiency live in **Γ\_work**. Their *measurement integrity* can influence **R** for a claim (e.g., if a reliability figure depends on calibrated energy input), but **costs themselves are not assurance**; keep them in Γ\_work and cite their **measurement assurance** as inputs here.


### B.3:6 - Archetypal grounding (worked examples)

#### B.3:6.1 - System archetype — **Battery pack safety claim**

* **Claim `C`:** *Pack P meets discharge current L with thermal safety margin δ in environment K.*
* **Context `K`:** Ambient ≤ 35 °C; airflow ≥ X; duty cycle Y. Scope `S = run`.
* **Graph:** Cells `ComponentOf` modules `ComponentOf` pack; BIC exposes main power and thermal interface.
* **Inputs:**

  * `F` per node: module spec F2, cell test F1 → `F_eff = F1`.
  * `G`: operating envelope regions; union constrained by supported test regimes.
  * `R`: per‑module reliability from test data; cutset is **hot‑spot path** near weakest cell.
  * `CL`: interface congruence (sensor calibration CL2; thermal contact CL1).
* **Aggregation:**

  * `R_raw = min R_i` along the thermal cutset.
  * `R_eff = max(0, R_raw − Φ(CL_min=CL1))`.
  * `G_eff`: union of supported (L,T) rectangles, dropping regions lacking validated thermal data.
  * `F_eff = min(F_cell=F1, F_module=F2) = F1`.
* **SCR:** Evidence for calibration, test campaigns, BIC.
* **Improvement path:** raise `CL` (better thermal interface verification), raise `F` (formal thermal model), add supported envelope → **R\_eff** and **G\_eff** increase monotonically.

#### B.3:6.2 - Episteme archetype — **Meta‑analysis claim**

* **Claim `C`:** *Intervention X reduces outcome O by Δ on population P.*
* **Context `K`:** Inclusion/exclusion criteria, measurement protocol; `S = design`.
* **Graph:** Studies `MemberOf` evidence corpus; effect models `ConstituentOf` synthesis; mappings align different outcome scales.
* **Inputs:**

  * `F`: two RCTs at F3, one observational at F2 → `F_eff = F2`.
  * `R`: per‑study replication/quality → weakest R on the entailment spine caps `R_raw`.
  * `CL`: mapping of scales (CL1 vs CL3).
  * `G`: populations union, but unsupported sub‑populations are dropped.
* **Aggregation:**
* **Aggregation:**
* **\[M‑1]** ordinal support ranking; note weakest‑link study.
* **\[M‑2]** compute `R_eff` with Φ table; record `CL_min` for scale mappings.
* **\[F‑constructive]** formalise the effect‑model equivalence and export proof‑term hash.  # [M/F]

  * `R_eff = max(0, min(R_RCT1, R_RCT2, R_OBS) − Φ(CL_min=CL1))`.
  * `G_eff`: union of supported sub‑populations; out‑of‑scope groups excluded.
* **SCR:** Data provenance, scale mappings, bias assessment.
* **Improvement path:** upgrade mapping verification to CL2/CL3; increase `F` via registered analysis plan; replicate lagging study.

#### B.3:6.3 - Order/Process archetype — **Manufacturing route assurance**

* **Claim `C`:** *Route R meets output defect rate ≤ ε.*
* **Context `K`:** Materials, equipment class; `S = run`.
* **Γ\_ctx artifacts:** `σ` order; declared independent branches; join conditions at inspection.
* **Assurance:**

  * `R_raw = min R_step` along the **critical path** (includes inspection effectiveness).
  * Penalty from poor join soundness `CL_min`.
  * Improvement via faster but **verified** inspection (↑R\_step) or tighter join spec (↑CL).

#### B.3:6.4 - Temporal archetype — **Versioned model credibility**

* **Claim `C`:** *Model M predicts within ±δ over τ.*
* **Context `K`:** Data regime and drift tolerance; `S = run`.
* **Γ\_time artifacts:** `PhaseOf` slices v1, v2, v3 covering `τ`.
* **Assurance:**

  * `R_raw = min(R_v1, R_v2, R_v3)`;
  * penalty if v2–v3 interface had low calibration congruence;
  * improvement via re‑calibration (↑CL) or new validation campaign (↑R\_v3).

### B.3:7 - Conformance Checklist (normative)

| ID          | Requirement                                                                                                                                                                   | Purpose                                      |                                   |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- | --------------------------------- |
| **CC‑B3.1** | An assurance result **SHALL** be a typed claim \`Assurance(H, C                                                                                                               |  K, S)`with`S ∈ {design, run}\`.             | Prevent scope drift and chimeras. |
| **CC‑B3.2** | `F` **SHALL** be treated as **ordinal** (`min`/thresholds only); `G` as **coverage** (set/measure union constrained by support); `R` as **ratio** (`min` + conservative ops). | Preserve scale integrity (CHR).              |                                   |
| **CC‑B3.3** | The **Congruence Level** `CL` **SHALL** live on **edges**; the penalty `Φ(CL)` **SHALL** be **monotone decreasing** and **bounded** (`R_eff ≥ 0`).                            | Make integration quality first‑class.        |                                   |
| **CC‑B3.4** | `R_eff` **SHALL** be computed as `R_eff = max(0, min_i R_i − Φ(CL_min))` for the relevant integration path(s), unless a stricter domain‑specific rule is justified.           | Enforce WLNK and penalize weak integrations. |                                   |
| **CC‑B3.5** | `F_eff = min_i F_i`; `G_eff = SpanUnion({G_i})` **constrained by support**.                                                                                                   | Prevent over‑generalization.                 |                                   |
| **CC‑B3.6** | An **Assurance SCR** **SHALL** be produced, listing node/edge values, Evidence Graph Ref, and any OrderSpec/TimeWindow identifiers, **and SHALL also display**:  (i) the **describedEntity binding** `describe(Object→GroundingHolon)` for the claim and the declared **CHR:ReferencePlane ∈ {world|concept|episteme}** (cf. C.2.3); (ii) a **TA/VA/LA breakdown** of anchored evidence **kept separable** per **CC–KD‑08**, with **decay/valid‑until** indicators on empirical bindings (A.10), and the **Epistemic‑Debt** tally as computed in **§ B.3.4**. | Provide auditability (A.10).                 |
| **CC‑B3.7** | **Agency‑CHR** values (A.13) **SHALL NOT** override WLNK or `Φ(CL)` penalties; if agency grade change alters capabilities, model it as a **Meta‑Holon Transition**.           | Preserve safety; keep agency separate.       |                                   |
| **CC‑B3.8** | Design‑time and run‑time assurance **SHALL NOT** be mixed in one tuple; compare them side‑by‑side if needed.                                                                  | Avoid design/run mixing.                     |                                   |

### B.3:8 - Anti‑patterns and repairs

| Anti‑pattern             | Symptom                                                    | Repair                                                                                                         |
| ------------------------ | ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| **Averaging assurance**  | Mean of `R_i` reported as system reliability               | Use `min R_i` on the cutset, then apply `Φ(CL_min)`.                                                           |
| **Ordinal arithmetic**   | Averaging `F` or `CL` to produce “2.3”                     | Use `min`/`max` or thresholds; never average ordinals.                                                         |
| **Coverage as centroid** | Replacing `G` union with a single “typical point”          | Keep `G` as set/coverage; if a numeric proxy is needed, derive it from the set.                                |
| **Ignoring congruence**  | No penalty for weak mappings/interfaces                    | Assign `CL` to integration edges; apply `Φ(CL_min)`.                                                           |
| **Design/run chimera**   | “One score” mixing blueprint and telemetry                 | Split into `S=design` and `S=run` tuples; compare explicitly.                                                  |
| **Agency override**      | Claiming higher assurance because a controller is “clever” | Agency may justify *how* improvements are achieved; it cannot remove WLNK or `Φ`.                              |
| **MemberOf as stock**    | Using `MemberOf` to sum reliabilities                      | Keep `MemberOf` for collections; reliability comes from the relevant **Γ** composition (e.g., Γ\_sys cutset). |


### B.3:9 - Consequences

**Benefits**

* **Comparable, conservative, improvable.** The tuple ⟨F, G, R⟩ with **edge‑level CL** gives a compact, auditable view that improves monotonically under targeted actions (formalize, replicate, reconcile).
* **Cross‑scale coherence.** Works for assemblies and arguments, procedures and histories, without leaking order/time/cost into structure.
* **Clear upgrade paths.** It is obvious **what to do** to raise each component (raise F/G/R locally or raise CL on the glue).

**Trade‑offs**

* **More explicit metadata.** You must state scale kinds, cutsets, and mapping congruence; this is intentional transparency.
* **Conservatism may feel pessimistic.** True synergy appears only via **MHT** or after raising CL—never by arithmetic optimism.


### B.3:10 - Rationale (informative)

B.3 distills mature post‑2015 practice across several fields into a single, small calculus:

* **Assurance by weakest link** reflects reliability engineering and safety cases in complex systems; composing claim strength by minima prevents over‑statement.
* **Formality and verifiability** mirror advances in model‑based engineering and formal verification, where raising F turns subjective arguments into verifiable artifacts.
* **Coverage as set/measure** follows evidence synthesis and validation practice that treat applicability as a domain region, not a scalar to “average.”
* **Congruence on edges** captures what meta‑analysis, interface control, and ontology alignment have repeatedly shown: integration quality is often the real bottleneck. Penalizing low‑CL is a principled way to prevent silent over‑confidence while rewarding verified reconciliation.

This arrangement preserves **A.11 Parsimony** (few characteristics), aligns with **A.14/A.15** (clear separation of structure, order, time, cost, values), and leaves Context for domain‑specific refinements that do not break the invariants.


### B.3:11 - Relations

* **Builds on:** B.1 (Universal Γ), B.1.1 (Proof Kit), B.1.2 (Γ_sys & BIC), B.1.3 (Γ_epist & SCR), B.1.4 (Γ_ctx/Γ_time), A.12 (Transformer), A.14 (Mereology), A.15 (Strict Distinction), **C.13 (Compose‑CAL)**.
* * **Coordinates with:** **E.14 (Human‑Centric Working‑Model)** for publication‑surface discipline and **B.3.5 (CT2R‑LOG)** for Working‑Model relation aliasing and grounding (`tv:*`, `validationMode`).
* **Used by:** KD‑CAL action patterns (to plan improvements), B.4 (Evolution loops that raise F/G/R or CL over time).
* **Triggers:** B.2 (Meta‑Holon Transition (MHT): Recognizing Emergence and Re‑identifying Wholes) when genuine new capabilities emerge that change the applicable cutsets or envelopes.

> **One‑page takeaway.**
> Report assurance as **⟨F, G, R⟩** for a **typed claim** under explicit **context/scope**, and penalize by the **lowest edge‑level congruence**.
> Improve assurance by raising **F**, **G**, **R**, or **CL**—and keep order, time, and cost in their own lanes.

### B.3:11a - Assurance posture for quantum-like claims

Quantum-like wording does not raise assurance burden by default. A local `C.26` modeling note can remain lightweight when it only prevents a representational mistake and does not support action, close audit, certify readiness, or claim empirical superiority.

Action path:

1. Decide the claim strength before building assurance machinery.
2. If the QL note only prevents a local misreading, keep it as QL-lite with ordinary evidence.
3. If the claim will be reused, state source role, ordinary FPF pattern, local stop condition, and evidence posture.
4. If the claim supports release, readiness, audit, compliance, assurance, or high-impact action, build the B.3 assurance claim over named evidence carriers and scope.
5. If the claim says QL is better, faster, more accurate, or uniquely necessary, compare rival models, baseline, mechanism, scope, and loss.
6. State decay/reopen conditions so an old QL-supported assurance claim does not silently stay current after probes, carriers, or scope change.

| Claim strength | B.3 expectation | Output |
| --- | --- | --- |
| Local modeling note | No assurance tuple beyond the ordinary pattern and evidence note | QL-lite note with local stop |
| Reusable pattern/example | Name source role, ordinary pattern, local stop condition, and evidence posture | Reusable example with source-support role |
| Decision, release, audit, readiness, or compliance use | Provide F-G-R/congruence support, evidence carriers, confidence, rival explanations, and decay/reopen conditions | Assurance tuple and evidence route |
| Comparative superiority claim | Add rival-model comparison, baseline, mechanism, and scope limits | Bounded superiority claim or reroute |

Useful outputs:

- no B.3 action when QL is only a local representational lens;
- a lightweight assurance note when reuse is modest;
- a full assurance tuple only when consequence level demands it;
- a rejected or weakened claim when evidence does not support the intended use.

### B.3:End
