use std::collections::HashMap;

use crate::codebase::storage::CodebaseIndex;

pub struct ImportGraph {
    edges: Vec<(String, String)>,
}

impl ImportGraph {
    pub fn from_index(index: &CodebaseIndex) -> anyhow::Result<Self> {
        let edges_raw = index.get_import_edges()?;
        let edges = edges_raw
            .into_iter()
            .map(|e| (e.source_file, e.imported))
            .collect();
        Ok(ImportGraph { edges })
    }

    pub fn pagerank(&self, damping: f64, iterations: usize) -> HashMap<String, f64> {
        // Collect all unique nodes
        let mut nodes: Vec<String> = Vec::new();
        for (src, dst) in &self.edges {
            if !nodes.contains(src) {
                nodes.push(src.clone());
            }
            if !nodes.contains(dst) {
                nodes.push(dst.clone());
            }
        }
        let n = nodes.len();
        if n == 0 {
            return HashMap::new();
        }

        let node_idx: HashMap<&str, usize> = nodes
            .iter()
            .enumerate()
            .map(|(i, s)| (s.as_str(), i))
            .collect();

        // outgoing edge counts
        let mut out_count = vec![0usize; n];
        for (src, _) in &self.edges {
            if let Some(&i) = node_idx.get(src.as_str()) {
                out_count[i] += 1;
            }
        }

        let init = 1.0 / n as f64;
        let mut rank = vec![init; n];

        for _ in 0..iterations {
            let mut new_rank = vec![(1.0 - damping) / n as f64; n];
            for (src, dst) in &self.edges {
                let si = match node_idx.get(src.as_str()) {
                    Some(&i) => i,
                    None => continue,
                };
                let di = match node_idx.get(dst.as_str()) {
                    Some(&i) => i,
                    None => continue,
                };
                if out_count[si] > 0 {
                    new_rank[di] += damping * rank[si] / out_count[si] as f64;
                }
            }
            rank = new_rank;
        }

        nodes
            .into_iter()
            .enumerate()
            .map(|(i, n)| (n, rank[i]))
            .collect()
    }

    pub fn neighbors(&self, file: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|(src, _)| src == file)
            .map(|(_, dst)| dst.clone())
            .collect()
    }
}

