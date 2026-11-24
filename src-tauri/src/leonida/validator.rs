use crate::leonida::compatibility::spec;
use crate::paths::{disk_space, DiskSpace};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Upstream looks for these three names to recognise a package folder
/// (`PROBES` in make_game.py). We only look at names, never at contents.
const UPSTREAM_PROBES: [&str; 3] = ["MicrosoftGame.Config", "Catalog000.bin", "appdata0.rpf"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameValidation {
    pub path: String,
    pub exists: bool,
    pub config_present: bool,
    pub archive_count: usize,
    pub archive_samples: Vec<String>,
    pub probes_found: Vec<String>,
    pub valid: bool,
    pub issues: Vec<String>,
    pub notes: Vec<String>,
    pub disk: Option<DiskSpace>,
    pub expected_version: String,
    pub expected_free_gb: u64,
}

/// Structural check only. The authoritative package check is `make_game.py --check`,
/// which the build runs as its first stage; this function exists so the wizard can
/// fail fast on an obviously wrong folder.
pub fn validate_game_dir(path: &Path) -> GameValidation {
    let spec = spec();
    let mut issues: Vec<String> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let display = path.to_string_lossy().to_string();

    if !path.is_dir() {
        return GameValidation {
            path: display,
            exists: false,
            config_present: false,
            archive_count: 0,
            archive_samples: Vec::new(),
            probes_found: Vec::new(),
            valid: false,
            issues: vec!["That folder does not exist.".into()],
            notes: vec!["Point the launcher at the folder that holds MicrosoftGame.Config and the .rpf archives.".into()],
            disk: None,
            expected_version: spec.upstream.game_version.clone(),
            expected_free_gb: spec.requirements.free_gb_full_build,
        };
    }

    let mut archive_count = 0usize;
    let mut archive_samples = Vec::new();
    let mut probes_found = Vec::new();
    let mut config_present = false;

    if let Ok(entries) = std::fs::read_dir(crate::paths::fs_path(path)) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let lower = file_name.to_ascii_lowercase();
            if lower == "microsoftgame.config" {
                config_present = true;
            }
            if UPSTREAM_PROBES
                .iter()
                .any(|p| p.to_ascii_lowercase() == lower)
            {
                probes_found.push(file_name.clone());
            }
            if lower.ends_with(".rpf") {
                archive_count += 1;
                if archive_samples.len() < 8 {
                    archive_samples.push(file_name);
                }
            }
        }
    }

    if !config_present {
        issues.push("MicrosoftGame.Config is not in this folder.".into());
    }
    if archive_count == 0 {
        issues.push("No .rpf archives were found in this folder.".into());
    }
    if config_present && archive_count > 0 && probes_found.len() < UPSTREAM_PROBES.len() {
        notes.push(format!(
            "Only {} of the {} marker files upstream looks for are present; the full check will confirm the package.",
            probes_found.len(),
            UPSTREAM_PROBES.len()
        ));
    }
    notes.push(format!(
        "Supported package version is {}. Upstream validates every file name and size in its check stage; the launcher does not read game content.",
        spec.upstream.game_version
    ));

    let disk = disk_space(path).ok();
    if let Some(space) = &disk {
        let free_gb = space.free_bytes / 1024 / 1024 / 1024;
        if free_gb < 5 {
            issues.push("Less than 5 GB free on the package drive; Windows needs headroom.".into());
        }
    }

    GameValidation {
        path: display,
        exists: true,
        config_present,
        archive_count,
        archive_samples,
        probes_found,
        valid: config_present && archive_count > 0,
        issues,
        notes,
        disk,
        expected_version: spec.upstream.game_version.clone(),
        expected_free_gb: spec.requirements.free_gb_full_build,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_folder_is_reported_not_panicked() {
        let result = validate_game_dir(Path::new(r"Q:\definitely\missing"));
        assert!(!result.valid);
        assert!(!result.exists);
        assert!(!result.issues.is_empty());
    }

    #[test]
    fn recognises_package_shape() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("MicrosoftGame.Config"), "<xml/>").unwrap();
        std::fs::write(tmp.path().join("appdata0.rpf"), "not a real archive").unwrap();
        std::fs::write(tmp.path().join("levels.RPF"), "caps extension").unwrap();
        let result = validate_game_dir(tmp.path());
        assert!(result.valid);
        assert!(result.config_present);
        assert_eq!(result.archive_count, 2);
        assert_eq!(result.probes_found.len(), 2);
    }

    #[test]
    fn empty_folder_lists_every_problem() {
        let tmp = tempfile::tempdir().unwrap();
        let result = validate_game_dir(tmp.path());
        assert!(!result.valid);
        assert!(result
            .issues
            .iter()
            .any(|i| i.contains("MicrosoftGame.Config")));
        assert!(result.issues.iter().any(|i| i.contains(".rpf")));
    }
}
