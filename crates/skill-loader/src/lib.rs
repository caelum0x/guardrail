//! Runtime skill registry / loader for Guardrail Alpha.
//!
//! This crate reads the on-disk strategy "Skills" shipped under `skills/` and
//! exposes them to the rest of the system as a typed [`SkillCatalog`]:
//!
//! * [`SkillCatalog::load`] parses `skills/INDEX.json` into typed
//!   [`SkillEntry`] records.
//! * [`SkillCatalog::get`] / [`SkillCatalog::require`] look up a skill by id.
//! * [`SkillCatalog::list`] returns lightweight [`SkillSummary`] views.
//! * [`SkillCatalog::load_spec`] parses a skill's `strategy_spec.yaml` into a
//!   [`SkillSpec`] (typed identity header + dynamic body).
//! * [`SkillCatalog::validate_paths`] / [`SkillCatalog::missing_paths`] verify
//!   that every declared directory and spec file exists on disk.
//!
//! The loader is deliberately I/O-tolerant: missing or malformed files yield a
//! typed [`SkillLoaderError`] rather than a panic.
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//! use skill_loader::SkillCatalog;
//!
//! let catalog = SkillCatalog::load(Path::new("/path/to/repo"))?;
//! catalog.validate_paths()?;
//! for summary in catalog.list() {
//!     println!("{}: {}", summary.id, summary.name);
//! }
//! # Ok::<(), skill_loader::SkillLoaderError>(())
//! ```

mod catalog;
mod error;
mod spec;

pub use catalog::{SkillCatalog, SkillEntry, SkillSummary, INDEX_RELATIVE_PATH};
pub use error::{Result, SkillLoaderError};
pub use spec::{SkillSpec, SpecHeader};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    /// Resolve the repository root from this crate's manifest dir
    /// (`crates/skill-loader`) so tests are independent of the working
    /// directory.
    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .expect("repo root should resolve")
    }

    #[test]
    fn loads_real_index_with_at_least_five_skills() {
        let catalog = SkillCatalog::load(&repo_root()).expect("INDEX.json should load");
        assert!(
            catalog.len() >= 5,
            "expected >= 5 skills, got {}",
            catalog.len()
        );
        assert!(!catalog.is_empty());
    }

    #[test]
    fn every_catalog_path_exists_on_disk() {
        let catalog = SkillCatalog::load(&repo_root()).expect("INDEX.json should load");
        let missing = catalog.missing_paths();
        assert!(
            missing.is_empty(),
            "catalog references missing paths: {missing:?}"
        );
        catalog
            .validate_paths()
            .expect("all declared paths should exist");
    }

    #[test]
    fn get_and_require_resolve_known_skill() {
        let catalog = SkillCatalog::load(&repo_root()).expect("INDEX.json should load");
        let first_id = catalog.entries()[0].id.clone();

        assert!(catalog.get(&first_id).is_some());
        assert!(catalog.require(&first_id).is_ok());
        assert!(catalog.get("definitely-not-a-skill").is_none());

        match catalog.require("definitely-not-a-skill") {
            Err(SkillLoaderError::UnknownSkill(id)) => assert_eq!(id, "definitely-not-a-skill"),
            other => panic!("expected UnknownSkill, got {other:?}"),
        }
    }

    #[test]
    fn list_matches_entry_count() {
        let catalog = SkillCatalog::load(&repo_root()).expect("INDEX.json should load");
        assert_eq!(catalog.list().len(), catalog.len());
    }

    #[test]
    fn loads_strategy_spec_for_first_skill() {
        let catalog = SkillCatalog::load(&repo_root()).expect("INDEX.json should load");
        let id = catalog.entries()[0].id.clone();
        let spec = catalog.load_spec(&id).expect("spec should parse");
        assert!(spec.header.name.is_some(), "spec should declare a name");
        assert!(
            spec.section("inputs").is_some(),
            "spec body should expose an inputs section"
        );
    }

    #[test]
    fn counts_examples_on_disk() {
        let catalog = SkillCatalog::load(&repo_root()).expect("INDEX.json should load");
        let root = catalog.root().to_path_buf();
        // At least one skill should ship worked examples on disk.
        let total: u32 = catalog
            .entries()
            .iter()
            .map(|e| e.count_examples_on_disk(&root))
            .sum();
        assert!(total > 0, "expected some example files across skills");
    }

    #[test]
    fn missing_index_is_typed_error_not_panic() {
        let tmp = std::env::temp_dir().join("skill-loader-no-index-xyz");
        let err = SkillCatalog::load(&tmp).expect_err("should fail without index");
        assert!(matches!(err, SkillLoaderError::IndexNotFound(_)));
    }
}
