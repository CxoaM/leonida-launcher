use crate::error::AppResult;
use crate::process::{resolve_executable, run_capture, CommandSpec};
use crate::version::Version;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

const PROBE_TIMEOUT: Duration = Duration::from_secs(25);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    Required,
    Recommended,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyStatus {
    Installed,
    Missing,
    Outdated,
    Invalid,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FixSuggestion {
    pub summary: String,
    pub url: Option<String>,
    pub winget_id: Option<String>,
    pub notes: Option<String>,
}

impl FixSuggestion {
    fn new(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            url: None,
            winget_id: None,
            notes: None,
        }
    }
    fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
    fn winget(mut self, id: impl Into<String>) -> Self {
        self.winget_id = Some(id.into());
        self
    }
    fn notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyReport {
    pub id: String,
    pub name: String,
    pub kind: DependencyKind,
    pub status: DependencyStatus,
    pub version: Option<String>,
    pub required_version: Option<String>,
    pub detail: String,
    pub path: Option<String>,
    /// Build stages that cannot run without this component.
    pub affects: Vec<String>,
    pub fix: Option<FixSuggestion>,
}

impl DependencyReport {
    fn new(
        id: &str,
        name: &str,
        kind: DependencyKind,
        status: DependencyStatus,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            status,
            version: None,
            required_version: None,
            detail: detail.into(),
            path: None,
            affects: Vec::new(),
            fix: None,
        }
    }
    fn affects(mut self, stages: &[&str]) -> Self {
        self.affects = stages.iter().map(|s| s.to_string()).collect();
        self
    }
    fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }
    fn required(mut self, version: impl Into<String>) -> Self {
        self.required_version = Some(version.into());
        self
    }
    fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }
    fn fix(mut self, fix: FixSuggestion) -> Self {
        self.fix = Some(fix);
        self
    }
    pub fn blocks(&self) -> bool {
        self.kind == DependencyKind::Required
            && matches!(
                self.status,
                DependencyStatus::Missing | DependencyStatus::Outdated | DependencyStatus::Invalid
            )
    }
}

#[derive(Debug, Clone)]
pub struct PythonTool {
    pub program: String,
    pub prefix: Vec<String>,
    pub version: Version,
    pub path: Option<String>,
}

impl PythonTool {
    pub fn spec(&self) -> CommandSpec {
        CommandSpec::new(&self.program).args(self.prefix.clone())
    }
}

fn discover_python_installs() -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        roots.push(PathBuf::from(local).join("Programs").join("Python"));
    }
    if let Some(pf) = std::env::var_os("ProgramFiles") {
        roots.push(PathBuf::from(pf));
    }
    for root in roots {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let candidate = entry.path().join("python.exe");
            if candidate.is_file() {
                found.push(candidate);
            }
        }
    }
    // Newest folder names first so the best interpreter wins when PATH is stale.
    found.sort();
    found.reverse();
    found
}

/// Find a usable Python. `py -3` is preferred because it resolves the newest
/// installed 3.x even when PATH still points at an older version.
pub async fn python_tool() -> Option<PythonTool> {
    let mut candidates: Vec<(String, Vec<String>)> = Vec::new();
    if let Ok(explicit) = std::env::var("LEONIDA_PYTHON") {
        if !explicit.trim().is_empty() {
            candidates.push((explicit.trim().to_string(), Vec::new()));
        }
    }
    candidates.push(("py".into(), vec!["-3".into()]));
    candidates.push(("python".into(), Vec::new()));
    candidates.push(("python3".into(), Vec::new()));
    for install in discover_python_installs() {
        candidates.push((install.to_string_lossy().to_string(), Vec::new()));
    }

    let mut best: Option<PythonTool> = None;
    for (program, prefix) in candidates {
        if program != "py"
            && resolve_executable(&program).is_none()
            && !Path::new(&program).is_file()
        {
            continue;
        }
        let spec = CommandSpec::new(&program)
            .args(prefix.clone())
            .arg("--version");
        let Ok(output) = run_capture(&spec, Duration::from_secs(15)).await else {
            continue;
        };
        let text = format!("{} {}", output.stdout, output.stderr);
        let Some(version) = Version::parse(text.split("Python").nth(1).unwrap_or(&text)) else {
            continue;
        };
        let path = resolve_executable(&program)
            .map(|p| p.to_string_lossy().to_string())
            .or_else(|| Path::new(&program).is_file().then(|| program.clone()));
        let candidate = PythonTool {
            program,
            prefix,
            version,
            path,
        };
        let better = match &best {
            None => true,
            Some(current) => {
                candidate.version.cmp_loose(&current.version) == std::cmp::Ordering::Greater
            }
        };
        if better {
            best = Some(candidate);
        }
    }
    best
}

