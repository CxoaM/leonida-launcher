use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameCandidate {
    pub path: String,
    pub archive_count: usize,
    pub config_present: bool,
    pub score: u32,
}

const SKIPPED_DIRS: [&str; 14] = [
    "windows",
    "program files",
    "program files (x86)",
    "programdata",
    "appdata",
    "$recycle.bin",
    "system volume information",
    "node_modules",
    ".git",
    "perflogs",
    "recovery",
    "intel",
    "amd",
    "nvidia",
];

/// Home folders where players usually keep a package, mirroring (but not copying
/// the code of) the folders upstream scans.
pub fn default_game_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    let home = std::env::var_os("USERPROFILE").map(PathBuf::from);
    if let Some(home) = &home {
        for name in [
            "Downloads",
            "Desktop",
            "Documents",
            "OneDrive\\Documents",
            "OneDrive\\Desktop",
            "Games",
            "Videos",
        ] {
            let candidate = home.join(name);
            if candidate.is_dir() {
                roots.push(candidate);
            }
        }
        roots.push(home.clone());
    }
    // Drive roots, shallow search only. Removable and network drives are skipped
    // when they are not ready, which keeps the scan fast.
    for letter in b'C'..=b'Z' {
        let drive = PathBuf::from(format!("{}:\\", letter as char));
        if drive.is_dir() {
            roots.push(drive);
        }
    }
    roots
}

fn scan_dir(
    dir: &Path,
    depth: u32,
    max_depth: u32,
    budget: &mut usize,
    out: &mut Vec<GameCandidate>,
) {
    if depth > max_depth || *budget == 0 {
        return;
    }
    let entries = match std::fs::read_dir(crate::paths::fs_path(dir)) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    let mut child_dirs: Vec<PathBuf> = Vec::new();
    let mut archive_count = 0usize;
    let mut config_present = false;

    for entry in entries.flatten() {
        *budget = budget.saturating_sub(1);
        if *budget == 0 {
            break;
        }
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if file_type.is_dir() {
            child_dirs.push(entry.path());
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name == "microsoftgame.config" {
            config_present = true;
        } else if name.ends_with(".rpf") {
            archive_count += 1;
        }
    }

    if config_present && archive_count > 0 {
        let score = 100 + (archive_count.min(50) as u32) * 2;
        out.push(GameCandidate {
            path: dir.to_string_lossy().to_string(),
            archive_count,
            config_present,
            score,
        });
        // A package folder is not descended into: archives are huge and there is
        // nothing else to find inside.
        return;
    }

    for child in child_dirs {
        let lower = child
            .file_name()
            .map(|n| n.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if SKIPPED_DIRS.contains(&lower.as_str()) || lower.starts_with('.') {
            continue;
        }
        scan_dir(&child, depth + 1, max_depth, budget, out);
    }
}

/// Find package folders two levels deep, with a hard budget so a machine with
/// many drives cannot stall the UI.
pub fn detect_game_folders(extra_roots: Vec<PathBuf>) -> Vec<GameCandidate> {
    let mut budget = 40_000usize;
    let mut found: Vec<GameCandidate> = Vec::new();
    for root in extra_roots {
        if !root.is_dir() {
            continue;
        }
        scan_dir(&root, 0, 2, &mut budget, &mut found);
        if budget == 0 {
            break;
        }
    }
    found.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
    found.dedup_by(|a, b| a.path.eq_ignore_ascii_case(&b.path));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_package_two_levels_down() {
        let tmp = tempfile::tempdir().unwrap();
        let package = tmp.path().join("Games").join("GTA VI");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(package.join("MicrosoftGame.Config"), "<xml/>").unwrap();
        std::fs::write(package.join("levels.rpf"), "x").unwrap();
        std::fs::write(package.join("appdata0.rpf"), "x").unwrap();
        let found = detect_game_folders(vec![tmp.path().to_path_buf()]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].archive_count, 2);
        assert!(found[0].path.ends_with("GTA VI"));
    }

    #[test]
    fn ignores_folders_without_config() {
        let tmp = tempfile::tempdir().unwrap();
        let junk = tmp.path().join("stuff");
        std::fs::create_dir_all(&junk).unwrap();
        std::fs::write(junk.join("levels.rpf"), "x").unwrap();
        assert!(detect_game_folders(vec![tmp.path().to_path_buf()]).is_empty());
    }
}
