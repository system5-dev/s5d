use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::models::*;

pub struct S5dProject {
    pub root: PathBuf,
}

pub struct InitReport {
    pub root: PathBuf,
    pub dirs_created: Vec<PathBuf>,
    pub files_created: Vec<PathBuf>,
}

impl S5dProject {
    pub fn find(from: &Path) -> Option<Self> {
        let mut current = from.to_path_buf();
        loop {
            if current.join(".s5d").is_dir() {
                return Some(Self { root: current });
            }
            if !current.pop() {
                return None;
            }
        }
    }

    pub fn init(at: &Path) -> anyhow::Result<(Self, InitReport)> {
        let s5d = at.join(".s5d");
        if s5d.exists() {
            // Idempotent: repair missing subdirs, return existing project
            let project = Self {
                root: at.to_path_buf(),
            };
            project.ensure_dirs()?;
            let report = InitReport {
                root: at.to_path_buf(),
                dirs_created: vec![],
                files_created: vec![],
            };
            return Ok((project, report));
        }

        let mut report = InitReport {
            root: at.to_path_buf(),
            dirs_created: vec![],
            files_created: vec![],
        };

        for dir in &["packages", "records", ".locks"] {
            let path = s5d.join(dir);
            fs::create_dir_all(&path)?;
            report.dirs_created.push(path);
        }

        let config = S5dConfig {
            schema: "1.0".into(),
            gate_commands: Default::default(),
            gate_runner: None,
            defaults: Some(Defaults {
                tier: Some("standard".into()),
                evidence_retention_days: Some(90),
            }),
        };
        let config_path = s5d.join("config.yaml");
        fs::write(&config_path, serde_yaml::to_string(&config)?)?;
        report.files_created.push(config_path);

        let ledger = Ledger {
            schema: "1.0".into(),
            entries: vec![],
        };
        let ledger_path = s5d.join("ledger.yaml");
        fs::write(&ledger_path, serde_yaml::to_string(&ledger)?)?;
        report.files_created.push(ledger_path);

        let index = Index { features: vec![] };
        let index_path = s5d.join("index.yaml");
        fs::write(&index_path, serde_yaml::to_string(&index)?)?;
        report.files_created.push(index_path);

        Ok((
            Self {
                root: at.to_path_buf(),
            },
            report,
        ))
    }

    pub fn s5d_dir(&self) -> PathBuf {
        self.root.join(".s5d")
    }

    /// Ensure all required subdirectories exist. Safe to call repeatedly.
    pub fn ensure_dirs(&self) -> anyhow::Result<()> {
        let s5d = self.s5d_dir();
        fs::create_dir_all(s5d.join("packages"))?;
        fs::create_dir_all(s5d.join("records"))?;
        fs::create_dir_all(s5d.join(".locks"))?;
        Ok(())
    }

    pub fn discover_specs(&self) -> anyhow::Result<Vec<(PathBuf, Spec)>> {
        let pattern = self.s5d_dir().join("packages").join("*.s5d.yaml");
        let pattern_str = pattern.to_string_lossy();
        let mut results = Vec::new();
        for entry in glob::glob(&pattern_str)? {
            let path = entry?;
            let content = fs::read_to_string(&path)?;
            let spec: Spec = serde_yaml::from_str(&content)?;
            results.push((path, spec));
        }
        Ok(results)
    }

    pub fn load_record(&self, spec_filename: &str) -> anyhow::Result<Option<Record>> {
        let record_name = spec_filename.replace(".s5d.yaml", ".record.yaml");
        let path = self.s5d_dir().join("records").join(&record_name);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        Ok(Some(serde_yaml::from_str(&content)?))
    }

    pub fn save_record(&self, spec_filename: &str, record: &Record) -> anyhow::Result<()> {
        let record_name = spec_filename.replace(".s5d.yaml", ".record.yaml");
        let path = self.s5d_dir().join("records").join(&record_name);
        fs::write(&path, serde_yaml::to_string(record)?)?;
        Ok(())
    }

    pub fn file_sha256(path: &Path) -> anyhow::Result<String> {
        let content = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(format!("sha256:{:x}", hasher.finalize()))
    }

    pub fn load_ledger(&self) -> anyhow::Result<Ledger> {
        let path = self.s5d_dir().join("ledger.yaml");
        if !path.exists() {
            return Ok(Ledger {
                schema: "1.0".into(),
                entries: vec![],
            });
        }
        let content = fs::read_to_string(&path)?;
        Ok(serde_yaml::from_str(&content)?)
    }

    pub fn save_ledger(&self, ledger: &Ledger) -> anyhow::Result<()> {
        let path = self.s5d_dir().join("ledger.yaml");
        fs::write(&path, serde_yaml::to_string(ledger)?)?;
        Ok(())
    }

    pub fn load_index(&self) -> anyhow::Result<Index> {
        let path = self.s5d_dir().join("index.yaml");
        if !path.exists() {
            return Ok(Index { features: vec![] });
        }
        let content = fs::read_to_string(&path)?;
        Ok(serde_yaml::from_str(&content)?)
    }

    pub fn save_index(&self, index: &Index) -> anyhow::Result<()> {
        let path = self.s5d_dir().join("index.yaml");
        fs::write(&path, serde_yaml::to_string(index)?)?;
        Ok(())
    }

    pub fn load_config(&self) -> anyhow::Result<S5dConfig> {
        let path = self.s5d_dir().join("config.yaml");
        let content = fs::read_to_string(&path)?;
        Ok(serde_yaml::from_str(&content)?)
    }
}
