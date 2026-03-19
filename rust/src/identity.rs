use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AliasTable {
    #[serde(default)]
    pub global: Vec<AliasEntry>,
    #[serde(default)]
    pub packages: Vec<AliasEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliasEntry {
    pub uuid: String,
    pub artifact_id: String,
    pub artifact_type: String,
    #[serde(default)]
    pub package_id: Option<String>,
    #[serde(default)]
    pub owning_package: Option<String>,
    #[serde(default)]
    pub deprecated: bool,
}

const GLOBAL_TYPES: &[&str] = &["Product", "SoftwareSystem", "Container", "Domain", "Role"];

fn is_global_type(artifact_type: &str) -> bool {
    GLOBAL_TYPES.contains(&artifact_type)
}

impl AliasTable {
    pub fn load(s5d_dir: &Path) -> anyhow::Result<Self> {
        let path = s5d_dir.join("aliases.yaml");
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        Ok(serde_yaml::from_str(&content)?)
    }

    pub fn save(&self, s5d_dir: &Path) -> anyhow::Result<()> {
        let path = s5d_dir.join("aliases.yaml");
        std::fs::write(&path, serde_yaml::to_string(self)?)?;
        Ok(())
    }

    pub fn resolve(
        &mut self,
        package_id: &str,
        artifact_id: &str,
        artifact_type: &str,
    ) -> (String, bool) {
        if is_global_type(artifact_type) {
            if let Some(entry) = self.global.iter().find(|e| {
                e.artifact_id == artifact_id && e.artifact_type == artifact_type && !e.deprecated
            }) {
                return (entry.uuid.clone(), false);
            }
            let uuid = Uuid::new_v4().to_string();
            self.global.push(AliasEntry {
                uuid: uuid.clone(),
                artifact_id: artifact_id.into(),
                artifact_type: artifact_type.into(),
                package_id: None,
                owning_package: Some(package_id.into()),
                deprecated: false,
            });
            (uuid, true)
        } else {
            if let Some(entry) = self.packages.iter().find(|e| {
                e.package_id.as_deref() == Some(package_id)
                    && e.artifact_id == artifact_id
                    && e.artifact_type == artifact_type
                    && !e.deprecated
            }) {
                return (entry.uuid.clone(), false);
            }
            let uuid = Uuid::new_v4().to_string();
            self.packages.push(AliasEntry {
                uuid: uuid.clone(),
                artifact_id: artifact_id.into(),
                artifact_type: artifact_type.into(),
                package_id: Some(package_id.into()),
                owning_package: None,
                deprecated: false,
            });
            (uuid, true)
        }
    }

    pub fn apply_renames(&mut self, package_id: &str, renames: &[crate::models::Rename]) {
        for rename in renames {
            let artifact_type = &rename.type_;
            if is_global_type(artifact_type) {
                if let Some(pos) = self.global.iter().position(|e| {
                    e.artifact_id == rename.old_id
                        && e.artifact_type == *artifact_type
                        && !e.deprecated
                }) {
                    let uuid = self.global[pos].uuid.clone();
                    let owning = self.global[pos].owning_package.clone();
                    self.global[pos].deprecated = true;
                    self.global.push(AliasEntry {
                        uuid,
                        artifact_id: rename.new_id.clone(),
                        artifact_type: artifact_type.clone(),
                        package_id: None,
                        owning_package: owning,
                        deprecated: false,
                    });
                }
            } else {
                let pkg = rename.old_package.as_deref().unwrap_or(package_id);
                if let Some(pos) = self.packages.iter().position(|e| {
                    e.package_id.as_deref() == Some(pkg)
                        && e.artifact_id == rename.old_id
                        && e.artifact_type == *artifact_type
                        && !e.deprecated
                }) {
                    let uuid = self.packages[pos].uuid.clone();
                    self.packages[pos].deprecated = true;
                    self.packages.push(AliasEntry {
                        uuid,
                        artifact_id: rename.new_id.clone(),
                        artifact_type: artifact_type.clone(),
                        package_id: Some(package_id.into()),
                        owning_package: None,
                        deprecated: false,
                    });
                }
            }
        }
    }
}