/// Split a user provided command line (for example `LLVM_MC="wsl -d Ubuntu -e .../llvm-mc"`)
/// into program + arguments, honouring double quotes.
pub fn split_command_line(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = input.trim().chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => in_quotes = !in_quotes,
            '\\' if in_quotes && chars.peek() == Some(&'"') => {
                current.push('"');
                chars.next();
            }
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

struct LlvmProbe {
    version: Option<String>,
    path: Option<String>,
    has_amdgpu: bool,
    #[allow(dead_code)]
    source: &'static str,
    error: Option<String>,
}

async fn probe_llvm_mc() -> LlvmProbe {
    let (program, prefix, source) = match std::env::var("LLVM_MC") {
        Ok(value) if !value.trim().is_empty() => {
            let mut parts = split_command_line(&value);
            if parts.is_empty() {
                return LlvmProbe {
                    version: None,
                    path: None,
                    has_amdgpu: false,
                    source: "env",
                    error: Some("LLVM_MC is set but empty".into()),
                };
            }
            let program = parts.remove(0);
            (program, parts, "env")
        }
        _ => match resolve_executable("llvm-mc") {
            Some(path) => (path.to_string_lossy().to_string(), Vec::new(), "path"),
            None => {
                return LlvmProbe {
                    version: None,
                    path: None,
                    has_amdgpu: false,
                    source: "path",
                    error: None,
                }
            }
        },
    };

    let spec = CommandSpec::new(&program)
        .args(prefix.clone())
        .arg("--version");
    match run_capture(&spec, PROBE_TIMEOUT).await {
        Ok(out) if out.success() => {
            let text = format!("{}\n{}", out.stdout, out.stderr);
            let version = Version::parse(
                text.lines()
                    .find(|l| l.to_ascii_lowercase().contains("llvm version"))
                    .unwrap_or(""),
            )
            .map(|v| v.short());
            let has_amdgpu = text.to_ascii_lowercase().contains("amdgpu");
            LlvmProbe {
                version,
                path: Some(program),
                has_amdgpu,
                source,
                error: None,
            }
        }
        Ok(out) => LlvmProbe {
            version: None,
            path: Some(program),
            has_amdgpu: false,
            source,
            error: Some(format!(
                "`{} --version` exited with {:?}",
                spec.display(),
                out.code
            )),
        },
        Err(err) => LlvmProbe {
            version: None,
            path: Some(program),
            has_amdgpu: false,
            source,
            error: Some(err.message),
        },
    }
}

async fn probe_dotnet_sdks() -> (Option<String>, Vec<String>) {
    let spec = CommandSpec::new("dotnet").arg("--list-sdks");
    match run_capture(&spec, PROBE_TIMEOUT).await {
        Ok(out) if out.success() => {
            let versions: Vec<String> = out
                .stdout
                .lines()
                .filter_map(|line| line.split_whitespace().next().map(|s| s.to_string()))
                .filter(|v| Version::parse(v).is_some())
                .collect();
            let best = versions
                .iter()
                .max_by(|a, b| crate::version::compare_versions(a, b))
                .cloned();
            (best, versions)
        }
        _ => (None, Vec::new()),
    }
}

async fn probe_visual_studio() -> Option<(String, String)> {
    let vswhere = std::env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"))
        .join("Microsoft Visual Studio")
        .join("Installer")
        .join("vswhere.exe");
    if !vswhere.is_file() {
        return None;
    }
    let spec = CommandSpec::new(vswhere.to_string_lossy().to_string()).args([
        "-latest",
        "-products",
        "*",
        "-requires",
        "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
        "-property",
        "installationPath",
    ]);
    let output = run_capture(&spec, PROBE_TIMEOUT).await.ok()?;
    if !output.success() {
        return None;
    }
    let path = output.stdout.trim().to_string();
    if path.is_empty() {
        return None;
    }
    let version_spec = CommandSpec::new(vswhere.to_string_lossy().to_string()).args([
        "-latest",
        "-products",
        "*",
        "-requires",
        "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
        "-property",
        "catalog_productDisplayVersion",
    ]);
    let version = run_capture(&version_spec, PROBE_TIMEOUT)
        .await
        .ok()
        .map(|o| o.stdout.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "2022".into());
    Some((path, version))
}

fn windows_sdk_versions() -> Vec<String> {
    let root = std::env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"))
        .join("Windows Kits")
        .join("10")
        .join("Include");
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut versions: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            Version::parse(&name).map(|v| v.short())
        })
        .collect();
    versions.sort_by(|a, b| crate::version::compare_versions(a, b));
    versions.reverse();
    versions
}

