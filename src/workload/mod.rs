mod defaults;
mod loader;
mod model;

pub use defaults::*;
pub use loader::*;
pub use model::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry_has_all_workloads() {
        let reg = default_registry();
        assert_eq!(reg.all().len(), 16);
    }

    #[test]
    fn test_base_workloads() {
        let reg = default_registry();
        let base: Vec<&str> = reg.base().iter().map(|w| w.id.as_str()).collect();
        assert!(base.contains(&"Telescope"));
        assert!(base.contains(&"Treesitter"));
    }

    #[test]
    fn test_find_by_alias() {
        let reg = default_registry();
        let w = reg.find_by_alias("lsp").unwrap();
        assert_eq!(w.id, "Lsp");
    }

    #[test]
    fn test_find_by_alias_case_insensitive() {
        let reg = default_registry();
        assert!(reg.find_by_alias("LSP").is_some());
        assert!(reg.find_by_alias("Lsp").is_some());
    }

    #[test]
    fn test_dependents_of() {
        let reg = default_registry();
        let deps = reg.dependents_of("Lsp");
        assert!(deps.contains(&"OmniSharp".to_string()));
        assert!(deps.contains(&"Node".to_string()));
        assert!(deps.contains(&"Python".to_string()));
    }

    #[test]
    fn test_json_roundtrip() {
        let reg = default_registry();
        let json = serde_json::to_string(&reg).unwrap();
        let loaded: WorkloadRegistry = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.workloads.len(), reg.workloads.len());
    }

    #[test]
    fn test_all_tutorial_topics() {
        let reg = default_registry();
        let topics = reg.all_tutorial_topics();
        assert!(!topics.is_empty());
        assert_eq!(topics.len(), 18);
    }

    #[test]
    fn test_tutorial_content_by_id() {
        let reg = default_registry();
        let (title, content) = reg.tutorial_content("leader-key").unwrap();
        assert_eq!(title, "Understanding the Leader Key");
        assert!(!content.is_empty());
    }

    #[test]
    fn test_tutorial_content_by_workload_id() {
        let reg = default_registry();
        let (title, content) = reg.tutorial_content("Python").unwrap();
        assert_eq!(title, "Python");
        assert!(content.contains("pyright"));
    }

    #[test]
    fn test_tutorial_content_by_alias() {
        let reg = default_registry();
        let (title, _) = reg.tutorial_content("py").unwrap();
        assert_eq!(title, "Python");
        let (title, _) = reg.tutorial_content("cs").unwrap();
        assert_eq!(title, "OmniSharp");
        let (title, _) = reg.tutorial_content("ts").unwrap();
        assert_eq!(title, "Node");
    }

    #[test]
    fn test_tutorial_content_unknown_returns_none() {
        let reg = default_registry();
        assert!(reg.tutorial_content("nonexistent").is_none());
    }

    #[test]
    fn test_all_workloads_have_features() {
        let reg = default_registry();
        for w in reg.all() {
            assert!(
                !w.features.is_empty(),
                "Workload '{}' has no features",
                w.id
            );
        }
    }

    #[test]
    fn test_workload_all_plugins_via_features() {
        let reg = default_registry();
        let lsp = reg.find_by_id("Lsp").unwrap();
        let plugins = lsp.all_plugins();
        assert!(!plugins.is_empty());
        assert!(plugins.iter().any(|p| p.contains("nvim-lspconfig")));
    }

    #[test]
    fn test_find_feature_by_path() {
        let reg = default_registry();
        let (w, f) = reg.find_feature_by_path("Lsp/lspconfig").unwrap();
        assert_eq!(w.id, "Lsp");
        assert_eq!(f.id, "lspconfig");
        assert!(reg.find_feature_by_path("Nonexistent/foo").is_none());
    }

    #[test]
    fn test_normalize_migrates_old_format() {
        let json = r#"{
            "workloads": [{
                "id": "Old",
                "name": "Old",
                "description": "test",
                "base": false,
                "depends_on": [],
                "plugins": ["{ \"some/plugin\" }"],
                "config_lua": "vim.g.test = true",
                "cli_aliases": ["old"]
            }]
        }"#;
        let mut reg: WorkloadRegistry = serde_json::from_str(json).unwrap();
        reg.normalize();
        let w = &reg.workloads[0];
        assert_eq!(w.features.len(), 1);
        assert_eq!(w.features[0].id, "old");
        assert_eq!(w.features[0].plugins.len(), 1);
        assert!(w.features[0].config_lua.is_some());
        // Old fields should be emptied after migration
        assert!(w.plugins.is_empty());
        assert!(w.config_lua.is_none());
    }

    #[test]
    fn test_presets_exist() {
        let reg = default_registry();
        assert!(reg.find_preset("minimal").is_some());
        assert!(reg.find_preset("ide-core").is_some());
        assert!(reg.find_preset("ide-full").is_some());
    }

    // --- New tests for dependency resolution ---

    #[test]
    fn test_resolve_dependencies_simple() {
        let reg = default_registry();
        let enabled = vec!["Python".to_string()];
        let resolved = reg.resolve_dependencies(&enabled);
        // Python depends on Lsp, so Lsp should come before Python
        assert!(resolved.contains(&"Lsp".to_string()));
        assert!(resolved.contains(&"Python".to_string()));
        let lsp_pos = resolved.iter().position(|x| x == "Lsp").unwrap();
        let py_pos = resolved.iter().position(|x| x == "Python").unwrap();
        assert!(lsp_pos < py_pos);
    }

    #[test]
    fn test_resolve_dependencies_already_included() {
        let reg = default_registry();
        let enabled = vec!["Lsp".to_string(), "Python".to_string()];
        let resolved = reg.resolve_dependencies(&enabled);
        // No duplicates
        let lsp_count = resolved.iter().filter(|x| *x == "Lsp").count();
        assert_eq!(lsp_count, 1);
    }

    #[test]
    fn test_resolve_dependencies_no_deps() {
        let reg = default_registry();
        let enabled = vec!["TreeView".to_string(), "Tabs".to_string()];
        let resolved = reg.resolve_dependencies(&enabled);
        assert_eq!(resolved, vec!["TreeView", "Tabs"]);
    }

    #[test]
    fn test_resolve_dependencies_chain() {
        let reg = default_registry();
        // Completion depends on Lsp
        let enabled = vec!["Completion".to_string()];
        let resolved = reg.resolve_dependencies(&enabled);
        assert!(resolved.contains(&"Lsp".to_string()));
        assert!(resolved.contains(&"Completion".to_string()));
    }

    #[test]
    fn test_validate_workloads_valid() {
        let reg = default_registry();
        let enabled = vec!["Lsp".to_string(), "Python".to_string()];
        assert!(reg.validate_workloads(&enabled).is_ok());
    }
}
