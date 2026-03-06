//! Path discovery for the MCP server.
//!
//! Locates the project root, the `fyaml-tokenize` binary, and the YAML Test Suite
//! directory by walking up from the executable location or current directory.

use std::path::{Path, PathBuf};

/// Discovered project paths for the MCP server.
pub struct YamalgamPaths {
    /// Path to the `fyaml-tokenize` binary.
    pub fyaml_tokenize: PathBuf,
    /// Path to the `vendor/yaml-test-suite/` directory.
    pub test_suite_dir: PathBuf,
    /// Project root directory (workspace root).
    pub project_root: PathBuf,
}

impl YamalgamPaths {
    /// Discover paths by walking up from the executable location.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The workspace root cannot be found (no `Cargo.toml` containing `[workspace]`)
    /// - `tools/fyaml-tokenize/fyaml-tokenize` does not exist
    /// - `vendor/yaml-test-suite/` does not exist
    pub fn discover() -> anyhow::Result<Self> {
        let project_root = find_workspace_root()?;

        let fyaml_tokenize = project_root.join("tools/fyaml-tokenize/fyaml-tokenize");
        anyhow::ensure!(
            fyaml_tokenize.exists(),
            "fyaml-tokenize binary not found: {}",
            fyaml_tokenize.display()
        );

        let test_suite_dir = project_root.join("vendor/yaml-test-suite");
        anyhow::ensure!(
            test_suite_dir.exists(),
            "YAML Test Suite not found: {}",
            test_suite_dir.display()
        );

        Ok(Self {
            fyaml_tokenize,
            test_suite_dir,
            project_root,
        })
    }
}

/// Walk up from the executable location (or `YAMALGAM_ROOT` / cwd as fallbacks)
/// looking for a `Cargo.toml` that contains `[workspace]`.
fn find_workspace_root() -> anyhow::Result<PathBuf> {
    // 1. Try walking up from executable location.
    if let Ok(exe) = std::env::current_exe()
        && let Some(root) = walk_up_for_workspace(exe.as_path())
    {
        return Ok(root);
    }

    // 2. Try YAMALGAM_ROOT env var.
    if let Ok(root) = std::env::var("YAMALGAM_ROOT") {
        let p = PathBuf::from(root);
        if has_workspace_toml(&p) {
            return Ok(p);
        }
    }

    // 3. Try current directory.
    if let Ok(cwd) = std::env::current_dir()
        && let Some(root) = walk_up_for_workspace(cwd.as_path())
    {
        return Ok(root);
    }

    anyhow::bail!(
        "cannot find yamalgam workspace root — set YAMALGAM_ROOT or run from within the project"
    )
}

/// Walk up from `start` looking for a directory containing a workspace `Cargo.toml`.
fn walk_up_for_workspace(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        if has_workspace_toml(&dir) {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Check whether `dir/Cargo.toml` exists and contains `[workspace]`.
fn has_workspace_toml(dir: &Path) -> bool {
    let cargo_toml = dir.join("Cargo.toml");
    std::fs::read_to_string(cargo_toml).is_ok_and(|content| content.contains("[workspace]"))
}
