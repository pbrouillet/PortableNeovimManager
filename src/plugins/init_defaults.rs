use crate::config::{GlobalSettings, InstanceManifest};

/// Generate smart default `init_lua_post` content based on enabled features.
///
/// Currently generates:
/// - TreeView: auto-open neo-tree on VimEnter (when no files passed)
pub fn generate_default_post(features: &[String]) -> Option<String> {
    let mut blocks = Vec::new();

    if features.iter().any(|f| f.eq_ignore_ascii_case("treeview")) {
        blocks.push(
            r#"-- Auto-open explorer on startup (when no files passed)
vim.api.nvim_create_autocmd("VimEnter", {
  callback = function()
    if vim.fn.argc() == 0 then
      vim.cmd("Neotree show")
    end
  end,
})"#,
        );
    }

    if blocks.is_empty() {
        None
    } else {
        Some(blocks.join("\n\n"))
    }
}

/// Generate smart default `init_lua_pre` content based on enabled features.
///
/// Currently returns None — reserved for future feature-aware pre-plugin defaults.
pub fn generate_default_pre(_features: &[String]) -> Option<String> {
    None
}

/// Resolve the effective init_lua_pre: instance overrides global.
/// None means "inherit global". Empty string means "explicitly empty".
pub fn resolve_init_lua_pre(manifest: &InstanceManifest, settings: &GlobalSettings) -> Option<String> {
    manifest
        .init_lua_pre
        .clone()
        .or_else(|| settings.default_init_lua_pre.clone())
}

/// Resolve the effective init_lua_post: instance overrides global.
pub fn resolve_init_lua_post(manifest: &InstanceManifest, settings: &GlobalSettings) -> Option<String> {
    manifest
        .init_lua_post
        .clone()
        .or_else(|| settings.default_init_lua_post.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_post_with_treeview() {
        let features = vec!["TreeView".to_string()];
        let result = generate_default_post(&features);
        assert!(result.is_some());
        let lua = result.unwrap();
        assert!(lua.contains("Neotree show"));
        assert!(lua.contains("VimEnter"));
        assert!(lua.contains("vim.fn.argc() == 0"));
    }

    #[test]
    fn test_default_post_without_treeview() {
        let features = vec!["Lsp".to_string(), "Dap".to_string()];
        let result = generate_default_post(&features);
        assert!(result.is_none());
    }

    #[test]
    fn test_default_post_case_insensitive() {
        let features = vec!["treeview".to_string()];
        let result = generate_default_post(&features);
        assert!(result.is_some());
    }

    #[test]
    fn test_default_pre_returns_none() {
        let features = vec!["TreeView".to_string()];
        assert!(generate_default_pre(&features).is_none());
    }

    #[test]
    fn test_resolve_instance_overrides_global() {
        let mut manifest = InstanceManifest::new("test".into(), "v0.10".into(), vec![]);
        manifest.init_lua_post = Some("instance override".into());

        let mut settings = GlobalSettings::default();
        settings.default_init_lua_post = Some("global default".into());

        let result = resolve_init_lua_post(&manifest, &settings);
        assert_eq!(result.as_deref(), Some("instance override"));
    }

    #[test]
    fn test_resolve_falls_back_to_global() {
        let manifest = InstanceManifest::new("test".into(), "v0.10".into(), vec![]);
        let mut settings = GlobalSettings::default();
        settings.default_init_lua_post = Some("global default".into());

        let result = resolve_init_lua_post(&manifest, &settings);
        assert_eq!(result.as_deref(), Some("global default"));
    }

    #[test]
    fn test_resolve_none_when_both_unset() {
        let manifest = InstanceManifest::new("test".into(), "v0.10".into(), vec![]);
        let settings = GlobalSettings::default();

        assert!(resolve_init_lua_post(&manifest, &settings).is_none());
        assert!(resolve_init_lua_pre(&manifest, &settings).is_none());
    }
}
