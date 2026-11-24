use crate::error::{AppError, AppResult};
use crate::paths::fs_path;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Read JSON, returning `None` for a missing file. A corrupt file is moved aside
/// instead of taking the app down, so a half-written settings file never bricks
/// the launcher.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> AppResult<Option<T>> {
    let real = fs_path(path);
    if !real.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&real)
        .map_err(|e| AppError::io(format!("Reading {}", path.display()), e))?;
    if raw.trim().is_empty() {
        return Ok(None);
    }
    match serde_json::from_str::<T>(&raw) {
        Ok(value) => Ok(Some(value)),
        Err(err) => {
            let backup = path.with_extension(format!(
                "corrupt-{}.json",
                chrono::Utc::now().format("%Y%m%d%H%M%S")
            ));
            let _ = std::fs::rename(&real, fs_path(&backup));
            Err(AppError::new(
                "corrupt_state",
                format!("{} was unreadable and has been set aside", path.display()),
            )
            .with_detail(err.to_string()))
        }
    }
}

/// Write JSON atomically: a temporary file in the same directory followed by a
/// rename, so a crash mid-write cannot truncate the real file.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(fs_path(parent))
            .map_err(|e| AppError::io(format!("Creating {}", parent.display()), e))?;
    }
    let pretty = serde_json::to_string_pretty(value)?;
    let tmp: PathBuf = path.with_extension("tmp");
    std::fs::write(fs_path(&tmp), pretty.as_bytes())
        .map_err(|e| AppError::io(format!("Writing {}", tmp.display()), e))?;
    std::fs::rename(fs_path(&tmp), fs_path(path))
        .map_err(|e| AppError::io(format!("Replacing {}", path.display()), e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Sample {
        name: String,
        count: u32,
    }

    #[test]
    fn round_trips_and_reports_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nested").join("state.json");
        assert!(read_json::<Sample>(&path).unwrap().is_none());
        let sample = Sample {
            name: "leonida".into(),
            count: 7,
        };
        write_json(&path, &sample).unwrap();
        assert_eq!(read_json::<Sample>(&path).unwrap(), Some(sample));
    }

    #[test]
    fn corrupt_file_is_quarantined() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("state.json");
        std::fs::write(&path, "{ not json").unwrap();
        let err = read_json::<Sample>(&path).unwrap_err();
        assert_eq!(err.code, "corrupt_state");
        assert!(!path.exists());
        let leftovers: Vec<_> = std::fs::read_dir(tmp.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(leftovers.len(), 1);
    }
}
