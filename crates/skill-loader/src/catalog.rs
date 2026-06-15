//! The typed skill catalog loaded from `skills/INDEX.json`.

use crate::error::{Result, SkillLoaderError};
use crate::spec::SkillSpec;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// The default location of the index file relative to a repository root.
pub const INDEX_RELATIVE_PATH: &str = "skills/INDEX.json";

/// A single entry in `skills/INDEX.json`.
///
/// Field shape mirrors the on-disk JSON exactly so the file deserializes
/// without remapping.
#[derive(Debug, Clone, Deserialize)]
pub struct SkillEntry {
    /// Stable, directory-derived identifier (e.g. `cmc-regime-routed-alpha`).
    pub id: String,

    /// Human-facing strategy name (e.g. `regime-routed-bsc-alpha`).
    pub name: String,

    /// Path to the skill directory, relative to the repository root.
    pub path: String,

    /// One-paragraph summary of what the skill does.
    pub summary: String,

    /// Market regimes the skill declares support for.
    #[serde(default)]
    pub regimes: Vec<String>,

    /// Named input data feeds the skill consumes.
    #[serde(default)]
    pub inputs: Vec<String>,

    /// Size of the eligible trading universe.
    #[serde(default)]
    pub eligible_universe_size: u32,

    /// Number of worked examples the index claims exist on disk.
    #[serde(default)]
    pub examples_count: u32,

    /// Path to the strategy spec YAML, relative to the repository root.
    pub spec_file: String,
}

impl SkillEntry {
    /// Absolute path to this skill's directory, anchored at `root`.
    pub fn dir(&self, root: &Path) -> PathBuf {
        root.join(&self.path)
    }

    /// Absolute path to this skill's `strategy_spec.yaml`, anchored at `root`.
    pub fn spec_path(&self, root: &Path) -> PathBuf {
        root.join(&self.spec_file)
    }

    /// Absolute path to this skill's `examples/` directory, anchored at `root`.
    pub fn examples_dir(&self, root: &Path) -> PathBuf {
        self.dir(root).join("examples")
    }

    /// Count the example files actually present on disk (`*.json` under
    /// `examples/`). Returns `0` when the directory is missing — never panics.
    pub fn count_examples_on_disk(&self, root: &Path) -> u32 {
        let dir = self.examples_dir(root);
        let Ok(entries) = fs::read_dir(&dir) else {
            return 0;
        };
        entries
            .filter_map(std::result::Result::ok)
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
            })
            .count() as u32
    }

    /// Load this skill's strategy spec from disk.
    pub fn load_spec(&self, root: &Path) -> Result<SkillSpec> {
        SkillSpec::load(&self.spec_path(root))
    }
}

/// A lightweight, owned summary of a [`SkillEntry`] for listing.
#[derive(Debug, Clone)]
pub struct SkillSummary {
    /// Stable identifier.
    pub id: String,
    /// Human-facing name.
    pub name: String,
    /// One-paragraph summary.
    pub summary: String,
    /// Declared supported regimes.
    pub regimes: Vec<String>,
}

impl From<&SkillEntry> for SkillSummary {
    fn from(e: &SkillEntry) -> Self {
        Self {
            id: e.id.clone(),
            name: e.name.clone(),
            summary: e.summary.clone(),
            regimes: e.regimes.clone(),
        }
    }
}

/// The runtime skill registry, loaded from `skills/INDEX.json`.
#[derive(Debug, Clone)]
pub struct SkillCatalog {
    root: PathBuf,
    entries: Vec<SkillEntry>,
}

impl SkillCatalog {
    /// Load the catalog from a repository `root` directory.
    ///
    /// `root` is the directory that contains `skills/INDEX.json`. Returns
    /// [`SkillLoaderError::IndexNotFound`] when the index is absent and never
    /// panics on missing files.
    pub fn load(root: &Path) -> Result<Self> {
        let index_path = root.join(INDEX_RELATIVE_PATH);
        if !index_path.exists() {
            return Err(SkillLoaderError::IndexNotFound(index_path));
        }

        let raw = fs::read_to_string(&index_path).map_err(|source| SkillLoaderError::Io {
            path: index_path.clone(),
            source,
        })?;

        let entries: Vec<SkillEntry> =
            serde_json::from_str(&raw).map_err(|source| SkillLoaderError::Json {
                path: index_path,
                source,
            })?;

        Ok(Self {
            root: root.to_path_buf(),
            entries,
        })
    }

    /// The repository root this catalog was loaded from.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Number of skills in the catalog.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// All catalog entries.
    pub fn entries(&self) -> &[SkillEntry] {
        &self.entries
    }

    /// Look up a skill by its `id`.
    pub fn get(&self, id: &str) -> Option<&SkillEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Look up a skill by id, returning a typed error when absent.
    pub fn require(&self, id: &str) -> Result<&SkillEntry> {
        self.get(id)
            .ok_or_else(|| SkillLoaderError::UnknownSkill(id.to_string()))
    }

    /// Lightweight summaries of every skill, in catalog order.
    pub fn list(&self) -> Vec<SkillSummary> {
        self.entries.iter().map(SkillSummary::from).collect()
    }

    /// Load the strategy spec for a skill by id.
    pub fn load_spec(&self, id: &str) -> Result<SkillSpec> {
        self.require(id)?.load_spec(&self.root)
    }

    /// Validate that every catalog entry's directory and spec file exist on
    /// disk. Returns the first [`SkillLoaderError::MissingPath`] encountered,
    /// or `Ok(())` when all paths resolve.
    pub fn validate_paths(&self) -> Result<()> {
        for entry in &self.entries {
            let dir = entry.dir(&self.root);
            if !dir.is_dir() {
                return Err(SkillLoaderError::MissingPath(dir));
            }
            let spec = entry.spec_path(&self.root);
            if !spec.is_file() {
                return Err(SkillLoaderError::MissingPath(spec));
            }
        }
        Ok(())
    }

    /// Collect every entry whose declared paths are missing on disk, paired
    /// with the offending path. Empty when the catalog is fully consistent.
    pub fn missing_paths(&self) -> Vec<(&SkillEntry, PathBuf)> {
        let mut missing = Vec::new();
        for entry in &self.entries {
            let dir = entry.dir(&self.root);
            if !dir.is_dir() {
                missing.push((entry, dir));
                continue;
            }
            let spec = entry.spec_path(&self.root);
            if !spec.is_file() {
                missing.push((entry, spec));
            }
        }
        missing
    }
}
