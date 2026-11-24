use crate::error::{AppError, AppResult};
use crate::leonida::compatibility::spec;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildRequest {
    pub stages: Vec<String>,
    pub game_path: Option<String>,
    pub out_path: String,
    pub work_path: String,
    pub jobs: u32,
    /// auto | on | off
    pub wave64_emulate: String,
    pub wav: bool,
    pub full_check: bool,
    pub fallback_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StageInfo {
    pub id: String,
    pub label: String,
    pub description: String,
    pub requires_dependencies: Vec<String>,
    pub selected_by_default: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkState {
    pub work_path: String,
    pub out_path: String,
    pub shader_output_present: bool,
    pub done_markers: Vec<String>,
    pub runtime_built: bool,
    pub resume_available: bool,
}

pub fn stage_catalog(detected: &[String]) -> Vec<StageInfo> {
    let definitions: [(&str, &str, &str, &[&str], bool); 5] = [
        (
            "check",
            "Package check",
            "Validates your own game copy against the manifest. Runs first and is never skipped.",
            &["python"],
            true,
        ),
        (
            "shaders",
            "Shader translation",
            "Carves RDNA2 shaders, disassembles them with LLVM and translates to SPIR-V. The longest CPU step.",
            &["python", "llvm-mc", "vulkan-sdk"],
            true,
        ),
        (
            "assets",
            "Asset export",
            "Exports models, texture dictionaries and audio banks. Needs the most disk space.",
            &["python", "dotnet-sdk"],
            false,
        ),
        (
            "runtime",
            "Runtime build",
            "Builds the loader that maps your executable and resolves its imports.",
            &["python", "vs-build-tools", "windows-sdk"],
            true,
        ),
        (
            "pipelines",
            "Pipeline pre-check",
            "Creates every translated shader on your GPU driver and saves a pipeline cache.",
            &["python", "vulkan-sdk"],
            false,
        ),
    ];
    definitions
        .iter()
        .filter(|(id, ..)| detected.is_empty() || detected.iter().any(|d| d == id))
        .map(|(id, label, description, requires, default)| StageInfo {
            id: id.to_string(),
            label: label.to_string(),
            description: description.to_string(),
            requires_dependencies: requires.iter().map(|s| s.to_string()).collect(),
            selected_by_default: *default,
        })
        .collect()
}

pub fn validate_stages(stages: &[String], detected: &[String]) -> AppResult<Vec<String>> {
    if stages.is_empty() {
        return Err(AppError::new(
            "invalid_stages",
            "Select at least one build stage",
        ));
    }
    let mut clean: Vec<String> = Vec::new();
    for stage in stages {
        let normalized = stage.trim().to_ascii_lowercase();
        if !normalized
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(AppError::new(
                "invalid_stages",
                format!("'{stage}' is not a valid stage name"),
            ));
        }
        if !detected.is_empty() && !detected.iter().any(|d| d == &normalized) {
            return Err(AppError::new(
                "unsupported_stage",
                format!("The installed Leonida does not report a '{normalized}' stage"),
            ));
        }
        if !clean.contains(&normalized) {
            clean.push(normalized);
        }
    }
    Ok(clean)
}

/// Build argv for `make_game.py`, using only options the installed checkout
/// documents. This is the single place where launcher options become upstream
/// arguments.
pub fn build_arguments(
    request: &BuildRequest,
    detected_stages: &[String],
    help_flags: &[String],
) -> AppResult<Vec<String>> {
    let has_flag = |flag: &str| help_flags.iter().any(|f| f == flag);

    let out = crate::paths::expand_home(&request.out_path);
    let work = crate::paths::expand_home(&request.work_path);
    if request.out_path.trim().is_empty() || request.work_path.trim().is_empty() {
        return Err(AppError::new(
            "invalid_paths",
            "Build output and work folders are required",
        ));
    }
    if out == work {
        return Err(AppError::new(
            "invalid_paths",
            "The output folder and the work folder must be different",
        ));
    }

    let stages = validate_stages(&request.stages, detected_stages)?;

    let mut args: Vec<String> = vec!["-I".into(), "make_game.py".into()];
    if has_flag("--no-tui") {
        args.push("--no-tui".into());
    }
    args.push("--stages".into());
    args.push(stages.join(","));

    if let Some(game) = request
        .game_path
        .as_ref()
        .map(|g| g.trim())
        .filter(|g| !g.is_empty())
    {
        let path = crate::paths::expand_home(game);
        if !path.is_dir() {
            return Err(AppError::new(
                "invalid_game_path",
                "The game folder does not exist",
            ));
        }
        args.push("--game".into());
        args.push(path.to_string_lossy().to_string());
    }

    args.push("--out".into());
    args.push(out.to_string_lossy().to_string());
    args.push("--work".into());
    args.push(work.to_string_lossy().to_string());

    if request.jobs > 0 {
        args.push("--jobs".into());
        args.push(request.jobs.to_string());
    }

    let wave = request.wave64_emulate.trim().to_ascii_lowercase();
    if !matches!(wave.as_str(), "auto" | "on" | "off") {
        return Err(AppError::new(
            "invalid_option",
            "--wave64-emulate expects auto, on or off",
        ));
    }
    if wave != "auto" {
        if !has_flag("--wave64-emulate") {
            return Err(AppError::new(
                "unsupported_option",
                "This Leonida revision does not document --wave64-emulate",
            ));
        }
        args.push("--wave64-emulate".into());
        args.push(wave);
    }

    if request.full_check {
        if !has_flag("--full-check") {
            return Err(AppError::new(
                "unsupported_option",
                "This Leonida revision does not document --full-check",
            ));
        }
        args.push("--full-check".into());
    }
    if request.wav {
        if !has_flag("--wav") {
            return Err(AppError::new(
                "unsupported_option",
                "This Leonida revision does not document --wav",
            ));
        }
        args.push("--wav".into());
    }

    for flag in &request.fallback_flags {
        let allowed = spec().expected.optional_flags.iter().any(|f| f == flag);
        if !allowed {
            return Err(AppError::new(
                "invalid_option",
                format!("{flag} is not a recognised fallback"),
            ));
        }
        if !has_flag(flag) {
            return Err(AppError::new(
                "unsupported_option",
                format!("The installed Leonida does not document {flag}"),
            ));
        }
        args.push(flag.clone());
    }

    Ok(args)
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressSignal {
    pub percent: Option<f64>,
    pub eta: Option<String>,
    pub stage: Option<String>,
}

/// Only report a percentage when upstream printed one. No invented smoothing.
pub fn parse_line(line: &str, stages: &[String]) -> ProgressSignal {
    static PERCENT: OnceLock<Regex> = OnceLock::new();
    static ETA: OnceLock<Regex> = OnceLock::new();
    let percent_re = PERCENT.get_or_init(|| Regex::new(r"(\d{1,3}(?:\.\d+)?)\s*%").unwrap());
    let eta_re =
        ETA.get_or_init(|| Regex::new(r"(?i)\beta\s*[:~]?\s*((?:\d+\s*[hms]\s*)+)").unwrap());

    let percent = percent_re
        .captures(line)
        .and_then(|c| c[1].parse::<f64>().ok())
        .filter(|p| (0.0..=100.0).contains(p));
    let eta = eta_re
        .captures(line)
        .map(|c| c[1].trim().replace(' ', ""))
        .filter(|s| !s.is_empty());

    let mut stage = None;
    let trimmed = line.trim_start_matches(['[', ' ', '=']);
    for candidate in stages {
        let rest = trimmed.strip_prefix(candidate.as_str());
        if let Some(rest) = rest {
            let first = rest.chars().next();
            if matches!(first, None | Some(':') | Some(']') | Some(' ') | Some('>')) {
                stage = Some(candidate.clone());
                break;
            }
        }
    }

    ProgressSignal {
        percent,
        eta,
        stage,
    }
}

fn walk_markers(dir: &Path, depth: u32, budget: &mut usize, out: &mut Vec<String>) {
    if depth > 5 || *budget == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(crate::paths::fs_path(dir)) else {
        return;
    };
    for entry in entries.flatten() {
        *budget = budget.saturating_sub(1);
        if *budget == 0 {
            return;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            walk_markers(&path, depth + 1, budget, out);
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if matches!(
            name.as_str(),
            "disasm.done" | "ogg.done" | "xma.done" | "translated.done"
        ) {
            out.push(path.to_string_lossy().to_string());
        }
    }
}

pub fn inspect_work(work: &Path, out: &Path) -> WorkState {
    let mut budget = 6000usize;
    let mut markers = Vec::new();
    if work.is_dir() {
        walk_markers(work, 0, &mut budget, &mut markers);
    }
    let shader_dir = work.join("shaders").join("spirv");
    let shader_output_present = shader_dir
        .read_dir()
        .map(|mut d| d.next().is_some())
        .unwrap_or(false);
    let runtime_built = out.join("gta6rt.exe").is_file() || out.join("gta6rt.sh").is_file();
    markers.sort();
    markers.dedup();
    WorkState {
        work_path: work.to_string_lossy().to_string(),
        out_path: out.to_string_lossy().to_string(),
        shader_output_present,
        resume_available: shader_output_present || !markers.is_empty() || runtime_built,
        done_markers: markers,
        runtime_built,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags() -> Vec<String> {
        vec![
            "--game".into(),
            "--out".into(),
            "--work".into(),
            "--stages".into(),
            "--jobs".into(),
            "--no-tui".into(),
            "--wave64-emulate".into(),
            "--wav".into(),
            "--full-check".into(),
            "--bary-fallback".into(),
        ]
    }

    fn request() -> BuildRequest {
        BuildRequest {
            stages: vec!["check".into(), "shaders".into()],
            game_path: None,
            out_path: r"D:\Out".into(),
            work_path: r"E:\Work".into(),
            jobs: 8,
            wave64_emulate: "auto".into(),
            wav: false,
            full_check: false,
            fallback_flags: vec![],
        }
    }

    #[test]
    fn builds_expected_argv() {
        let args =
            build_arguments(&request(), &["check".into(), "shaders".into()], &flags()).unwrap();
        assert_eq!(args[0], "-I");
        assert_eq!(args[1], "make_game.py");
        assert!(args.contains(&"--no-tui".to_string()));
        assert!(args.contains(&"check,shaders".to_string()));
        assert!(args.contains(&r"D:\Out".to_string()));
        assert!(!args.iter().any(|a| a.contains("--wave64-emulate")));
    }

    #[test]
    fn rejects_unknown_stage_and_flag() {
        let mut req = request();
        req.stages = vec!["mine-solana".into()];
        assert_eq!(
            build_arguments(&req, &["check".into()], &flags())
                .unwrap_err()
                .code,
            "unsupported_stage"
        );

        let mut req = request();
        req.wave64_emulate = "on".into();
        let err = build_arguments(
            &req,
            &["check".into(), "shaders".into()],
            &[
                "--stages".into(),
                "--out".into(),
                "--work".into(),
                "--game".into(),
                "--jobs".into(),
            ],
        )
        .unwrap_err();
        assert_eq!(err.code, "unsupported_option");
    }

    #[test]
    fn rejects_same_output_and_work_folder() {
        let mut req = request();
        req.work_path = req.out_path.clone();
        let err = build_arguments(&req, &["check".into()], &flags()).unwrap_err();
        assert_eq!(err.code, "invalid_paths");
    }

    #[test]
    fn parses_only_real_percentages_and_eta() {
        let stages = vec!["check".into(), "shaders".into()];
        let signal = parse_line("shaders: 1234/26503 (4.7%) eta 1h 20m", &stages);
        assert_eq!(signal.percent, Some(4.7));
        assert_eq!(signal.eta.as_deref(), Some("1h20m"));
        assert_eq!(signal.stage.as_deref(), Some("shaders"));

        let signal = parse_line("translated 100%", &stages);
        assert_eq!(signal.percent, Some(100.0));
        assert!(signal.stage.is_none());

        let signal = parse_line("working...", &stages);
        assert!(signal.percent.is_none());
        assert!(signal.eta.is_none());
    }

    #[test]
    fn stage_detection_needs_a_boundary() {
        let stages = vec!["check".into()];
        assert!(parse_line("checkers are fun", &stages).stage.is_none());
        assert_eq!(
            parse_line("[check] verifying", &stages).stage.as_deref(),
            Some("check")
        );
        assert_eq!(
            parse_line("check: ok", &stages).stage.as_deref(),
            Some("check")
        );
    }

    #[test]
    fn work_inspection_finds_markers_and_runtime() {
        let tmp = tempfile::tempdir().unwrap();
        let work = tmp.path().join("work");
        let out = tmp.path().join("out");
        std::fs::create_dir_all(work.join("shaders").join("spirv")).unwrap();
        std::fs::write(work.join("shaders").join("spirv").join("a.spv"), "x").unwrap();
        std::fs::create_dir_all(work.join("assets").join("audio")).unwrap();
        std::fs::write(work.join("assets").join("audio").join("xma.done"), "ok").unwrap();
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("gta6rt.exe"), "MZ").unwrap();
        let state = inspect_work(&work, &out);
        assert!(state.shader_output_present);
        assert!(state.runtime_built);
        assert!(state.resume_available);
        assert!(state.done_markers.iter().any(|m| m.ends_with("xma.done")));
    }

    #[test]
    fn empty_work_is_not_resumable() {
        let tmp = tempfile::tempdir().unwrap();
        let state = inspect_work(&tmp.path().join("missing"), &tmp.path().join("missing-out"));
        assert!(!state.resume_available);
    }

    #[test]
    fn stage_catalog_filters_to_detected() {
        let catalog = stage_catalog(&["check".into(), "assets".into()]);
        let ids: Vec<&str> = catalog.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["check", "assets"]);
    }
}
