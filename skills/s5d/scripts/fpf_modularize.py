#!/usr/bin/env python3
"""Build an agent-readable modular FPF corpus from one markdown source.

The source remains canonical. Generated module files preserve original section
text and add only YAML frontmatter for navigation and provenance.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable


SCHEMA = "s5d.fpf-corpus/0.1"

RELATION_MARKERS = {
    "Builds on": "builds_on",
    "Refines": "refines",
    "Prerequisite for": "prerequisite_for",
    "Used by": "used_by",
    "Uses": "uses",
    "Coordinates with": "coordinates_with",
    "Constrains": "constrains",
    "Constrained by": "constrained_by",
    "Informs": "informs",
    "Interacts with": "interacts_with",
    "Enables": "enables",
    "Constitutes": "constitutes",
    "Specialized by": "specialized_by",
    "Specialises": "specializes",
    "Specializes": "specializes",
    "Consumes/cites": "consumes_cites",
    "Produces": "produces",
    "Instances": "instances",
}
SECTION_ID = r"(?:[A-K]\.[0-9][A-Za-z0-9-]*(?:[.:][A-Za-z0-9][A-Za-z0-9-]*)*|P-[0-9]+|C-[0-9]+)"
ID_RE = re.compile(rf"\b({SECTION_ID})\b")
HEADING_RE = re.compile(r"^(#{1,6})\s+(.+?)\s*$")
PART_RE = re.compile(r"(?i)^Part\s+([A-Z])\b")
MODULE_HEADING_RE = re.compile(
    rf"^({SECTION_ID})"
    r"\s*(?:[-–—:|]\s*)?(.*)$"
)


@dataclass
class Section:
    id: str
    title: str
    kind: str
    part: str
    level: int
    start_line: int
    end_line: int = 0
    path: str = ""
    status: str | None = None
    keywords: list[str] = field(default_factory=list)
    queries: list[str] = field(default_factory=list)
    relations: dict[str, list[str]] = field(default_factory=dict)


def main() -> None:
    skill_root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(
        description="Split FPF markdown into modules, cards, and relation graph."
    )
    parser.add_argument("source", type=Path, help="Canonical FPF markdown source")
    parser.add_argument(
        "--out",
        type=Path,
        default=skill_root / "references" / "corpus",
        help="Output corpus directory",
    )
    parser.add_argument(
        "--clean",
        action="store_true",
        help="Remove the output directory before writing",
    )
    args = parser.parse_args()

    source = args.source.resolve()
    out_dir = args.out.resolve()
    text = source.read_text(encoding="utf-8")
    build_corpus(source, text, out_dir, clean=args.clean)


def build_corpus(source: Path, text: str, out_dir: Path, *, clean: bool) -> None:
    if clean and out_dir.exists():
        shutil.rmtree(out_dir)

    lines = text.splitlines()
    source_hash = sha256_text(text)
    toc_metadata = parse_toc_metadata(lines)
    sections = find_sections(lines)
    attach_metadata(sections, toc_metadata, lines)
    write_corpus(source, text, source_hash, sections, out_dir)
    verify_source_spans(text, sections, out_dir)


def parse_toc_metadata(lines: list[str]) -> dict[str, dict[str, object]]:
    metadata: dict[str, dict[str, object]] = {}
    for line in lines:
        stripped = line.strip()
        if not stripped.startswith("|"):
            continue
        columns = [clean_markdown(col.strip()) for col in stripped.strip("|").split("|")]
        if len(columns) < 4:
            continue
        module_id = columns[0].strip()
        if not ID_RE.fullmatch(module_id):
            continue
        metadata[module_id] = {
            "title": columns[1].strip(),
            "status": columns[2].strip() or None,
            "keywords": extract_keywords(columns[3]),
            "queries": extract_queries(columns[3]),
            "relations": extract_relations(" ".join(columns[4:]), source_line=0),
        }
    return metadata


def find_sections(lines: list[str]) -> list[Section]:
    sections: list[Section] = []
    current_part = "root"
    seen_slugs: dict[str, int] = {}

    headings: list[tuple[int, str, int]] = []
    for index, line in enumerate(lines, start=1):
        match = HEADING_RE.match(line)
        if not match:
            continue
        level = len(match.group(1))
        title = clean_markdown(match.group(2).strip().strip("#").strip())
        headings.append((level, title, index))

    for heading_index, (level, title, line_no) in enumerate(headings):
        if title.lower() == "table of content":
            continue

        part_match = PART_RE.match(title)
        if level == 1 and title.lower().startswith("preface"):
            current_part = "preface"
            continue
        if level == 1 and part_match:
            current_part = part_match.group(1).upper()
            continue
        if level > 2:
            continue

        module_match = MODULE_HEADING_RE.match(title)
        if module_match:
            module_id = module_match.group(1)
            module_title = module_match.group(2).strip() or title
            kind = "pattern"
            part = module_id[0] if module_id[0].isalpha() else current_part
        elif current_part == "preface" and level == 2:
            base = slugify(title)
            seen_slugs[base] = seen_slugs.get(base, 0) + 1
            suffix = "" if seen_slugs[base] == 1 else f"-{seen_slugs[base]}"
            module_id = f"preface.{base}{suffix}"
            module_title = title
            kind = "preface_article"
            part = "preface"
        else:
            continue

        end_line = len(lines)
        for next_level, _, next_line in headings[heading_index + 1 :]:
            if next_level <= 2:
                end_line = next_line - 1
                break

        sections.append(
            Section(
                id=module_id,
                title=module_title,
                kind=kind,
                part=part,
                level=level,
                start_line=line_no,
                end_line=end_line,
            )
        )

    return sections


def attach_metadata(
    sections: list[Section],
    toc_metadata: dict[str, dict[str, object]],
    lines: list[str],
) -> None:
    for section in sections:
        block = lines[section.start_line - 1 : section.end_line]
        section.keywords = unique(
            [
                *toc_metadata.get(section.id, {}).get("keywords", []),
                *extract_keywords("\n".join(block)),
            ]
        )
        section.queries = unique(
            [
                *toc_metadata.get(section.id, {}).get("queries", []),
                *extract_queries("\n".join(block)),
            ]
        )
        section.status = toc_metadata.get(section.id, {}).get("status") or None
        relations: dict[str, list[str]] = {}
        merge_relations(
            relations,
            toc_metadata.get(section.id, {}).get("relations", {}),
        )
        for kind, targets in extract_relations(
            "\n".join(block), source_line=section.start_line
        ).items():
            relations.setdefault(kind, [])
            relations[kind] = unique([*relations[kind], *targets])
        section.relations = relations


def write_corpus(
    source: Path,
    text: str,
    source_hash: str,
    sections: list[Section],
    out_dir: Path,
) -> None:
    source_dir = out_dir / "source"
    modules_dir = out_dir / "modules"
    cards_dir = out_dir / "cards"
    graph_dir = out_dir / "graph"
    agent_dir = out_dir / "agent"
    for directory in [source_dir, modules_dir, cards_dir, graph_dir, agent_dir]:
        directory.mkdir(parents=True, exist_ok=True)

    source_name = source.name
    (source_dir / source_name).write_text(text, encoding="utf-8")

    node_ids = {section.id for section in sections}
    nodes = []
    edges = []
    query_rows = []

    for section in sections:
        module_path = module_rel_path(section, suffix=".md")
        card_path = module_rel_path(section, prefix="cards", suffix=".card.yaml")
        section.path = module_path
        source_lines = text.splitlines()
        body = "\n".join(source_lines[section.start_line - 1 : section.end_line]) + "\n"
        frontmatter = module_frontmatter(section, source_name, source_hash)
        write_text(out_dir / module_path, f"---\n{frontmatter}---\n\n{body}")
        write_text(out_dir / card_path, card_yaml(section, source_name, source_hash, body))

        nodes.append(
            {
                "id": section.id,
                "kind": section.kind,
                "title": section.title,
                "part": section.part,
                "status": section.status,
                "path": module_path,
                "card": card_path,
                "token_estimate": estimate_tokens(body),
            }
        )
        query_rows.append(
            {
                "id": section.id,
                "title": section.title,
                "kind": section.kind,
                "part": section.part,
                "status": section.status,
                "keywords": section.keywords[:24],
                "queries": section.queries[:12],
                "load": {"card": card_path, "full": module_path},
            }
        )
        for kind, targets in section.relations.items():
            for target in targets:
                edges.append(
                    {
                        "from": section.id,
                        "to": target,
                        "kind": kind,
                        "status": "inferred",
                        "target_exists": target in node_ids,
                        "source": {
                            "path": f"source/{source_name}",
                            "start_line": section.start_line,
                            "end_line": section.end_line,
                        },
                    }
                )

    write_jsonl(graph_dir / "nodes.jsonl", nodes)
    write_jsonl(graph_dir / "edges.jsonl", edges)
    write_jsonl(agent_dir / "query-index.jsonl", query_rows)
    write_text(agent_dir / "glossary.yaml", glossary_yaml(query_rows))
    write_text(graph_dir / "adjacency.yaml", adjacency_yaml(edges))
    write_text(agent_dir / "load-policy.md", load_policy_md())
    write_text(agent_dir / "entrypoints.yaml", entrypoints_yaml(node_ids))
    write_text(out_dir / "manifest.yaml", manifest_yaml(source_name, source_hash, text, sections, edges))
    write_text(out_dir / "README.md", corpus_readme(len(sections), len(edges)))


def module_frontmatter(section: Section, source_name: str, source_hash: str) -> str:
    data = {
        "id": section.id,
        "title": section.title,
        "kind": section.kind,
        "part": section.part,
        "status": section.status,
        "source": {
            "path": f"source/{source_name}",
            "sha256": source_hash,
            "start_line": section.start_line,
            "end_line": section.end_line,
        },
        "relations": section.relations,
    }
    return to_yaml(data)


def card_yaml(section: Section, source_name: str, source_hash: str, body: str) -> str:
    data = {
        "id": section.id,
        "title": section.title,
        "kind": section.kind,
        "part": section.part,
        "status": section.status,
        "token_estimate": {
            "card": 220 + len(section.keywords) * 4 + len(section.queries) * 8,
            "full": estimate_tokens(body),
        },
        "when_to_load": unique([*title_terms(section.title), *section.keywords])[:24],
        "provides": section.queries[:8],
        "relations": section.relations,
        "source": {
            "module": section.path,
            "path": f"source/{source_name}",
            "sha256": source_hash,
            "start_line": section.start_line,
            "end_line": section.end_line,
        },
        "summary": first_paragraph(body),
    }
    return to_yaml(data)


def extract_relations(text: str, *, source_line: int) -> dict[str, list[str]]:
    del source_line
    marker_pattern = "|".join(re.escape(marker + ":") for marker in RELATION_MARKERS)
    relations: dict[str, list[str]] = {}
    for raw_line in text.splitlines():
        line = clean_markdown(raw_line)
        matches = list(re.finditer(marker_pattern, line, flags=re.IGNORECASE))
        for index, match in enumerate(matches):
            marker_text = match.group(0).rstrip(":")
            canonical = next(
                kind
                for marker, kind in RELATION_MARKERS.items()
                if marker.lower() == marker_text.lower()
            )
            end = matches[index + 1].start() if index + 1 < len(matches) else len(line)
            segment = line[match.end() : end]
            ids = extract_relation_ids(segment)
            if ids:
                relations.setdefault(canonical, [])
                relations[canonical] = unique([*relations[canonical], *ids])
    return relations


def extract_relation_ids(segment: str) -> list[str]:
    expanded = expand_id_ranges(segment)
    return [
        module_id
        for module_id in unique(ID_RE.findall(expanded))
        if not module_id.endswith(":End") and ":QF" not in module_id
    ]


def expand_id_ranges(segment: str) -> str:
    segment = re.sub(
        r"\b([A-K])\.(\d+)\.(\d+)-\1\.\2\.(\d+)\b",
        lambda match: " ".join(
            f"{match.group(1)}.{match.group(2)}.{number}"
            for number in bounded_range(int(match.group(3)), int(match.group(4)))
        ),
        segment,
    )
    segment = re.sub(
        r"\b([A-K])\.(\d+)-\1\.(\d+)\b",
        lambda match: " ".join(
            f"{match.group(1)}.{number}"
            for number in bounded_range(int(match.group(2)), int(match.group(3)))
        ),
        segment,
    )
    return segment


def bounded_range(start: int, end: int) -> range:
    if end < start or end - start > 25:
        return range(start, start + 1)
    return range(start, end + 1)


def extract_keywords(text: str) -> list[str]:
    keywords: list[str] = []
    for match in re.finditer(r"(?i)Keywords:\s*(.+?)(?:Queries:|$)", clean_markdown(text)):
        for token in re.split(r"[,;]", match.group(1)):
            token = token.strip(" .:*`\"'()[]{}")
            if token and len(token) <= 80:
                keywords.append(token)
    return unique(keywords)


def extract_queries(text: str) -> list[str]:
    queries: list[str] = []
    for query in re.findall(r'"([^"]{8,240})"', text):
        query = clean_markdown(query).strip()
        if query.endswith("?") or query.lower().startswith(("what ", "how ", "when ", "where ", "why ")):
            queries.append(query)
    return unique(queries)


def merge_relations(dest: dict[str, list[str]], source: object) -> None:
    if not isinstance(source, dict):
        return
    for kind, targets in source.items():
        if isinstance(kind, str) and isinstance(targets, list):
            dest.setdefault(kind, [])
            dest[kind] = unique([*dest[kind], *[str(target) for target in targets]])


def module_rel_path(section: Section, prefix: str = "modules", suffix: str = ".md") -> str:
    part = safe_filename(section.part)
    return f"{prefix}/{part}/{safe_filename(section.id)}{suffix}"


def verify_source_spans(text: str, sections: Iterable[Section], out_dir: Path) -> None:
    source_lines = text.splitlines()
    for section in sections:
        module_path = out_dir / section.path
        module_text = module_path.read_text(encoding="utf-8")
        body = module_text.split("---\n\n", 1)[1]
        expected = "\n".join(source_lines[section.start_line - 1 : section.end_line]) + "\n"
        if body != expected:
            raise RuntimeError(f"source span verification failed for {section.id}")


def adjacency_yaml(edges: list[dict[str, object]]) -> str:
    by_from: dict[str, dict[str, list[str]]] = {}
    for edge in edges:
        by_from.setdefault(str(edge["from"]), {}).setdefault(str(edge["kind"]), []).append(str(edge["to"]))
    return to_yaml({"adjacency": by_from})


def glossary_yaml(query_rows: list[dict[str, object]]) -> str:
    terms: dict[str, list[str]] = {}
    for row in query_rows:
        module_id = str(row["id"])
        title = str(row["title"])
        raw_terms = [*title_terms(title), *[str(item) for item in row.get("keywords", [])]]
        for raw_term in raw_terms:
            term = raw_term.lower().strip(" .,:;*`\"'()[]{}<>")
            if (
                len(term) < 3
                or len(term) > 60
                or not term[0].isalnum()
                or not any(ch.isalnum() for ch in term)
            ):
                continue
            terms.setdefault(term, [])
            if module_id not in terms[term] and len(terms[term]) < 12:
                terms[term].append(module_id)
    compact = {term: terms[term] for term in sorted(terms)}
    return to_yaml({"schema": "s5d.fpf.glossary/0.1", "terms": compact})


def manifest_yaml(
    source_name: str,
    source_hash: str,
    text: str,
    sections: list[Section],
    edges: list[dict[str, object]],
) -> str:
    data = {
        "schema": SCHEMA,
        "source": {
            "path": f"source/{source_name}",
            "sha256": source_hash,
            "bytes": len(text.encode("utf-8")),
            "lines": len(text.splitlines()),
            "token_estimate": estimate_tokens(text),
        },
        "artifacts": {
            "modules": len(sections),
            "edges": len(edges),
            "cards": len(sections),
        },
        "canonicality": {
            "source": "canonical",
            "modules": "lossless_section_views",
            "cards": "navigation_only",
            "graph": "inferred_from_relation_markers",
        },
    }
    return to_yaml(data)


def entrypoints_yaml(node_ids: set[str]) -> str:
    candidates = [
        ("problem-framing", ["problem", "anomaly", "hypothesis", "acceptance"], ["E.10", "G.0", "A.1.1"]),
        ("bounded-context", ["domain", "context", "meaning", "ambiguity"], ["A.1.1", "E.10", "F.1"]),
        ("multi-view", ["view", "publication", "dashboard", "projection"], ["E.17.0", "E.17", "A.6.3"]),
        ("evidence", ["evidence", "traceability", "assurance", "provenance"], ["A.10", "B.3", "G.6"]),
        ("comparison", ["compare", "characteristic", "pareto", "selection"], ["A.18", "A.19", "G.0", "G.5"]),
        ("unification", ["term", "bridge", "glossary", "alignment"], ["F.17", "F.9", "A.6.9"]),
    ]
    rows = []
    for entry_id, triggers, modules in candidates:
        rows.append(
            {
                "id": entry_id,
                "triggers": triggers,
                "load_cards": [module for module in modules if module in node_ids],
                "expand": ["builds_on", "refines", "coordinates_with"],
            }
        )
    return to_yaml({"schema": "s5d.fpf.entrypoints/0.1", "entrypoints": rows})


def load_policy_md() -> str:
    return """# FPF Agent Load Policy

