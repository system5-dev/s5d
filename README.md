# S5D

> **Status: alpha / experimental**

**A control plane for repository changes made with AI.** It doesn't replace git, tests, CI, or your tracker — it adds a thin layer of decision discipline and traceability on top, so a change is never just "the agent wrote some code." The machine core tracks desired state, actual code state, transitions, evidence, and violations; the human-facing text (scope, phase names, status summaries) is a *derived* rendering, never the source of truth.

**🇬🇧 English** · [🇷🇺 Русская версия](#что-делает-s5d)

## What S5D does

### The problem it solves

A normal flow is *understand the task → write code → run tests → open a PR*. When an AI agent writes or co-writes the code, that isn't enough: the agent can produce plausible code **and** a plausible report, while the reasoning — why this and not the alternative, what actually passed — lives only in a chat window and then evaporates. S5D makes that reasoning durable, bound to real state, and checkable. Code should never appear as "agent magic"; you should always be able to recover *why* a change exists.

### Four things on top of normal development

1. **Target state** — before code, describe what the system should *become* and which existing architecture it reuses (domains, capabilities, components). You design the system you want; you don't just patch the one you have, and you don't invent new architecture without a reason.
2. **Explicit choice** — compare real alternatives before committing. A decision needs ≥3 hypotheses that differ *in kind*, not three flavours of one idea. *"We picked X because it's right"* is not a decision. *"We picked X because it covers revocation, which Y can't, while Z is simpler but weaker on security"* is.
3. **Run evidence** — record what agents and tools actually produced: which gates ran, which tests passed, which review findings closed, who approved, and the SHA256 of the spec at approval and import. The report is **bound to real state**, not taken on faith — which matters most exactly when an agent generated it.
4. **Verify in code** — check that the code matches the decision. If the spec says component `auth/session` owns the change but the diff touches `billing`, that's **drift** — S5D surfaces it, and you trace, reconcile, or roll back.

### Where the truth lives

Only two places hold lifecycle truth:

- **`.s5d/packages/*` — specs.** A spec is the plan and the contract: intent, acceptance criteria, components, gates, hypotheses, contracts.
- **`.s5d/records/*` — records.** A record is the history of what happened to that contract: preview, approval, gate results, import, reflection.

**Not** markdown notes, **not** a Shape doc, **not** a review report on its own, **not** "we agreed in chat." If it isn't in a package or a record, it isn't accepted S5D state. The CLI enforces a SHA256 chain between spec and record, so an approval granted for one plan can't silently cover a different one — edit a spec after approval and the approval no longer matches.

### The flow

`Route → Shape → Discover → Target → Decide → Spec → Run → Verify → Ship → Learn`

It's the map, not a mandatory checklist — most tasks run only part of it.

- **Route** — classify the request: which tier, which mode, where to enter.
- **Shape** — turn a raw or vague request ("make checkout better") into a clear intake kernel: why it matters, impacted capabilities, constraints, non-goals, success signal, assumptions, open questions. Shape is *intake only* — it never creates accepted state.
- **Discover** — map an unfamiliar repo's current architecture into structured tables (domains, capabilities, components, edges), every claim tagged `[VERIFIED] / [INFERRED] / [SPECULATIVE]`.
- **Target** — state what's anomalous now and define acceptance *before* options. Describe the behaviour that should be true, not the implementation method.
- **Decide** — for real tradeoffs: ≥3 distinct hypotheses, evidence per hypothesis, a confirmed winner. Human confirmation here is mandatory and cannot be waived.
- **Spec** — turn the decision into an executable contract: problem → acceptance scenarios → components → gates, tracing every user-facing change `Feature → UseCase → Capability → Component`.
- **Run** — preview → human approval (mandatory) → implement → run gates. An agent finishing the work is evidence; human acceptance stays explicit.
- **Verify / Ship / Learn** — tests and (for material work) adversarial review → import under the SHA256 chain → push/deploy only with explicit human permission → reflect and record reusable heuristics.

### When you need it — and when you don't

**Skip S5D** for bugfixes under ~30 LOC, config-only changes, docs-only changes, status queries, and small local edits with no architectural meaning. Just do them.

**Use S5D** for a new feature; an architecture change; anything touching auth, payment, security, PII, or compliance; changes spanning multiple domains; AI-agent work that must prove traceability; decisions with a real tradeoff; and changes that need acceptance criteria or touch a trust boundary.

Rule of thumb: **if you can't say which spec covers your diff, you probably need S5D.** Tiers (Note → Decision → Lightweight → Standard → High) scale the rigor to the risk — see [Tiers](#tiers) below; when unsure, pick the higher one.

### What can never be skipped

S5D lets you waive *workflow steps* with a recorded `WAIVER` line — but two things are never waivable: **human confirmation of a decision** and **human approval of a run**. An agent's say-so is never a substitute for either. *"The AI decided, so it's confirmed"* and *"skip approval, it's urgent"* are both refused by design.

### In one line

A git commit says *what changed.* S5D adds *why it had to change, which alternatives were weighed, who confirmed the choice, which files belong to it, what checks passed, who verified the result, and what was learned* — turning a change from a patch into a traceable engineering decision.

---

## Что делает S5D

[🇬🇧 English version](#what-s5d-does) · **🇷🇺 Русский**

### Какую проблему решает

Обычный процесс: *понять задачу → написать код → прогнать тесты → открыть PR.* Когда код пишет или дописывает AI-агент, этого мало: агент может выдать правдоподобный код **и** правдоподобный отчёт, а рассуждение — почему именно так, а не иначе, что реально прошло — остаётся только в окне чата и потом испаряется. S5D делает это рассуждение долговечным, привязанным к реальному состоянию и проверяемым. Код не должен появляться как «магия агента» — всегда должно быть понятно, *зачем* изменение существует.

### Четыре вещи поверх обычной разработки

1. **Целевое состояние (target state)** — до кода опиши, какой система должна *стать* и какую существующую архитектуру (домены, capabilities, компоненты) она переиспользует. Ты проектируешь нужную систему, а не просто патчишь текущую и не изобретаешь новую архитектуру без причины.
2. **Явный выбор** — сравни реальные альтернативы до фиксации. Решению нужно ≥3 гипотезы, *разные по сути*, а не три варианта одной идеи. *«Выбрали X, потому что так правильно»* — это не решение. *«Выбрали X, потому что он закрывает revocation, чего не может Y, а Z проще, но слабее по безопасности»* — решение.
3. **Доказательства выполнения (run evidence)** — записывай, что реально произвели агенты и инструменты: какие gates прогнаны, какие тесты прошли, какие review-findings закрыты, кто одобрил, и SHA256 спека на момент approval и import. Отчёт **привязан к реальному состоянию**, а не принят на веру — это критично именно тогда, когда отчёт сгенерировал агент.
4. **Проверка в коде** — убедись, что код совпадает с решением. Если спек говорит, что изменение принадлежит компоненту `auth/session`, а diff трогает `billing` — это **drift**, и S5D его показывает; дальше ты trace-ишь, reconcile-ишь или откатываешь.

### Где живёт истина

Lifecycle-истина живёт только в двух местах:

- **`.s5d/packages/*` — спеки.** Спек — это план и контракт: intent, критерии приёмки, компоненты, gates, гипотезы, контракты.
- **`.s5d/records/*` — записи (records).** Record — это история того, что с контрактом произошло: preview, approval, результаты gates, import, рефлексия.

**Не** markdown-заметки, **не** Shape-документ, **не** review-отчёт сам по себе, **не** «договорились в чате». Если этого нет в package или record — это не принятое состояние S5D. CLI держит SHA256-цепочку между спеком и record, поэтому approval, выданный на один план, не может молча покрыть другой — поменяй спек после approval, и approval перестаёт совпадать.

### Поток (flow)

`Route → Shape → Discover → Target → Decide → Spec → Run → Verify → Ship → Learn`

Это карта, а не обязательный чек-лист — большинство задач проходят лишь часть.

- **Route** — классифицировать запрос: какой tier, какой mode, с какой стадии входить.
- **Shape** — превратить сырой или мутный запрос («сделать нормальный checkout») в чёткое intake-ядро: зачем нужно, затронутые capabilities, ограничения, non-goals, сигнал успеха, допущения, открытые вопросы. Shape — *только intake*, он не создаёт принятого состояния.
- **Discover** — разложить текущую архитектуру незнакомого репозитория в структурированные таблицы (домены, capabilities, компоненты, связи); каждое утверждение помечено `[VERIFIED] / [INFERRED] / [SPECULATIVE]`.
- **Target** — сказать, что сейчас аномально, и определить приёмку *до* вариантов. Описывай поведение, которое должно стать истинным, а не способ реализации.
- **Decide** — для настоящих tradeoff-ов: ≥3 разные по сути гипотезы, доказательства по каждой, подтверждённый winner. Подтверждение человеком здесь обязательно и не waiver-ится.
- **Spec** — превратить решение в исполнимый контракт: проблема → сценарии приёмки → компоненты → gates, трассируя каждое пользовательское изменение `Feature → UseCase → Capability → Component`.
- **Run** — preview → одобрение человеком (обязательно) → реализация → прогон gates. Завершение работы агентом — это доказательство; приёмка человеком остаётся явной.
- **Verify / Ship / Learn** — тесты и (для значимых изменений) adversarial review → import под SHA256-цепочкой → push/deploy только с явного разрешения человека → рефлексия и запись переиспользуемых эвристик.

### Когда он нужен, а когда нет

**Пропусти S5D** для багфиксов меньше ~30 строк, config-only, docs-only изменений, status-запросов и мелких локальных правок без архитектурного смысла. Просто сделай их.

**Используй S5D** для новой фичи; изменения архитектуры; всего, что трогает auth, payment, security, PII или compliance; изменений в нескольких доменах; работы AI-агента, где нужна доказуемая traceability; решений с реальным tradeoff; и изменений, которым нужны критерии приёмки или которые трогают trust boundary.

Эвристика: **если ты не можешь сказать, какой спек покрывает твой diff, — скорее всего, S5D нужен.** Tiers (Note → Decision → Lightweight → Standard → High) масштабируют строгость под риск — см. [Tiers](#tiers) ниже; в сомнениях выбирай более высокий.

### Что нельзя пропустить никогда

S5D позволяет waiver-нуть *шаги процесса* записанной строкой `WAIVER` — но две вещи не waiver-ятся никогда: **подтверждение решения человеком** и **одобрение run человеком**. Слова агента не заменяют ни то, ни другое. *«AI решил — значит, подтверждено»* и *«пропустим approval, это срочно»* отклоняются by design.

### В одну строку

Git-commit говорит *что изменилось.* S5D добавляет *зачем это должно было измениться, какие альтернативы взвесили, кто подтвердил выбор, какие файлы к нему относятся, какие проверки прошли, кто верифицировал результат и чему научились* — превращая изменение из патча в трассируемое инженерное решение.

## How it works

Two files per change: a **spec** (what you intend) and a **record** (what happened). The spec can also carry optional workflow support for phases and roles; the record can close the loop with telemetry refs and an explicit verdict. The CLI enforces a SHA256 chain between them:

```
spec → preview → approve → import → drift-check
                                         ↓
                                    reconcile / rollback
```

## Quick start

```bash
# Install
git clone https://github.com/system5-dev/s5d.git
cd s5d
./install.sh

# Initialize in your repo
s5d init
# If the repo has .git/, init installs a Rust-backed pre-commit hook.
# Manual hook entrypoint: s5d hook pre-commit

# Check or apply S5D binary/skill updates
s5d admin update check
s5d admin update apply
# Sessions self-update: the plugin's SessionStart hook runs `s5d update auto`,
# which applies in the background when the checkout is clean and on the default
# branch (otherwise it prints the prompt). Opt out with S5D_AUTO_UPDATE=0.
# Trust note: self-update builds and installs whatever origin's default branch
# serves (ff-only) — enable it only for checkouts whose origin you trust.

# Optional, recommended on a repo S5D hasn't seen: build the discovery index
# (file index, evidence, dependency graph under .s5d/discovery/) so specs and
# trace queries can link to real code. Agent flows run a fuller Discover stage.
s5d discover sync

# Create a spec — the scaffold validates out of the box; edit the generated
# placeholder architecture (domain/capability/component, TODO paths) to match reality
s5d new feat.my-feature --product myapp

# Edit the spec YAML, then:
s5d verify validate .s5d/packages/feat.my-feature__*.s5d.yaml
s5d state preview .s5d/packages/feat.my-feature__*.s5d.yaml
s5d state approve .s5d/packages/feat.my-feature__*.s5d.yaml --reviewer reviewername

# Implement your code, then:
s5d verify run-gates .s5d/packages/feat.my-feature__*.s5d.yaml
s5d state import .s5d/packages/feat.my-feature__*.s5d.yaml --verified-by verifiername

# Optional: run a bounded agent task and record evidence
s5d run list .s5d/packages/feat.my-feature__*.s5d.yaml
s5d run start .s5d/packages/feat.my-feature__*.s5d.yaml --id prototype
s5d run task .s5d/packages/feat.my-feature__*.s5d.yaml --phase prototype --engine ralph
s5d run exec .s5d/packages/feat.my-feature__*.s5d.yaml --id prototype --engine local-engine
s5d run accept .s5d/packages/feat.my-feature__*.s5d.yaml --id prototype --reviewer yourname

# Later: close the loop with telemetry-backed outcome
s5d state reflect .s5d/packages/feat.my-feature__*.s5d.yaml \
  --summary "Telemetry stayed inside target bounds" \
  --verdict confirmed \
  --measurement-window "7d post-ship" \
  --telemetry "grafana://my-dashboard" \
  --heuristic "Keep rollout verdicts tied to explicit telemetry refs"

# Later: verify nothing drifted
s5d state drift-check
```

## Agent Run Control

S5D records agent execution as evidence against a desired system state. It can render that evidence for humans, but the durable state remains machine-readable.

- `s5d run list/start/exec/accept` manages active work state in `.record.yaml`
- `s5d run exec --engine <name>` executes an approved command template from `.s5d/config.yaml`, captures stdout/stderr under `.s5d/runs/`, and records the output hash in `.record.yaml`
- `s5d run task --engine ralph [--mode init|bugfix]` emits a bounded task package for the active work state only
- each `run task` call persists the package under `.s5d/tasks/`
- engine completion does not accept the work; human `run accept` remains explicit
- `s5d run harness start`, `s5d run harness status`, and `s5d run harness exec` add the operational layer: isolated git worktree, clean preflight, heartbeat/status, timeout, and journal under `.s5d/harness/`
- harness state is not run truth; `.record.yaml` remains authoritative for active work state, evidence, gates, and approvals
- `s5d run benchmark <suite.json|yaml> [--format markdown|json]` scores paired native vs skill-guided assistant runs with a fixed rubric and required scenario tags so improvements can be compared as evidence instead of anecdotes
- `s5d discover sync/check` builds `.s5d/discovery/*`: file index, evidence JSONL, graph JSON, and a metamodel projection. The core is stack-agnostic; language parsers can be added later as optional evidence providers.
- `ralph-init` warms repo context from docs, tests, environment setup, and current test results
- `ralph-bugfix` enforces regression-first bugfix execution with explicit root-cause evidence
- `s5d state reflect --verdict --measurement-window --telemetry` records outcome evidence after rollout

Legacy aliases such as `s5d validate`, `s5d preview`, `s5d apply preview`, `s5d phase run`, `s5d execute loop`, `s5d harness start`, `s5d update check`, and `s5d install` remain callable for existing scripts, but the public surface shown in `s5d --help` uses `verify`, `state`, `run`, and `admin`.

## Non-goals

- Not a replacement for git
- Not a replacement for tests
- Not a planning or project management system
- Not required for small fixes (bugfix <30 LOC, config-only, docs-only — just do them)
- Not a code generator

## Agent integration

S5D works as an MCP server (experimental) for AI coding agents:

```bash
# Claude Code
s5d init --claude

# All supported agents
s5d init --all
```

### Install as a plugin

Instead of cloning, install S5D directly from the GitHub repo.

**Claude Code** — add the marketplace, then install the plugin:

```
/plugin marketplace add system5-dev/s5d
/plugin install s5d@s5d
```

Updates: `/plugin marketplace update s5d`.

**Gemini CLI** — install the extension from the repo URL (requires `git`):

```bash
gemini extensions install https://github.com/system5-dev/s5d
```

Updates: `gemini extensions update s5d`.

**Codex (and CLI/MCP from a checkout)** — Codex has no marketplace install; use the
`install.sh` path above, then register the MCP server and skills from the checkout:

```bash
s5d init --codex   # or --claude / --gemini / --cursor / --all
```


## CI enforcement

```bash
s5d ci init            # GitHub Actions (default); --gitlab / --all for more
```

Generates a self-contained PR pipeline that installs the pinned s5d release binary and runs `s5d ci exec` — built-in read-only checks only: spec validation, component path/architecture marker checks for specs that declare components, explicit architecture checks, and drift-check. Configured command gates (test/lint/contract) **never execute in generated CI** — a fork PR must not run repo-configured commands on the runner. The generated files carry a version marker; `s5d ci check` reports stale or user-modified config.

## Tiers

| Tier | When | Default gates |
|------|------|---------------|
| Note | Capture context | None |
| Decision | Compare alternatives | Review |
| Lightweight | Simple feature | Schema |
| Standard | Regular feature | Schema + Graph |
| High | Auth / payment / PII | Schema + Graph + Review |

The review gate is satisfied by recorded review evidence: `s5d decision add-evidence <spec> --hypothesis-id <id> --evidence-type gate:review --verdict pass ...` (works on decision- and high-tier specs).

Schema, graph, and architecture gates run built-in validation. Generated CI always checks declared `components[].paths` as implementation markers; add `architecture` to a spec when you also want the explicit architecture gate in local lifecycle flows. Use `s5d codebase sync/check` when you track `.s5d/codebase/*` coverage snapshots, and `s5d discover sync/check` when you track `.s5d/discovery/*` discovery artifacts. Add `lint`, `test`, `contract` gates to your spec when you've configured commands for them in `.s5d/config.yaml`.

`install.sh` must be run from a checked-out repository copy. `curl | bash` is intentionally unsupported.

## Documentation

- `skills/s5d/SKILL.md` — command reference and flow
- `skills/s5d/metamodel.md` — artifact definitions and validation rules
- `skills/s5d/session-protocol.md` — WAL format, conflict resolution
- `docs/testing-and-benchmarking.md` — test layers and skill-vs-native benchmark contract
- `examples/skill-benchmark.json` — five-family fuzzy benchmark suite template (`happy-path`, `edge-case`, `failure-handling`, `scope-drift`, `stale-intent`)

## Credits

Built on the shoulders of giants:

- Groundwork and tooling from **system5-dev**.
- Metamodel authored by **Ivan Podobed** (Иван Подобед).
- Partially draws on **FPF** by **Anatoly Levenchuk** (Анатолий Левенчук).

No warranties either way — see License. 🙂

## License

Apache-2.0 **with the Commons Clause** — source-available; you may not Sell the
Software (resale, paid hosting, or a paid product/service deriving substantially
from S5D). Internal and non-commercial use, modification, and redistribution
remain permitted. See [LICENSE](LICENSE).
