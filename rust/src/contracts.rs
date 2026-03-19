use crate::models::*;
use sha2::{Digest, Sha256};
use std::path::Path;

pub fn check_contracts(spec: &Spec, project_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();

    for contract in &spec.contracts {
        if contract.path.is_none() && contract.inline.is_none() {
            errors.push(format!(
                "contract {}: neither path nor inline provided",
                contract.id
            ));
            continue;
        }

        if let Some(ref path_str) = contract.path {
            let expected_sha = match &contract.sha256 {
                Some(s) => s,
                None => {
                    errors.push(format!(
                        "contract {}: path ref requires sha256",
                        contract.id
                    ));
                    continue;
                }
            };

            let contract_path = project_root.join(path_str);
            // Boundary check: path must stay within project root
            match (contract_path.canonicalize(), project_root.canonicalize()) {
                (Ok(canonical), Ok(root_canonical)) => {
                    if !canonical.starts_with(&root_canonical) {
                        errors.push(format!(
                            "contract {}: path '{}' escapes project root",
                            contract.id, path_str
                        ));
                        continue;
                    }
                }
                _ => {
                    // If canonicalize fails, the file likely doesn't exist — handle below
                }
            }
            if !contract_path.exists() {
                errors.push(format!(
                    "contract {}: file not found: {}",
                    contract.id, path_str
                ));
                continue;
            }

            match std::fs::read(&contract_path) {
                Ok(content) => {
                    let mut hasher = Sha256::new();
                    hasher.update(&content);
                    let actual = format!("sha256:{:x}", hasher.finalize());
                    if actual != *expected_sha {
                        errors.push(format!(
                            "contract {}: sha256 mismatch\n    expected: {}\n    actual:   {}",
                            contract.id, expected_sha, actual
                        ));
                    }
                }
                Err(e) => {
                    errors.push(format!("contract {}: read error: {}", contract.id, e));
                }
            }
        }

        let valid_formats = ["openapi", "json_schema", "protobuf", "typespec"];
        if !valid_formats.contains(&contract.format.as_str()) {
            errors.push(format!(
                "contract {}: unknown format '{}'",
                contract.id, contract.format
            ));
        }

        if contract.binds_to.is_empty() {
            errors.push(format!("contract {}: binds_to is empty", contract.id));
        }
    }

    errors
}
