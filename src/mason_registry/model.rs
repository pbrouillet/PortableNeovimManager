use serde::{Deserialize, Serialize};

/// Category enum for mason packages.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MasonCategory {
    #[serde(rename = "LSP")]
    Lsp,
    #[serde(rename = "DAP")]
    Dap,
    #[serde(rename = "Formatter")]
    Formatter,
    #[serde(rename = "Linter")]
    Linter,
}

impl std::fmt::Display for MasonCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lsp => write!(f, "LSP"),
            Self::Dap => write!(f, "DAP"),
            Self::Formatter => write!(f, "Formatter"),
            Self::Linter => write!(f, "Linter"),
        }
    }
}

/// Optional Neovim-specific metadata from mason package definitions.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct MasonNeovimMeta {
    /// The lspconfig server name (e.g. "pyright", "lua_ls", "rust_analyzer").
    #[serde(default)]
    pub lspconfig: Option<String>,
}

/// A package from the mason-registry.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MasonPackage {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub categories: Vec<MasonCategory>,
    #[serde(default)]
    pub licenses: Vec<String>,
    #[serde(default)]
    pub neovim: Option<MasonNeovimMeta>,
}

impl MasonPackage {
    /// Check if this package belongs to a given category.
    pub fn is_category(&self, cat: &MasonCategory) -> bool {
        self.categories.contains(cat)
    }

    /// Get the lspconfig server name, if this is an LSP package with one.
    pub fn lspconfig_name(&self) -> Option<&str> {
        self.neovim.as_ref()?.lspconfig.as_deref()
    }

    /// Primary language (first in the list), if any.
    pub fn primary_language(&self) -> Option<&str> {
        self.languages.first().map(|s| s.as_str())
    }
}

/// The full mason registry: a list of packages.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MasonRegistry {
    pub packages: Vec<MasonPackage>,
}

impl MasonRegistry {
    pub fn new(packages: Vec<MasonPackage>) -> Self {
        Self { packages }
    }

    /// Filter packages by category.
    pub fn by_category(&self, cat: &MasonCategory) -> Vec<&MasonPackage> {
        self.packages.iter().filter(|p| p.is_category(cat)).collect()
    }

    /// Filter packages by language (case-insensitive).
    pub fn by_language(&self, language: &str) -> Vec<&MasonPackage> {
        let lower = language.to_lowercase();
        self.packages
            .iter()
            .filter(|p| p.languages.iter().any(|l| l.to_lowercase() == lower))
            .collect()
    }

    /// Search packages by query string (matches name or description, case-insensitive).
    pub fn search(&self, query: &str) -> Vec<&MasonPackage> {
        let lower = query.to_lowercase();
        self.packages
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&lower)
                    || p.description.to_lowercase().contains(&lower)
                    || p.languages.iter().any(|l| l.to_lowercase().contains(&lower))
            })
            .collect()
    }

    /// Find a package by exact name.
    pub fn find_by_name(&self, name: &str) -> Option<&MasonPackage> {
        self.packages.iter().find(|p| p.name == name)
    }

    /// Get all unique categories present in the registry.
    pub fn available_categories(&self) -> Vec<MasonCategory> {
        let mut cats: Vec<MasonCategory> = Vec::new();
        for p in &self.packages {
            for c in &p.categories {
                if !cats.contains(c) {
                    cats.push(c.clone());
                }
            }
        }
        cats
    }

    /// Get all unique languages present in the registry, sorted.
    pub fn available_languages(&self) -> Vec<String> {
        let mut langs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for p in &self.packages {
            for l in &p.languages {
                langs.insert(l.clone());
            }
        }
        langs.into_iter().collect()
    }

    /// Total package count.
    pub fn len(&self) -> usize {
        self.packages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}
