use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::*;

pub struct S5dProject {
    pub root: PathBuf,
}

pub struct InitReport {
    pub root: PathBuf,
    pub dirs_created: Vec<PathBuf>,
    pub files_created: Vec<PathBuf>,
}

pub struct ProjectLock {
    path: PathBuf,
}

impl Drop for ProjectLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
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

        for dir in &["packages", "records", "tasks", "harness", ".locks"] {
            let path = s5d.join(dir);
            fs::create_dir_all(&path)?;
            report.dirs_created.push(path);
        }

        let config = S5dConfig {
            schema: "1.0".into(),
            gate_commands: Default::default(),
            engines: Default::default(),
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
        fs::create_dir_all(s5d.join("tasks"))?;
        fs::create_dir_all(s5d.join("harness"))?;
        fs::create_dir_all(s5d.join(".locks"))?;
        Ok(())
    }

    pub fn save_task_artifact(
        &self,
        spec_filename: &str,
        phase_id: &str,
        mode: &str,
        content: &str,
    ) -> anyhow::Result<PathBuf> {
        let tasks_dir = self.s5d_dir().join("tasks");
        fs::create_dir_all(&tasks_dir)?;

        let spec_stem = spec_filename
            .strip_suffix(".s5d.yaml")
            .unwrap_or(spec_filename);
        let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
        let task_name = format!(
            "{}__{}__{}__{}.ralph.md",
            spec_stem, phase_id, mode, timestamp
        );
        let path = tasks_dir.join(task_name);
        atomic_write_string(&path, content)?;
        Ok(path)
    }

    pub fn acquire_lock(&self, name: &str) -> anyhow::Result<ProjectLock> {
        let locks_dir = self.s5d_dir().join(".locks");
        fs::create_dir_all(&locks_dir)?;
        let safe = sanitize_lock_name(name);
        let path = locks_dir.join(format!("{}.lock", safe));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|err| anyhow::anyhow!("failed to acquire lock '{}': {}", safe, err))?;
        writeln!(
            file,
            "pid={}\nstarted_at={}",
            std::process::id(),
            Utc::now().to_rfc3339()
        )?;
        file.sync_all()?;
        Ok(ProjectLock { path })
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
        atomic_write_string(&path, &serde_yaml::to_string(record)?)?;
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
        atomic_write_string(&path, &serde_yaml::to_string(ledger)?)?;
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
        atomic_write_string(&path, &serde_yaml::to_string(index)?)?;
        Ok(())
    }

    pub fn load_config(&self) -> anyhow::Result<S5dConfig> {
        let path = self.s5d_dir().join("config.yaml");
        let content = fs::read_to_string(&path)?;
        Ok(serde_yaml::from_str(&content)?)
    }
}

pub fn atomic_write_string(path: &Path, content: &str) -> anyhow::Result<()> {
    atomic_write_bytes(path, content.as_bytes())
}

pub fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "cannot atomically write path without parent: {}",
            path.display()
        )
    })?;
    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("cannot determine filename for {}", path.display()))?;
    let tmp_name = format!(
        ".{}.tmp.{}.{}",
        file_name,
        std::process::id(),
        Uuid::new_v4()
    );
    let tmp_path = parent.join(tmp_name);

    let mut file = File::create(&tmp_path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);

    fs::rename(&tmp_path, path)?;

    if let Ok(dir_handle) = File::open(parent) {
        let _ = dir_handle.sync_all();
    }

    Ok(())
}

fn sanitize_lock_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "default".into()
    } else {
        sanitized
    }
}
