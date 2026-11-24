use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// All directories the launcher owns. Nothing is written inside the Leonida git
/// working copy: settings, logs, cache and build artefacts live elsewhere.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePaths {
    pub config_dir: String,
    pub data_dir: String,
    pub cache_dir: String,
    pub logs_dir: String,
    pub documents_dir: String,
    pub default_repository: String,
    pub default_out: String,
    pub default_work: String,
    pub default_cache: String,
}

impl WorkspacePaths {
    pub fn resolve(app: &AppHandle) -> AppResult<Self> {
        let config_dir = app.path().app_config_dir().map_err(|e| {
            AppError::new("paths", "Cannot resolve the application config folder")
                .with_detail(e.to_string())
        })?;
        let data_dir = app.path().app_local_data_dir().map_err(|e| {
            AppError::new("paths", "Cannot resolve the application data folder")
                .with_detail(e.to_string())
        })?;
        let documents_dir = app
            .path()
            .document_dir()
            .unwrap_or_else(|_| data_dir.clone());

        let workspace = data_dir.join("workspace");
        Ok(Self {
            cache_dir: data_dir.join("cache").to_string_lossy().to_string(),
            logs_dir: config_dir.join("logs").to_string_lossy().to_string(),
            config_dir: config_dir.to_string_lossy().to_string(),
            data_dir: data_dir.to_string_lossy().to_string(),
            default_repository: workspace.join("repository").to_string_lossy().to_string(),
            default_work: workspace.join("work").to_string_lossy().to_string(),
            default_out: documents_dir.join("Leonida").to_string_lossy().to_string(),
            default_cache: data_dir.join("cache").to_string_lossy().to_string(),
            documents_dir: documents_dir.to_string_lossy().to_string(),
        })
    }

    pub fn ensure(&self) -> AppResult<()> {
        for dir in [
            &self.config_dir,
            &self.data_dir,
            &self.logs_dir,
            &self.cache_dir,
        ] {
            std::fs::create_dir_all(dir)
                .map_err(|e| AppError::io(format!("Creating {}", dir), e))?;
        }
        Ok(())
    }
}

/// A path that exists for disk-space queries even when the target folder has not
/// been created yet: free space is a property of the volume, so the closest
/// existing ancestor is authoritative.
pub fn existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut current = path.to_path_buf();
    loop {
        if current.exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskSpace {
    pub path: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub volume: String,
}

#[cfg(windows)]
pub fn disk_space(path: &Path) -> AppResult<DiskSpace> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let probe = existing_ancestor(path).ok_or_else(|| {
        AppError::new(
            "disk_space",
            "Path does not exist and has no existing parent",
        )
    })?;
    let mut wide: Vec<u16> = probe.as_os_str().encode_wide().collect();
    wide.push(0);

    let mut free_available: u64 = 0;
    let mut total: u64 = 0;
    let mut total_free: u64 = 0;
    // SAFETY: `wide` is a NUL terminated UTF-16 buffer and the out pointers are valid.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free_available,
            &mut total,
            &mut total_free,
        )
    };
    if ok == 0 {
        return Err(AppError::io(
            format!("Reading free space of {}", probe.display()),
            std::io::Error::last_os_error(),
        ));
    }
    Ok(DiskSpace {
        path: probe.to_string_lossy().to_string(),
        total_bytes: total,
        free_bytes: free_available,
        volume: volume_label(&probe),
    })
}

#[cfg(windows)]
fn volume_label(path: &Path) -> String {
    path.components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .unwrap_or_default()
}

#[cfg(not(windows))]
pub fn disk_space(_path: &Path) -> AppResult<DiskSpace> {
    Err(AppError::new(
        "unsupported_platform",
        "Disk space inspection is implemented for Windows",
    ))
}

/// Expand a leading `~` so users can type shell-ish paths into the settings form.
pub fn expand_home(input: &str) -> PathBuf {
    let trimmed = input.trim().trim_matches('"');
    if let Some(rest) = trimmed
        .strip_prefix("~/")
        .or_else(|| trimmed.strip_prefix("~\\"))
    {
        if let Some(home) = dirs_home() {
            return home.join(rest);
        }
    }
    PathBuf::from(trimmed)
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
}

/// Windows tools still struggle with paths beyond MAX_PATH. Prefixing with the
/// extended-length marker keeps long user folders (and Cyrillic names) working.
#[cfg(windows)]
pub fn fs_path(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if raw.starts_with(r"\\?\") || raw.len() < 240 {
        return path.to_path_buf();
    }
    if raw.starts_with(r"\\") {
        PathBuf::from(format!(r"\\?\UNC\{}", raw.trim_start_matches(r"\")))
    } else {
        PathBuf::from(format!(r"\\?\{raw}"))
    }
}

#[cfg(not(windows))]
pub fn fs_path(path: &Path) -> PathBuf {
    path.to_path_buf()
}

/// True when both paths live on the same Windows volume (drive letter or UNC share).
pub fn same_volume(a: &Path, b: &Path) -> bool {
    let key = |p: &Path| -> String {
        let text = p.to_string_lossy().to_string();
        if text.len() >= 2 && text.as_bytes()[1] == b':' {
            text[..2].to_ascii_uppercase()
        } else if let Some(rest) = text.strip_prefix(r"\\") {
            let mut parts = rest.splitn(3, '\\');
            let server = parts.next().unwrap_or("");
            let share = parts.next().unwrap_or("");
            format!(r"\\{server}\{share}").to_ascii_uppercase()
        } else {
            String::new()
        }
    };
    key(a) == key(b) && !key(a).is_empty()
}

pub fn path_is_within(child: &Path, parent: &Path) -> bool {
    let Ok(child) = child.canonicalize() else {
        return false;
    };
    let Ok(parent) = parent.canonicalize() else {
        return false;
    };
    child.starts_with(&parent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_tilde() {
        let expanded = expand_home("~/Leonida");
        assert!(expanded.is_absolute() || expanded.to_string_lossy().contains("Leonida"));
        assert!(!expanded.to_string_lossy().starts_with('~'));
    }

    #[test]
    fn strips_quotes_users_paste() {
        assert_eq!(
            expand_home("  \"D:\\Games\\Leonida\"  "),
            PathBuf::from("D:\\Games\\Leonida")
        );
    }

    #[test]
    fn same_volume_compares_drive_letters() {
        assert!(same_volume(Path::new(r"D:\a\b"), Path::new(r"d:\c")));
        assert!(!same_volume(Path::new(r"D:\a"), Path::new(r"E:\a")));
        assert!(!same_volume(Path::new(r"relative"), Path::new(r"other")));
    }

    #[test]
    fn existing_ancestor_walks_up() {
        let tmp = tempfile::tempdir().unwrap();
        let deep = tmp.path().join("not").join("created");
        assert_eq!(existing_ancestor(&deep), Some(tmp.path().to_path_buf()));
    }
}
