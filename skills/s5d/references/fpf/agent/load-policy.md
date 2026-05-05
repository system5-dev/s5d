# FPF Agent Load Policy

Goal: use FPF without loading the full source.

1. Start with `agent/entrypoints.yaml`, `agent/glossary.yaml`, and `agent/query-index.jsonl`.
2. Select 1-5 candidate cards by trigger, keyword, title, or query match.
3. Read only matching `cards/**/*.card.yaml`.
4. Load a full `modules/**/*.md` file only when exact wording, checklist, or rationale is needed.
5. Expand graph one hop at a time through `builds_on`, `refines`, and `coordinates_with`.
6. Treat cards as navigation. Treat modules as evidence. Treat `source/FPF-Spec.md` as canonical.
7. Cite module id plus source span when using FPF in a decision.
