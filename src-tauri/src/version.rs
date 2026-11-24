use serde::{Deserialize, Serialize};
use std::fmt;

/// A loose, dotted version number. Leonida has no strict semver contract, so
/// comparison is component-wise with zero padding, exactly how a human reads
/// `1.0.2.1` vs `1.0.3`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub parts: Vec<u64>,
    pub raw: String,
}

impl Version {
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        let start = trimmed.find(|c: char| c.is_ascii_digit())?;
        let tail = &trimmed[start..];
        let numeric: String = tail
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let cleaned = numeric.trim_matches('.').to_string();
        if cleaned.is_empty() {
            return None;
        }
        let parts: Vec<u64> = cleaned
            .split('.')
            .filter(|p| !p.is_empty())
            .filter_map(|p| p.parse::<u64>().ok())
            .collect();
        if parts.is_empty() {
            return None;
        }
        Some(Self {
            parts,
            raw: cleaned,
        })
    }

    pub fn at_least(&self, major: u64, minor: u64) -> bool {
        let a = self.parts.first().copied().unwrap_or(0);
        let b = self.parts.get(1).copied().unwrap_or(0);
        (a, b) >= (major, minor)
    }

    pub fn major(&self) -> u64 {
        self.parts.first().copied().unwrap_or(0)
    }

    pub fn short(&self) -> String {
        self.parts
            .iter()
            .take(3)
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(".")
    }

    pub fn cmp_loose(&self, other: &Version) -> std::cmp::Ordering {
        let len = self.parts.len().max(other.parts.len());
        for i in 0..len {
            let a = self.parts.get(i).copied().unwrap_or(0);
            let b = other.parts.get(i).copied().unwrap_or(0);
            match a.cmp(&b) {
                std::cmp::Ordering::Equal => continue,
                other => return other,
            }
        }
        std::cmp::Ordering::Equal
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// Compare two version strings; unparseable input sorts lower.
pub fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    match (Version::parse(left), Version::parse(right)) {
        (Some(a), Some(b)) => a.cmp_loose(&b),
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) => left.cmp(right),
    }
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    compare_versions(candidate, current) == std::cmp::Ordering::Greater
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_prefixes_and_suffixes() {
        assert_eq!(Version::parse("Python 3.12.4").unwrap().raw, "3.12.4");
        assert_eq!(Version::parse("v1.0.0").unwrap().raw, "1.0.0");
        assert_eq!(Version::parse("1.0.2.1").unwrap().parts, vec![1, 0, 2, 1]);
        assert!(Version::parse("no digits here").is_none());
    }

    #[test]
    fn compares_with_padding() {
        assert!(is_newer("1.0.1", "1.0.0"));
        assert!(is_newer("1.10.0", "1.9.9"));
        assert!(!is_newer("1.0.0", "1.0.0.0"));
        assert_eq!(compare_versions("1.0", "1.0.0"), std::cmp::Ordering::Equal);
    }

    #[test]
    fn reads_major_minor() {
        let v = Version::parse("10.0.26100.0").unwrap();
        assert!(v.at_least(10, 0));
        assert_eq!(v.short(), "10.0.26100");
        assert!(!Version::parse("8.0").unwrap().at_least(9, 0));
    }
}