pub fn long_paths_enabled() -> Option<bool> {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let key = hklm
            .open_subkey(r"SYSTEM\CurrentControlSet\Control\FileSystem")
            .ok()?;
        let value: u32 = key.get_value("LongPathsEnabled").ok()?;
        Some(value == 1)
    }
    #[cfg(not(windows))]
    {
        None
    }
}

pub async fn detect_all(windows_sdk_min: &str) -> Vec<DependencyReport> {
    let mut reports: Vec<DependencyReport> = Vec::new();

    // Python ------------------------------------------------------------------
    let python_required = "3.12";
    match python_tool().await {
        Some(tool) if tool.version.at_least(3, 12) => {
            reports.push(
                DependencyReport::new(
                    "python",
                    "Python",
                    DependencyKind::Required,
                    DependencyStatus::Installed,
                    format!(
                        "Interpreter {} with the standard library tools Leonida needs.",
                        tool.version.short()
                    ),
                )
                .version(tool.version.short())
                .required(python_required)
                .path(tool.path.clone().unwrap_or_else(|| tool.program.clone())),
            );
        }
        Some(tool) => {
            reports.push(
                DependencyReport::new(
                    "python",
                    "Python",
                    DependencyKind::Required,
                    DependencyStatus::Outdated,
                    format!(
                        "Python {} is installed, but Leonida requires 3.12 or newer.",
                        tool.version.short()
                    ),
                )
                .version(tool.version.short())
                .required(python_required)
                .path(tool.path.clone().unwrap_or_default())
                .affects(&["check", "shaders", "assets", "runtime", "pipelines"])
                .fix(
                    FixSuggestion::new(
                        "Install Python 3.12+ and leave 'Add python.exe to PATH' checked.",
                    )
                    .url("https://www.python.org/downloads/")
                    .winget("Python.Python.3.12")
                    .notes("py -3 is used when available, so multiple Python versions are fine."),
                ),
            );
        }
        None => {
            reports.push(
                DependencyReport::new(
                    "python",
                    "Python",
                    DependencyKind::Required,
                    DependencyStatus::Missing,
                    "No Python interpreter was found. Leonida's build entry point is make_game.py.",
                )
                .required(python_required)
                .affects(&["check", "shaders", "assets", "runtime", "pipelines"])
                .fix(
                    FixSuggestion::new("Install Python 3.12 or newer, then reopen the launcher.")
                        .url("https://www.python.org/downloads/")
                        .winget("Python.Python.3.12"),
                ),
            );
        }
    }

    // Git ---------------------------------------------------------------------
    match run_capture(&CommandSpec::new("git").arg("--version"), PROBE_TIMEOUT).await {
        Ok(out) if out.success() => {
            let version = Version::parse(&out.stdout)
                .map(|v| v.short())
                .unwrap_or_default();
            reports.push(
                DependencyReport::new(
                    "git",
                    "Git for Windows",
                    DependencyKind::Required,
                    DependencyStatus::Installed,
                    "Used to clone Leonida, follow upstream commits and update submodules.",
                )
                .version(version)
                .path(
                    resolve_executable("git")
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default(),
                ),
            );
        }
        _ => reports.push(
            DependencyReport::new(
                "git",
                "Git for Windows",
                DependencyKind::Required,
                DependencyStatus::Missing,
                "Git is required to fetch the Leonida sources and to track upstream commits.",
            )
            .fix(
                FixSuggestion::new("Install Git for Windows, then reopen the launcher.")
                    .url("https://git-scm.com/download/win")
                    .winget("Git.Git"),
            ),
        ),
    }

    // LLVM --------------------------------------------------------------------
    let llvm = probe_llvm_mc().await;
    let llvm_report = match (&llvm.version, llvm.has_amdgpu) {
        (Some(version), true) => DependencyReport::new(
            "llvm-mc",
            "LLVM llvm-mc with AMDGPU",
            DependencyKind::Required,
            DependencyStatus::Installed,
            "The AMDGPU target is present, so shader disassembly can run.",
        )
        .version(version.clone())
        .required("21+ with AMDGPU")
        .path(llvm.path.clone().unwrap_or_default()),
        (Some(version), false) => DependencyReport::new(
            "llvm-mc",
            "LLVM llvm-mc with AMDGPU",
            DependencyKind::Required,
            DependencyStatus::Invalid,
            "llvm-mc answers, but the AMDGPU target is missing. Official Windows LLVM builds omit it.",
        )
        .version(version.clone())
        .required("21+ with AMDGPU")
        .path(llvm.path.clone().unwrap_or_default())
        .affects(&["shaders", "pipelines"])
        .fix(
            FixSuggestion::new("Build a native llvm-mc with the AMDGPU target or use the conda/ WSL route.")
                .notes("Run scripts\\build_llvm_mc.ps1 inside the Leonida repository, or `conda install -c conda-forge llvm-tools`, then set LLVM_MC to the full path of llvm-mc.exe. Leonida's docs/design.md documents all three routes.")
                .url("https://github.com/ubiquitarianism/leonida/blob/main/docs/design.md"),
        ),
        (None, _) => {
            let mut report = DependencyReport::new(
                "llvm-mc",
                "LLVM llvm-mc with AMDGPU",
                DependencyKind::Required,
                DependencyStatus::Missing,
                match &llvm.error {
                    Some(err) => format!("llvm-mc could not be run: {err}"),
                    None => "llvm-mc was not found on PATH and LLVM_MC is not set.".to_string(),
                },
            )
            .required("21+ with AMDGPU")
            .affects(&["shaders", "pipelines"]);
            if let Some(path) = &llvm.path {
                report = report.path(path.clone());
            }
            report.fix(
                FixSuggestion::new("Install LLVM 21+ with the AMDGPU target, or point LLVM_MC at a compatible build.")
                    .notes("On Windows the official LLVM release has no AMDGPU target. Use scripts\\build_llvm_mc.ps1 (20-40 min), conda-forge llvm-tools, or a WSL llvm-mc via LLVM_MC.")
                    .winget("LLVM.LLVM")
                    .url("https://github.com/llvm/llvm-project/releases"),
            )
        }
    };
    reports.push(llvm_report);

    // SPIR-V tools ------------------------------------------------------------
    let spirv_as = resolve_executable("spirv-as");
    let spirv_val = resolve_executable("spirv-val");
    let vulkan_sdk = std::env::var("VULKAN_SDK")
        .ok()
        .filter(|v| !v.trim().is_empty());
    match (&spirv_as, &spirv_val, &vulkan_sdk) {
        (Some(_), Some(_), sdk) => {
            let version = run_capture(
                &CommandSpec::new("spirv-val").arg("--version"),
                PROBE_TIMEOUT,
            )
            .await
            .ok()
            .and_then(|o| {
                let text = format!("{} {}", o.stdout, o.stderr);
                text.split_whitespace()
                    .find(|token| {
                        token.to_ascii_lowercase().starts_with('v')
                            && token.chars().any(|c| c.is_ascii_digit())
                    })
                    .and_then(Version::parse)
                    .map(|v| v.short())
            });
            let mut report = DependencyReport::new(
                "vulkan-sdk",
                "Vulkan SDK (spirv-as / spirv-val)",
                DependencyKind::Required,
                DependencyStatus::Installed,
                "SPIR-V assembly and validation tools are on PATH.",
            )
            .required("1.3+");
            if let Some(v) = version {
                report = report.version(v);
            } else if let Some(sdk) = sdk {
                report = report.version(sdk.clone());
            }
            reports.push(report);
        }
        _ => {
            let sdk_tools = vulkan_sdk.as_ref().map(|sdk| {
                let bin = PathBuf::from(sdk).join("Bin");
                (
                    bin.join("spirv-as.exe").is_file(),
                    bin.join("spirv-val.exe").is_file(),
                )
            });
            let installed_but_hidden = matches!(sdk_tools, Some((true, true)));
            let (status, detail, notes) = if installed_but_hidden {
                (
                    DependencyStatus::Invalid,
                    "The Vulkan SDK is installed, but spirv-as / spirv-val are not on PATH.",
                    "Leonida calls spirv-as and spirv-val by name. Open a new terminal after installing the SDK, or add %VULKAN_SDK%\\Bin to PATH.",
                )
            } else {
                (
                    DependencyStatus::Missing,
                    "spirv-as and spirv-val were not found. They ship with the LunarG Vulkan SDK.",
                    "VULKAN_SDK must also be set when the runtime stage builds vk_pipeline_check.",
                )
            };
            reports.push(
                DependencyReport::new(
                    "vulkan-sdk",
                    "Vulkan SDK (spirv-as / spirv-val)",
                    DependencyKind::Required,
                    status,
                    detail,
                )
                .required("1.3+")
                .affects(&["shaders", "pipelines"])
                .fix(
                    FixSuggestion::new("Install the LunarG Vulkan SDK, then reopen the launcher.")
                        .url("https://vulkan.lunarg.com/")
                        .winget("KhronosGroup.VulkanSDK")
                        .notes(notes),
                ),
            );
        }
    }

    // .NET SDK ----------------------------------------------------------------
    let (dotnet_best, dotnet_all) = probe_dotnet_sdks().await;
    match dotnet_best {
        Some(version)
            if Version::parse(&version)
                .map(|v| v.major() >= 8)
                .unwrap_or(false) =>
        {
            reports.push(
                DependencyReport::new(
                    "dotnet-sdk",
                    ".NET SDK",
                    DependencyKind::Required,
                    DependencyStatus::Installed,
                    format!(
                        "{} SDK(s) detected; the assets stage builds rpf9dump with the SDK.",
                        dotnet_all.len()
                    ),
                )
                .version(version)
                .required("8+")
                .path(
                    resolve_executable("dotnet")
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default(),
                ),
            );
        }
        Some(version) => {
            reports.push(
                DependencyReport::new(
                    "dotnet-sdk",
                    ".NET SDK",
                    DependencyKind::Required,
                    DependencyStatus::Outdated,
                    format!("Only .NET SDK {version} was found. The .NET runtime alone cannot build rpf9dump."),
                )
                .version(version)
                .required("8+")
                .affects(&["assets"])
                .fix(
                    FixSuggestion::new("Install the .NET 8 SDK (not just the runtime), then reopen the launcher.")
                        .url("https://dotnet.microsoft.com/download/dotnet/8.0")
                        .winget("Microsoft.DotNet.SDK.8"),
                ),
            );
        }
        None => {
            reports.push(
                DependencyReport::new(
                    "dotnet-sdk",
                    ".NET SDK",
                    DependencyKind::Required,
                    DependencyStatus::Missing,
                    "No .NET SDK was found. `dotnet --list-sdks` returned nothing.",
                )
                .required("8+")
                .affects(&["assets"])
                .fix(
                    FixSuggestion::new(
                        "Install the .NET 8 SDK from Microsoft, then reopen the launcher.",
                    )
                    .url("https://dotnet.microsoft.com/download/dotnet/8.0")
                    .winget("Microsoft.DotNet.SDK.8"),
                ),
            );
        }
    }

    // Visual Studio C++ tools ------------------------------------------------
    match probe_visual_studio().await {
        Some((path, version)) => reports.push(
            DependencyReport::new(
                "vs-build-tools",
                "Visual Studio 2022 C++ tools",
                DependencyKind::Required,
                DependencyStatus::Installed,
                "The x64 C++ toolset required by the runtime stage is present.",
            )
            .version(version)
            .path(path)
            .affects(&["runtime"]),
        ),
        None => reports.push(
            DependencyReport::new(
                "vs-build-tools",
                "Visual Studio 2022 C++ tools",
                DependencyKind::Required,
                DependencyStatus::Missing,
                "vswhere could not find the 'Desktop development with C++' workload.",
            )
            .required("VS 2022 + C++")
            .affects(&["runtime"])
            .fix(
                FixSuggestion::new(
                    "Install Visual Studio 2022 Build Tools with the C++ workload and Windows SDK.",
                )
                .url("https://visualstudio.microsoft.com/downloads/")
                .winget("Microsoft.VisualStudio.2022.BuildTools")
                .notes(
                    "The workload must include Microsoft.VisualStudio.Component.VC.Tools.x86.x64.",
                ),
            ),
        ),
    }

    // Windows SDK ------------------------------------------------------------
    let sdk_versions = windows_sdk_versions();
    let sdk_ok = sdk_versions
        .first()
        .map(|v| crate::version::compare_versions(v, windows_sdk_min) != std::cmp::Ordering::Less)
        .unwrap_or(false);
    if sdk_ok {
        reports.push(
            DependencyReport::new(
                "windows-sdk",
                "Windows SDK",
                DependencyKind::Required,
                DependencyStatus::Installed,
                "The runtime stage needs the Windows SDK that declares 1.2 root signatures.",
            )
            .version(sdk_versions[0].clone())
            .required(windows_sdk_min)
            .affects(&["runtime"]),
        );
    } else {
        reports.push(
            DependencyReport::new(
                "windows-sdk",
                "Windows SDK",
                DependencyKind::Required,
                DependencyStatus::if_missing_or_old(sdk_versions.first()),
                match sdk_versions.first() {
                    Some(v) => format!(
                        "Windows SDK {v} is installed; {windows_sdk_min} or newer is required."
                    ),
                    None => "No Windows SDK 10 installations were found under Windows Kits\\10."
                        .to_string(),
                },
            )
            .required(windows_sdk_min)
            .path(sdk_versions.first().cloned().unwrap_or_default())
            .affects(&["runtime"])
            .fix(
                FixSuggestion::new(
                    "Install the Windows 10/11 SDK through the Visual Studio Installer.",
                )
                .notes("Individual components → Windows SDK 10.0.26100 or newer.")
                .url("https://developer.microsoft.com/windows/downloads/windows-sdk/"),
            ),
        );
    }

    // FFmpeg (optional) ------------------------------------------------------
    match run_capture(&CommandSpec::new("ffmpeg").arg("-version"), PROBE_TIMEOUT).await {
        Ok(out) if out.success() => {
            let version = out
                .stdout
                .lines()
                .next()
                .and_then(Version::parse)
                .map(|v| v.short())
                .unwrap_or_default();
            reports.push(
                DependencyReport::new(
                    "ffmpeg",
                    "FFmpeg",
                    DependencyKind::Optional,
                    DependencyStatus::Installed,
                    "Available for the --wav option and XMA2 audio decoding.",
                )
                .version(version)
                .affects(&["assets"]),
            );
        }
        _ => reports.push(
            DependencyReport::new(
                "ffmpeg",
                "FFmpeg",
                DependencyKind::Optional,
                DependencyStatus::Missing,
                "Optional. Needed only for --wav or when XMA2 streams must be decoded.",
            )
            .affects(&["assets"])
            .fix(
                FixSuggestion::new("Install FFmpeg and add it to PATH.")
                    .url("https://ffmpeg.org/download.html")
                    .winget("Gyan.FFmpeg"),
            ),
        ),
    }

    // CMake (optional on Windows) -------------------------------------------
    let cmake = run_capture(&CommandSpec::new("cmake").arg("--version"), PROBE_TIMEOUT)
        .await
        .ok();
    match cmake {
        Some(out) if out.success() => reports.push(
            DependencyReport::new(
                "cmake",
                "CMake",
                DependencyKind::Optional,
                DependencyStatus::Installed,
                "Only used for the non-Windows vk_pipeline_check path.",
            )
            .version(
                Version::parse(&out.stdout)
                    .map(|v| v.short())
                    .unwrap_or_default(),
            ),
        ),
        _ => reports.push(
            DependencyReport::new(
                "cmake",
                "CMake",
                DependencyKind::Optional,
                DependencyStatus::Missing,
                "Not required on Windows. The pipelines stage uses the runtime build instead.",
            )
            .affects(&["pipelines"]),
        ),
    }

    // Long path support ------------------------------------------------------
    if cfg!(windows) {
        match long_paths_enabled() {
            Some(true) => reports.push(DependencyReport::new(
                "long-paths",
                "Windows long paths",
                DependencyKind::Recommended,
                DependencyStatus::Installed,
                "LongPathsEnabled is set, so deep asset export folders are safe.",
            )),
            Some(false) => reports.push(
                DependencyReport::new(
                    "long-paths",
                    "Windows long paths",
                    DependencyKind::Recommended,
                    DependencyStatus::Invalid,
                    "LongPathsEnabled is off. Asset exports can exceed MAX_PATH and fail late in a build.",
                )
                .affects(&["assets"])
                .fix(FixSuggestion::new("Enable long path support and reboot.").notes("Set HKLM\\SYSTEM\\CurrentControlSet\\Control\\FileSystem\\LongPathsEnabled to 1, or run: reg add HKLM\\SYSTEM\\CurrentControlSet\\Control\\FileSystem /v LongPathsEnabled /t REG_DWORD /d 1 /f")),
            ),
            None => reports.push(DependencyReport::new(
                "long-paths",
                "Windows long paths",
                DependencyKind::Recommended,
                DependencyStatus::Installed,
                "LongPathsEnabled is not set explicitly; Windows 10 1607+ usually handles the paths Leonida writes.",
            )),
        }
    }

    // winget (for guided installs) ------------------------------------------
    if let Ok(out) = run_capture(&CommandSpec::new("winget").arg("--version"), PROBE_TIMEOUT).await
    {
        if out.success() {
            reports.push(
                DependencyReport::new(
                    "winget",
                    "winget",
                    DependencyKind::Optional,
                    DependencyStatus::Installed,
                    "Guided installations can use winget.",
                )
                .version(
                    Version::parse(&out.stdout)
                        .map(|v| v.short())
                        .unwrap_or_default(),
                ),
            );
        }
    }

    reports
}

