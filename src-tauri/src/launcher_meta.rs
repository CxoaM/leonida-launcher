use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const META_JSON: &str = include_str!("../resources/launcher.json");
pub const PLACEHOLDER_OWNER: &str = "YOUR_GITHUB_OWNER";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherMetaFile {
    pub owner: String,
    pub repository: String,
    pub upstream_repository: String,
    pub license: String,
    pub channel: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherMeta {
    pub product_name: String,
    pub version: String,
    pub owner: String,
    pub repository_url: String,
    pub upstream_url: String,
    pub license: String,
    pub channel: String,
    pub updater_configured: bool,
    pub updater_endpoint: String,
    pub identifier: String,
}

impl LauncherMeta {
    pub fn configured_owner(&self) -> bool {
        !self.owner.is_empty() && self.owner != PLACEHOLDER_OWNER
    }

    /// The updater manifest lives next to the published release assets, which is
    /// why the endpoint is derived from the repository the launcher ships with.
    pub fn release_endpoint(&self) -> String {
        if !self.configured_owner() {
            return String::new();
        }
        format!(
            "https://github.com/{}/{}/releases/latest/download/latest.json",
            self.owner,
            self.repo_slug()
        )
    }

    pub fn repo_slug(&self) -> &str {
        static DEFAULT: OnceLock<&'static str> = OnceLock::new();
        let file = file();
        if file.repository.trim().is_empty() {
            DEFAULT.get_or_init(|| "leonida-launcher")
        } else {
            file.repository.as_str()
        }
    }
}

pub fn file() -> &'static LauncherMetaFile {
    static FILE: OnceLock<LauncherMetaFile> = OnceLock::new();
    FILE.get_or_init(|| {
        serde_json::from_str::<LauncherMetaFile>(META_JSON).unwrap_or_else(|_| LauncherMetaFile {
            owner: PLACEHOLDER_OWNER.into(),
            repository: "leonida-launcher".into(),
            upstream_repository: "https://github.com/ubiquitarianism/leonida".into(),
            license: "MIT".into(),
            channel: "stable".into(),
            notes: String::new(),
        })
    })
}

pub fn meta() -> LauncherMeta {
    let file = file();
    let owner = file.owner.trim().to_string();
    let repository = if file.repository.trim().is_empty() {
        "leonida-launcher".to_string()
    } else {
        file.repository.trim().to_string()
    };
    let configured = !owner.is_empty() && owner != PLACEHOLDER_OWNER;
    let repository_url = if configured {
        format!("https://github.com/{owner}/{repository}")
    } else {
        String::new()
    };
    let updater_endpoint = if configured {
        format!("https://github.com/{owner}/{repository}/releases/latest/download/latest.json")
    } else {
        String::new()
    };
    LauncherMeta {
        product_name: "Leonida Launcher".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        owner,
        repository_url,
        upstream_url: file.upstream_repository.clone(),
        license: file.license.clone(),
        channel: file.channel.clone(),
        updater_configured: configured,
        updater_endpoint,
        identifier: "dev.leonida.launcher".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_is_derived_or_empty() {
        let meta = meta();
        if meta.updater_configured {
            assert!(meta
                .updater_endpoint
                .ends_with("/releases/latest/download/latest.json"));
            assert!(meta.updater_endpoint.contains(&meta.owner));
        } else {
            assert!(meta.updater_endpoint.is_empty());
        }
    }

    #[test]
    fn baked_metadata_is_parseable() {
        let file = file();
        assert!(!file.upstream_repository.is_empty());
        assert!(file.upstream_repository.starts_with("https://"));
    }
}