Goal: use FPF without loading the full source.

1. Start with `agent/entrypoints.yaml`, `agent/glossary.yaml`, and `agent/query-index.jsonl`.
2. Select 1-5 candidate cards by trigger, keyword, title, or query match.
3. Read only matching `cards/**/*.card.yaml`.
4. Load a full `modules/**/*.md` file only when exact wording, checklist, or rationale is needed.
5. Expand graph one hop at a time through `builds_on`, `refines`, and `coordinates_with`.
6. Treat cards as navigation. Treat modules as evidence. Treat `source/FPF-Spec.md` as canonical.
7. Cite module id plus source span when using FPF in a decision.
"""


def corpus_readme(module_count: int, edge_count: int) -> str:
    return f"""# FPF Modular Corpus

Generated agent-readable corpus for FPF.

- Modules: {module_count}
- Relation edges: {edge_count}

Use `agent/load-policy.md` first. Cards are navigation-only; module files preserve
the original section text; `source/FPF-Spec.md` remains canonical.

Agent bootstrap files:

- `agent/entrypoints.yaml` - curated route hints for common FPF tasks
- `agent/glossary.yaml` - compact term to module-id map
- `agent/query-index.jsonl` - one compact search row per module
"""


def to_yaml(value: object, indent: int = 0) -> str:
    pad = " " * indent
    if isinstance(value, dict):
        lines: list[str] = []
        for key, item in value.items():
            if item is None:
                lines.append(f"{pad}{key}: null")
            elif isinstance(item, (dict, list)):
                lines.append(f"{pad}{key}:")
                lines.append(to_yaml(item, indent + 2).rstrip())
            else:
                lines.append(f"{pad}{key}: {yaml_scalar(item)}")
        return "\n".join(lines) + "\n"
    if isinstance(value, list):
        if not value:
            return f"{pad}[]\n"
        lines = []
        for item in value:
            if isinstance(item, (dict, list)):
                lines.append(f"{pad}-")
                lines.append(to_yaml(item, indent + 2).rstrip())
            else:
                lines.append(f"{pad}- {yaml_scalar(item)}")
        return "\n".join(lines) + "\n"
    return f"{pad}{yaml_scalar(value)}\n"


def yaml_scalar(value: object) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return str(value)
    return json.dumps(str(value), ensure_ascii=False)


def write_jsonl(path: Path, rows: Iterable[dict[str, object]]) -> None:
    write_text(path, "".join(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n" for row in rows))


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def clean_markdown(text: str) -> str:
    return (
        text.replace("**", "")
        .replace("*", "")
        .replace("`", "")
        .replace("&nbsp;", " ")
        .replace("<br>", " ")
        .replace("<br/>", " ")
        .replace("<br />", " ")
    )


def unique(items: Iterable[str]) -> list[str]:
    seen: set[str] = set()
    out: list[str] = []
    for item in items:
        item = str(item).strip()
        if not item or item in seen:
            continue
        seen.add(item)
        out.append(item)
    return out


def title_terms(title: str) -> list[str]:
    stop = {"the", "and", "for", "with", "from", "into", "this", "that"}
    return [
        term.lower()
        for term in re.findall(r"[A-Za-z][A-Za-z0-9-]{2,}", title)
        if term.lower() not in stop
    ]


def first_paragraph(body: str) -> str:
    paragraphs = [p.strip() for p in body.split("\n\n") if p.strip()]
    for paragraph in paragraphs[1:]:
        clean = clean_markdown(paragraph)
        if clean.startswith(("#", "|", ":---")):
            continue
        return " ".join(clean.split())[:800]
    return ""


def safe_filename(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9._-]+", "_", value).strip("._") or "module"


def slugify(value: str) -> str:
    slug = re.sub(r"[^A-Za-z0-9]+", "-", value.lower()).strip("-")
    return slug or "section"


def estimate_tokens(text: str) -> int:
    return max(1, len(text.encode("utf-8")) // 4)


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


if __name__ == "__main__":
    main()