impl DependencyStatus {
    fn if_missing_or_old(found: Option<&String>) -> DependencyStatus {
        if found.is_some() {
            DependencyStatus::Outdated
        } else {
            DependencyStatus::Missing
        }
    }
}

/// Map a winget package id to the dependency it installs, with the extra
/// arguments some packages need to be useful for Leonida.
pub fn winget_install_args(package_id: &str) -> AppResult<Vec<String>> {
    let allowed = [
        "Python.Python.3.12",
        "Git.Git",
        "Microsoft.DotNet.SDK.8",
        "Microsoft.VisualStudio.2022.BuildTools",
        "KhronosGroup.VulkanSDK",
        "LLVM.LLVM",
        "Gyan.FFmpeg",
        "Kitware.CMake",
    ];
    if !allowed.contains(&package_id) {
        return Err(crate::error::AppError::new(
            "not_allowed",
            "That package is not on the launcher's allow-list",
        ));
    }
    let mut args = vec![
        "install".to_string(),
        "--id".to_string(),
        package_id.to_string(),
        "-e".to_string(),
        "--accept-package-agreements".to_string(),
        "--accept-source-agreements".to_string(),
        "--disable-interactivity".to_string(),
    ];
    if package_id == "Microsoft.VisualStudio.2022.BuildTools" {
        args.push("--override".into());
        args.push(
            "--quiet --wait --norestart --nocache --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
                .into(),
        );
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_quoted_command_lines() {
        assert_eq!(
            split_command_line(r#""C:\Program Files\LLVM\bin\llvm-mc.exe" --version"#),
            vec![r"C:\Program Files\LLVM\bin\llvm-mc.exe", "--version"]
        );
        assert_eq!(
            split_command_line("wsl -d Ubuntu -e /usr/lib/llvm-21/bin/llvm-mc"),
            vec!["wsl", "-d", "Ubuntu", "-e", "/usr/lib/llvm-21/bin/llvm-mc"]
        );
    }

    #[test]
    fn winget_arguments_are_allow_listed() {
        assert!(winget_install_args("evil.package").is_err());
        let args = winget_install_args("Python.Python.3.12").unwrap();
        assert!(args.contains(&"Python.Python.3.12".to_string()));
        assert!(args.contains(&"--disable-interactivity".to_string()));
    }

    #[test]
    fn build_tools_override_is_added_for_visual_studio() {
        let args = winget_install_args("Microsoft.VisualStudio.2022.BuildTools").unwrap();
        assert!(args
            .iter()
            .any(|a| a.contains("Microsoft.VisualStudio.Workload.VCTools")));
    }

    #[test]
    fn blocking_dep_requires_required_kind() {
        let mut report = DependencyReport::new(
            "x",
            "X",
            DependencyKind::Required,
            DependencyStatus::Missing,
            "",
        );
        assert!(report.blocks());
        report.kind = DependencyKind::Optional;
        assert!(!report.blocks());
    }

    #[test]
    fn python_discovery_never_panics() {
        let installs = discover_python_installs();
        assert!(installs.iter().all(|p| p.ends_with("python.exe")));
    }
}
