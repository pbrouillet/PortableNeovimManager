use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Feature — an individual toggleable unit within a workload
// ---------------------------------------------------------------------------

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Feature {
    pub id: String,
    pub name: String,
    pub description: String,
    pub plugins: Vec<String>,
    #[serde(default)]
    pub config_lua: Option<String>,
    /// Whether this feature is on by default when its parent workload is enabled.
    #[serde(default = "default_true")]
    pub default_enabled: bool,
}

// ---------------------------------------------------------------------------
// Preset — a named set of workloads for quick bulk-enable
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub workloads: Vec<String>,
}

// ---------------------------------------------------------------------------
// Workload
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Workload {
    pub id: String,
    pub name: String,
    pub description: String,
    pub base: bool,
    pub depends_on: Vec<String>,
    /// Features within this workload (new format).
    #[serde(default)]
    pub features: Vec<Feature>,
    /// DEPRECATED — kept for backward compat with old workloads.json.
    /// Use `features` instead. Auto-migrated on load via `normalize()`.
    #[serde(default, skip_serializing)]
    pub plugins: Vec<String>,
    /// DEPRECATED — kept for backward compat with old workloads.json.
    #[serde(default, skip_serializing)]
    pub config_lua: Option<String>,
    pub cli_aliases: Vec<String>,
    #[serde(default)]
    pub tutorial: Option<String>,
    /// Optional category for grouping in the UI (e.g. "Languages").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

impl Workload {
    /// Migrate old-format workloads (plugins/config_lua on Workload) to the
    /// new feature-based format.  No-op if features are already populated.
    pub fn normalize(&mut self) {
        if self.features.is_empty() && !self.plugins.is_empty() {
            self.features = vec![Feature {
                id: self.id.to_lowercase(),
                name: self.name.clone(),
                description: self.description.clone(),
                plugins: std::mem::take(&mut self.plugins),
                config_lua: self.config_lua.take(),
                default_enabled: true,
            }];
        }
    }

    /// Collect all plugin specs from all features.
    pub fn all_plugins(&self) -> Vec<String> {
        self.features
            .iter()
            .flat_map(|f| f.plugins.iter().cloned())
            .collect()
    }

    /// Collect all config_lua blocks from all features.
    pub fn all_config_lua(&self) -> Vec<String> {
        self.features
            .iter()
            .filter_map(|f| f.config_lua.clone())
            .collect()
    }

    /// Find a feature by its id within this workload.
    pub fn find_feature(&self, feature_id: &str) -> Option<&Feature> {
        self.features.iter().find(|f| f.id == feature_id)
    }
}

// ---------------------------------------------------------------------------
// Tutorial
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tutorial {
    pub id: String,
    pub title: String,
    pub content: String,
}

// ---------------------------------------------------------------------------
// WorkloadRegistry
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorkloadRegistry {
    pub workloads: Vec<Workload>,
    #[serde(default)]
    pub presets: Vec<Preset>,
    #[serde(default)]
    pub tutorials: Vec<Tutorial>,
}

impl WorkloadRegistry {
    /// Normalize all workloads (migrate old format if needed).
    pub fn normalize(&mut self) {
        for w in &mut self.workloads {
            w.normalize();
        }
    }

    pub fn all(&self) -> &[Workload] {
        &self.workloads
    }

    pub fn base(&self) -> Vec<&Workload> {
        self.workloads.iter().filter(|w| w.base).collect()
    }

    pub fn optional(&self) -> Vec<&Workload> {
        self.workloads.iter().filter(|w| !w.base).collect()
    }

    pub fn find_by_id(&self, id: &str) -> Option<&Workload> {
        self.workloads.iter().find(|w| w.id == id)
    }

    pub fn find_by_alias(&self, alias: &str) -> Option<&Workload> {
        let lower = alias.to_lowercase();
        self.workloads
            .iter()
            .find(|w| w.cli_aliases.iter().any(|a| a.to_lowercase() == lower))
    }

    /// Returns IDs of workloads that depend on the given workload ID.
    pub fn dependents_of(&self, id: &str) -> Vec<String> {
        self.workloads
            .iter()
            .filter(|w| w.depends_on.iter().any(|dep| dep == id))
            .map(|w| w.id.clone())
            .collect()
    }

    /// Transitively resolve all dependencies for the given workload IDs.
    /// Returns a deduplicated list that includes the original IDs plus all
    /// transitive dependencies, in dependency-first order.
    pub fn resolve_dependencies(&self, enabled: &[String]) -> Vec<String> {
        let mut resolved: Vec<String> = Vec::new();
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();

        for id in enabled {
            self.resolve_dep_recursive(id, &mut resolved, &mut visited);
        }

        resolved
    }

    fn resolve_dep_recursive(
        &self,
        id: &str,
        resolved: &mut Vec<String>,
        visited: &mut std::collections::HashSet<String>,
    ) {
        if visited.contains(id) {
            return;
        }
        visited.insert(id.to_string());

        if let Some(workload) = self.find_by_id(id) {
            // Resolve dependencies first (depth-first)
            for dep in &workload.depends_on {
                self.resolve_dep_recursive(dep, resolved, visited);
            }
        }

        resolved.push(id.to_string());
    }

    /// Validate that all dependencies for the given workloads are satisfiable.
    /// Returns Err with a list of missing dependency descriptions.
    pub fn validate_workloads(&self, enabled: &[String]) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        let enabled_set: std::collections::HashSet<&str> =
            enabled.iter().map(|s| s.as_str()).collect();

        for id in enabled {
            if let Some(workload) = self.find_by_id(id) {
                for dep in &workload.depends_on {
                    if self.find_by_id(dep).is_none() {
                        errors.push(format!(
                            "Workload '{id}' depends on '{dep}', which does not exist in the registry"
                        ));
                    }
                }
            } else if !enabled_set.contains(id.as_str()) {
                errors.push(format!("Unknown workload '{id}'"));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Find a general tutorial by its id.
    pub fn find_tutorial_by_id(&self, id: &str) -> Option<&Tutorial> {
        let lower = id.to_lowercase();
        self.tutorials.iter().find(|t| t.id.to_lowercase() == lower)
    }

    /// Returns a combined list of all tutorial topics: general tutorials first,
    /// then workloads that have a tutorial.  Each entry is (id, title).
    pub fn all_tutorial_topics(&self) -> Vec<(String, String)> {
        let mut topics: Vec<(String, String)> = self
            .tutorials
            .iter()
            .map(|t| (t.id.clone(), t.title.clone()))
            .collect();
        for w in &self.workloads {
            if w.tutorial.is_some() {
                topics.push((w.id.clone(), format!("{} — {}", w.name, w.description)));
            }
        }
        topics
    }

    /// Look up tutorial content by topic id.  Checks general tutorials first,
    /// then workload ids, then workload aliases.
    pub fn tutorial_content(&self, topic: &str) -> Option<(String, String)> {
        // General tutorials
        if let Some(t) = self.find_tutorial_by_id(topic) {
            return Some((t.title.clone(), t.content.clone()));
        }
        // Workload by id
        if let Some(w) = self.find_by_id(topic) {
            if let Some(ref content) = w.tutorial {
                return Some((w.name.clone(), content.clone()));
            }
        }
        // Workload by alias
        if let Some(w) = self.find_by_alias(topic) {
            if let Some(ref content) = w.tutorial {
                return Some((w.name.clone(), content.clone()));
            }
        }
        None
    }

    /// Find a preset by its id.
    pub fn find_preset(&self, id: &str) -> Option<&Preset> {
        let lower = id.to_lowercase();
        self.presets.iter().find(|p| p.id.to_lowercase() == lower)
    }

    /// Resolve a "WorkloadId/FeatureId" path to the workload and feature.
    pub fn find_feature_by_path(&self, path: &str) -> Option<(&Workload, &Feature)> {
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() != 2 {
            return None;
        }
        let workload = self.find_by_id(parts[0])?;
        let feature = workload.find_feature(parts[1])?;
        Some((workload, feature))
    }
}
