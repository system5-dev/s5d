use crate::project::atomic_write_string;
use crate::{Artifacts, Links, S5dProject};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoverySnapshot {
    pub manifest: DiscoveryManifest,
    pub files: Vec<DiscoveryFile>,
    pub graph: DiscoveryGraph,
    pub evidence: Vec<DiscoveryEvidence>,
    pub metamodel: DiscoveryMetamodel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryManifest {
    pub schema: String,
    pub root: String,
    pub target: String,
    pub files: usize,
    pub nodes: usize,
    pub edges: usize,
    pub evidence: usize,
    pub artifacts: DiscoveryArtifactPaths,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryArtifactPaths {
    pub files: String,
    pub graph: String,
    pub evidence: String,
    pub metamodel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryFile {
    pub id: String,
    pub path: String,
    pub kind: String,
    pub language: String,
    pub bytes: u64,
    pub lines: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryGraph {
    pub schema: String,
    pub nodes: Vec<DiscoveryNode>,
    pub edges: Vec<DiscoveryEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryNode {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub status: DiscoveryClaimStatus,
    pub confidence: f64,
    #[serde(default)]
    pub provenance: Vec<DiscoveryProvenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub status: DiscoveryClaimStatus,
    pub confidence: f64,
    #[serde(default)]
    pub provenance: Vec<DiscoveryProvenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryEvidence {
    pub id: String,
    pub claim: String,
    pub status: DiscoveryClaimStatus,
    pub confidence: f64,
    pub evidence: DiscoveryProvenance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryProvenance {
    pub path: String,
    #[serde(default)]
    pub line: Option<usize>,
    pub kind: String,
    #[serde(default)]
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryClaimStatus {
    Verified,
    Inferred,
    Speculative,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryMetamodel {
    pub schema: String,
    pub source_graph: String,
    pub products: Vec<DiscoveryClaimView>,
    pub domains: Vec<DiscoveryClaimView>,
    pub capabilities: Vec<DiscoveryClaimView>,
    pub entities: Vec<DiscoveryClaimView>,
    pub features: Vec<DiscoveryClaimView>,
    pub use_cases: Vec<DiscoveryClaimView>,
    pub systems: Vec<DiscoveryClaimView>,
    pub containers: Vec<DiscoveryClaimView>,
    pub components: Vec<DiscoveryClaimView>,
    pub projections: Vec<DiscoveryClaimView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveryClaimView {
    pub id: String,
    pub name: String,
    pub status: DiscoveryClaimStatus,
    pub confidence: f64,
    #[serde(default)]
    pub evidence: Vec<DiscoveryProvenance>,
}

pub fn build_discovery_snapshot(
    project: &S5dProject,
    target: &Path,
) -> anyhow::Result<DiscoverySnapshot> {
    let project_root = project.root.canonicalize()?;
    let target = if target.is_absolute() {
        target.to_path_buf()
    } else {
        project.root.join(target)
    }
    .canonicalize()?;

    if !target.starts_with(&project_root) {
        anyhow::bail!(
            "discovery target {} is outside project root {}",
            target.display(),
            project_root.display()
        );
    }

    let mut files = Vec::new();
    collect_project_files(&project_root, &target, &mut files)?;
    add_spec_files(project, &project_root, &mut files)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    files.dedup_by(|left, right| left.path == right.path);

    let mut builder = DiscoveryGraphBuilder::default();
    for file in &files {
        builder.add_node(DiscoveryNode {
            id: file.id.clone(),
            kind: "file".into(),
            label: file.path.clone(),
            status: DiscoveryClaimStatus::Verified,
            confidence: 1.0,
            provenance: vec![DiscoveryProvenance {
                path: file.path.clone(),
                line: None,
                kind: file.kind.clone(),
                symbol: None,
            }],
        });
    }

    add_inferred_components(&mut builder, &files);
    add_spec_artifacts(project, &project_root, &mut builder)?;

    let graph = builder.into_graph();
    let evidence = build_evidence(&graph);
    let metamodel = build_metamodel_projection(&graph);
    let manifest = DiscoveryManifest {
        schema: "s5d.discovery/1.0".into(),
        root: ".".into(),
        target: display_rel(&project_root, &target),
        files: files.len(),
        nodes: graph.nodes.len(),
        edges: graph.edges.len(),
        evidence: evidence.len(),
        artifacts: DiscoveryArtifactPaths {
            files: "files.jsonl".into(),
            graph: "graph.json".into(),
            evidence: "evidence.jsonl".into(),
            metamodel: "metamodel.yaml".into(),
        },
    };

    Ok(DiscoverySnapshot {
        manifest,
        files,
        graph,
        evidence,
        metamodel,
    })
}

pub fn write_discovery_snapshot(
    project: &S5dProject,
    out_dir: &Path,
    snapshot: &DiscoverySnapshot,
) -> anyhow::Result<()> {
    let out_dir = if out_dir.is_absolute() {
        out_dir.to_path_buf()
    } else {
        project.root.join(out_dir)
    };
    std::fs::create_dir_all(&out_dir)?;

    atomic_write_string(
        &out_dir.join("manifest.yaml"),
        &serde_yaml::to_string(&snapshot.manifest)?,
    )?;
    atomic_write_string(&out_dir.join("files.jsonl"), &to_jsonl(&snapshot.files)?)?;
    atomic_write_string(
        &out_dir.join("graph.json"),
        &serde_json::to_string_pretty(&snapshot.graph)?,
    )?;
    atomic_write_string(
        &out_dir.join("evidence.jsonl"),
        &to_jsonl(&snapshot.evidence)?,
    )?;
    atomic_write_string(
        &out_dir.join("metamodel.yaml"),
        &serde_yaml::to_string(&snapshot.metamodel)?,
    )?;
    Ok(())
}

pub fn read_discovery_snapshot(out_dir: &Path) -> anyhow::Result<Option<DiscoverySnapshot>> {
    let manifest_path = out_dir.join("manifest.yaml");
    let files_path = out_dir.join("files.jsonl");
    let graph_path = out_dir.join("graph.json");
    let evidence_path = out_dir.join("evidence.jsonl");
    let metamodel_path = out_dir.join("metamodel.yaml");
    if !manifest_path.exists()
        || !files_path.exists()
        || !graph_path.exists()
        || !evidence_path.exists()
        || !metamodel_path.exists()
    {
        return Ok(None);
    }

    Ok(Some(DiscoverySnapshot {
        manifest: serde_yaml::from_str(&std::fs::read_to_string(manifest_path)?)?,
        files: from_jsonl(&std::fs::read_to_string(files_path)?)?,
        graph: serde_json::from_str(&std::fs::read_to_string(graph_path)?)?,
        evidence: from_jsonl(&std::fs::read_to_string(evidence_path)?)?,
        metamodel: serde_yaml::from_str(&std::fs::read_to_string(metamodel_path)?)?,
    }))
}

#[derive(Default)]
struct DiscoveryGraphBuilder {
    nodes: BTreeMap<String, DiscoveryNode>,
    edges: BTreeMap<(String, String, String), DiscoveryEdge>,
}

impl DiscoveryGraphBuilder {
    fn add_node(&mut self, node: DiscoveryNode) {
        self.nodes.entry(node.id.clone()).or_insert(node);
    }

    fn add_edge(&mut self, edge: DiscoveryEdge) {
        self.edges
            .entry((edge.from.clone(), edge.to.clone(), edge.kind.clone()))
            .or_insert(edge);
    }

    fn into_graph(self) -> DiscoveryGraph {
        DiscoveryGraph {
            schema: "s5d.discovery.graph/1.0".into(),
            nodes: self.nodes.into_values().collect(),
            edges: self.edges.into_values().collect(),
        }
    }
}

fn collect_project_files(
    project_root: &Path,
    path: &Path,
    files: &mut Vec<DiscoveryFile>,
) -> anyhow::Result<()> {
    let mut walker = WalkBuilder::new(path);
    walker
        .hidden(false)
        .ignore(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .parents(true)
        .filter_entry(|entry| !should_skip_dir(entry.path()));
    if !project_root.join(".git").exists() {
        walker.add_custom_ignore_filename(".gitignore");
    }

    for entry in walker.build() {
        let entry = entry?;
        let entry_path = entry.path();
        let is_file = entry
            .file_type()
            .map(|file_type| file_type.is_file())
            .unwrap_or_else(|| entry_path.is_file());
        if is_file && is_discovery_file(entry_path) {
            files.push(build_file(project_root, entry_path)?);
        }
    }
    Ok(())
}

fn add_spec_files(
    project: &S5dProject,
    project_root: &Path,
    files: &mut Vec<DiscoveryFile>,
) -> anyhow::Result<()> {
    for (path, _) in project.discover_specs()? {
        files.push(build_file(project_root, &path)?);
    }
    Ok(())
}

fn build_file(project_root: &Path, path: &Path) -> anyhow::Result<DiscoveryFile> {
    let bytes = std::fs::read(path)?;
    let path_rel = display_rel(project_root, path);
    Ok(DiscoveryFile {
        id: format!("file:{}", path_rel),
        path: path_rel,
        kind: file_kind(path),
        language: file_language(path),
        bytes: bytes.len() as u64,
        lines: count_lines(&bytes),
        sha256: sha256_hex(&bytes),
    })
}

fn add_inferred_components(builder: &mut DiscoveryGraphBuilder, files: &[DiscoveryFile]) {
    for file in files.iter().filter(|file| file.kind == "source") {
        let component_path = component_candidate_path(&file.path);
        let component_id = format!("component_candidate:{}", component_path);
        let provenance = DiscoveryProvenance {
            path: file.path.clone(),
            line: None,
            kind: "path".into(),
            symbol: None,
        };
        builder.add_node(DiscoveryNode {
            id: component_id.clone(),
            kind: "component".into(),
            label: component_path,
            status: DiscoveryClaimStatus::Inferred,
            confidence: 0.55,
            provenance: vec![provenance.clone()],
        });
        builder.add_edge(DiscoveryEdge {
            from: component_id,
            to: file.id.clone(),
            kind: "contains_file".into(),
            status: DiscoveryClaimStatus::Verified,
            confidence: 1.0,
            provenance: vec![provenance],
        });
    }
}

fn add_spec_artifacts(
    project: &S5dProject,
    project_root: &Path,
    builder: &mut DiscoveryGraphBuilder,
) -> anyhow::Result<()> {
    for (path, spec) in project.discover_specs()? {
        let spec_path = display_rel(project_root, &path);
        let spec_file = format!("file:{}", spec_path);
        let spec_content = std::fs::read_to_string(&path)?;
        let spec_node = format!("spec:{}", spec.id);
        let spec_provenance = DiscoveryProvenance {
            path: spec_path.clone(),
            line: Some(1),
            kind: "spec".into(),
            symbol: Some(spec.id.clone()),
        };
        builder.add_node(DiscoveryNode {
            id: spec_node.clone(),
            kind: "spec".into(),
            label: spec.id.clone(),
            status: DiscoveryClaimStatus::Verified,
            confidence: 1.0,
            provenance: vec![spec_provenance.clone()],
        });
        builder.add_edge(DiscoveryEdge {
            from: spec_file,
            to: spec_node.clone(),
            kind: "defines".into(),
            status: DiscoveryClaimStatus::Verified,
            confidence: 1.0,
            provenance: vec![spec_provenance.clone()],
        });

        let Some(artifacts) = spec.artifacts.as_ref() else {
            continue;
        };
        add_artifact_nodes(builder, &spec_node, &spec_path, &spec_content, artifacts);
        if let Some(links) = spec.links.as_ref() {
            add_spec_links(builder, &spec_path, &spec_content, links);
        }
    }
    Ok(())
}

fn add_artifact_nodes(
    builder: &mut DiscoveryGraphBuilder,
    spec_node: &str,
    spec_path: &str,
    spec_content: &str,
    artifacts: &Artifacts,
) {
    for product in &artifacts.products {
        add_declared_node(
            builder,
            spec_node,
            spec_path,
            spec_content,
            "product",
            &product.id,
            &product.name,
        );
    }
    for domain in &artifacts.domains {
        add_declared_node(
            builder,
            spec_node,
            spec_path,
            spec_content,
            "domain",
            &domain.id,
            &domain.name,
        );
        add_declared_edge(
            builder,
            &format!("product:{}", domain.product),
            &format!("domain:{}", domain.id),
            "owns_domain",
            spec_path,
            find_line(spec_content, &domain.id),
        );
    }
    for capability in &artifacts.capabilities {
        add_declared_node(
            builder,
            spec_node,
            spec_path,
            spec_content,
            "capability",
            &capability.id,
            &capability.name,
        );
        add_declared_edge(
            builder,
            &format!("domain:{}", capability.domain),
            &format!("capability:{}", capability.id),
            "owns_capability",
            spec_path,
            find_line(spec_content, &capability.id),
        );
    }
    for entity in &artifacts.entities {
        add_declared_node(
            builder,
            spec_node,
            spec_path,
            spec_content,
            "entity",
            &entity.id,
            &entity.name,
        );
        add_declared_edge(
            builder,
            &format!("domain:{}", entity.domain),
            &format!("entity:{}", entity.id),
            "owns_entity",
            spec_path,
            find_line(spec_content, &entity.id),
        );
    }
    for feature in &artifacts.features {
        add_declared_node(
            builder,
            spec_node,
            spec_path,
            spec_content,
            "feature",
            &feature.id,
            &feature.name,
        );
        add_declared_edge(
            builder,
            &format!("product:{}", feature.product),
            &format!("feature:{}", feature.id),
            "offers_feature",
            spec_path,
            find_line(spec_content, &feature.id),
        );
    }
    for use_case in &artifacts.use_cases {
        add_declared_node(
            builder,
            spec_node,
            spec_path,
            spec_content,
            "use_case",
            &use_case.id,
            &use_case.name,
        );
        add_declared_edge(
            builder,
            &format!("feature:{}", use_case.feature),
            &format!("use_case:{}", use_case.id),
            "has_use_case",
            spec_path,
            find_line(spec_content, &use_case.id),
        );
    }
    for system in &artifacts.systems {
        add_declared_node(
            builder,
            spec_node,
            spec_path,
            spec_content,
            "system",
            &system.id,
            &system.name,
        );
        add_declared_edge(
            builder,
            &format!("product:{}", system.product),
            &format!("system:{}", system.id),
            "owns_system",
            spec_path,
            find_line(spec_content, &system.id),
        );
    }
    for container in &artifacts.containers {
        add_declared_node(
            builder,
            spec_node,
            spec_path,
            spec_content,
            "container",
            &container.id,
            &container.name,
        );
        add_declared_edge(
            builder,
            &format!("system:{}", container.system),
            &format!("container:{}", container.id),
            "has_container",
            spec_path,
            find_line(spec_content, &container.id),
        );
    }
    for component in &artifacts.components {
        add_declared_node(
            builder,
            spec_node,
            spec_path,
            spec_content,
            "component",
            &component.id,
            &component.name,
        );
        add_declared_edge(
            builder,
            &format!("feature:{}", component.feature),
            &format!("component:{}", component.id),
            "implemented_by",
            spec_path,
            find_line(spec_content, &component.id),
        );
        add_declared_edge(
            builder,
            &format!("domain:{}", component.domain),
            &format!("component:{}", component.id),
            "owns_component",
            spec_path,
            find_line(spec_content, &component.id),
        );
        add_declared_edge(
            builder,
            &format!("container:{}", component.container),
            &format!("component:{}", component.id),
            "contains_component",
            spec_path,
            find_line(spec_content, &component.id),
        );
        for component_path in &component.paths {
            add_declared_path_node(
                builder,
                spec_path,
                component_path,
                find_line(spec_content, component_path),
            );
            add_declared_edge(
                builder,
                &format!("component:{}", component.id),
                &format!("path:{}", component_path),
                "claims_path",
                spec_path,
                find_line(spec_content, component_path),
            );
        }
    }
}

fn add_spec_links(
    builder: &mut DiscoveryGraphBuilder,
    spec_path: &str,
    spec_content: &str,
    links: &Links,
) {
    for relation in links
        .entity_relations
        .iter()
        .filter(|relation| relation.projection)
    {
        add_declared_edge(
            builder,
            &format!("entity:{}", relation.entity),
            &format!("entity:{}", relation.related_entity),
            "projects_to",
            spec_path,
            find_line(spec_content, &relation.entity),
        );
    }
}

fn add_declared_node(
    builder: &mut DiscoveryGraphBuilder,
    spec_node: &str,
    spec_path: &str,
    spec_content: &str,
    kind: &str,
    id: &str,
    label: &str,
) {
    let provenance = DiscoveryProvenance {
        path: spec_path.into(),
        line: find_line(spec_content, id),
        kind: "spec".into(),
        symbol: Some(id.into()),
    };
    let node_id = format!("{}:{}", kind, id);
    builder.add_node(DiscoveryNode {
        id: node_id.clone(),
        kind: kind.into(),
        label: label.into(),
        status: DiscoveryClaimStatus::Verified,
        confidence: 1.0,
        provenance: vec![provenance.clone()],
    });
    builder.add_edge(DiscoveryEdge {
        from: spec_node.into(),
        to: node_id,
        kind: "declares".into(),
        status: DiscoveryClaimStatus::Verified,
        confidence: 1.0,
        provenance: vec![provenance],
    });
}

fn add_declared_path_node(
    builder: &mut DiscoveryGraphBuilder,
    spec_path: &str,
    path_pattern: &str,
    line: Option<usize>,
) {
    builder.add_node(DiscoveryNode {
        id: format!("path:{}", path_pattern),
        kind: "path".into(),
        label: path_pattern.into(),
        status: DiscoveryClaimStatus::Verified,
        confidence: 1.0,
        provenance: vec![DiscoveryProvenance {
            path: spec_path.into(),
            line,
            kind: "spec".into(),
            symbol: Some(path_pattern.into()),
        }],
    });
}

fn add_declared_edge(
    builder: &mut DiscoveryGraphBuilder,
    from: &str,
    to: &str,
    kind: &str,
    path: &str,
    line: Option<usize>,
) {
    builder.add_edge(DiscoveryEdge {
        from: from.into(),
        to: to.into(),
        kind: kind.into(),
        status: DiscoveryClaimStatus::Verified,
        confidence: 1.0,
        provenance: vec![DiscoveryProvenance {
            path: path.into(),
            line,
            kind: "spec".into(),
            symbol: None,
        }],
    });
}

fn build_evidence(graph: &DiscoveryGraph) -> Vec<DiscoveryEvidence> {
    let mut evidence = Vec::new();
    for node in &graph.nodes {
        for provenance in &node.provenance {
            evidence.push(DiscoveryEvidence {
                id: format!("evidence:{}:{}", node.kind, node.id),
                claim: format!("{} {} exists", node.kind, node.id),
                status: node.status.clone(),
                confidence: node.confidence,
                evidence: provenance.clone(),
            });
        }
    }
    evidence.sort_by(|left, right| left.id.cmp(&right.id));
    evidence
}

fn build_metamodel_projection(graph: &DiscoveryGraph) -> DiscoveryMetamodel {
    DiscoveryMetamodel {
        schema: "s5d.discovery.metamodel/1.0".into(),
        source_graph: "graph.json".into(),
        products: claim_views(graph, "product"),
        domains: claim_views(graph, "domain"),
        capabilities: claim_views(graph, "capability"),
        entities: claim_views(graph, "entity"),
        features: claim_views(graph, "feature"),
        use_cases: claim_views(graph, "use_case"),
        systems: claim_views(graph, "system"),
        containers: claim_views(graph, "container"),
        components: claim_views(graph, "component"),
        projections: graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "projects_to")
            .map(|edge| DiscoveryClaimView {
                id: edge.to.clone(),
                name: edge.to.clone(),
                status: edge.status.clone(),
                confidence: edge.confidence,
                evidence: edge.provenance.clone(),
            })
            .collect(),
    }
}

fn claim_views(graph: &DiscoveryGraph, kind: &str) -> Vec<DiscoveryClaimView> {
    graph
        .nodes
        .iter()
        .filter(|node| node.kind == kind)
        .map(|node| DiscoveryClaimView {
            id: node.id.clone(),
            name: node.label.clone(),
            status: node.status.clone(),
            confidence: node.confidence,
            evidence: node.provenance.clone(),
        })
        .collect()
}

fn to_jsonl<T: Serialize>(items: &[T]) -> anyhow::Result<String> {
    let mut out = String::new();
    for item in items {
        out.push_str(&serde_json::to_string(item)?);
        out.push('\n');
    }
    Ok(out)
}

fn from_jsonl<T: for<'de> Deserialize<'de>>(content: &str) -> anyhow::Result<Vec<T>> {
    let mut out = Vec::new();
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        out.push(serde_json::from_str(line)?);
    }
    Ok(out)
}

fn is_discovery_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(
            "rs" | "py"
                | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "go"
                | "java"
                | "kt"
                | "swift"
                | "c"
                | "cc"
                | "cpp"
                | "h"
                | "hpp"
                | "rb"
                | "php"
                | "cs"
                | "scala"
                | "sql"
                | "yaml"
                | "yml"
                | "toml"
                | "json"
                | "md"
        )
    )
}

fn should_skip_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name == ".git"
        || name == ".s5d"
        || name == "target"
        || name == "node_modules"
        || name == "dist"
        || name == "build"
        || name == "vendor"
        || name == "__pycache__"
}

fn file_kind(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    if path_str.contains("/.s5d/packages/") {
        "spec".into()
    } else {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("md") => "doc".into(),
            Some("yaml" | "yml" | "toml" | "json") => "config".into(),
            _ => "source".into(),
        }
    }
}

fn file_language(path: &Path) -> String {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => "Rust",
        Some("py") => "Python",
        Some("ts" | "tsx" | "js" | "jsx") => "JavaScript/TypeScript",
        Some("go") => "Go",
        Some("java") => "Java",
        Some("kt") => "Kotlin",
        Some("swift") => "Swift",
        Some("c" | "cc" | "cpp" | "h" | "hpp") => "C/C++",
        Some("rb") => "Ruby",
        Some("php") => "PHP",
        Some("cs") => "C#",
        Some("scala") => "Scala",
        Some("sql") => "SQL",
        Some("yaml" | "yml") => "YAML",
        Some("toml") => "TOML",
        Some("json") => "JSON",
        Some("md") => "Markdown",
        _ => "Unknown",
    }
    .into()
}

fn component_candidate_path(path: &str) -> String {
    let mut parts = path.split('/').collect::<Vec<_>>();
    if parts.len() <= 1 {
        return "root".into();
    }
    if matches!(
        parts.first(),
        Some(&"src" | &"lib" | &"app" | &"apps" | &"crates")
    ) {
        parts.truncate(parts.len().min(2));
    } else {
        parts.truncate(1);
    }
    parts.join("/")
}

fn count_lines(bytes: &[u8]) -> usize {
    if bytes.is_empty() {
        0
    } else {
        bytes.iter().filter(|byte| **byte == b'\n').count() + 1
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn find_line(content: &str, needle: &str) -> Option<usize> {
    content
        .lines()
        .position(|line| line.contains(needle))
        .map(|index| index + 1)
}

fn display_rel(project_root: &Path, path: &Path) -> String {
    let rel = path
        .strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    if rel.is_empty() {
        ".".into()
    } else {
        rel
    }
}
