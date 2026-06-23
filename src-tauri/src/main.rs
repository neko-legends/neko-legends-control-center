#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Mutex, OnceLock},
    thread,
    time::Duration,
};
use tauri::{AppHandle, Manager, PhysicalSize, Size, WebviewWindow, WindowEvent};
#[cfg(windows)]
use winreg::{enums::*, RegKey};

const MIN_WINDOW_WIDTH: u32 = 520;
const MIN_WINDOW_HEIGHT: u32 = 390;
const FALLBACK_WINDOW_WIDTH: u32 = 720;
const FALLBACK_WINDOW_HEIGHT: u32 = 520;
const PORTRAIT_WINDOW_WIDTH_RATIO: f32 = 0.9;
const GITHUB_OWNER: &str = "neko-legends";
const CONTROL_CENTER_REPO: &str = "NekoLegendsControlCenter";
const TOOLS_CATALOG_URL: &str = "https://nekolegends.com/res/nekoLegendsControlCenter/tools.json";
const UNDER_DEVELOPMENT_CATEGORY: &str = "Under Development";
const VENICE_MEDIA_LOCAL_ID: &str = "venice-media-local";
const VENICE_MEDIA_LOCAL_DISPLAY_NAME: &str = "Venice Media Local";
const AGENT_API_REGISTRY_FILE: &str = "agent-api-registry.json";
const MAX_INSTALLED_APP_VERSIONS: usize = 2;

static RUNNING_APPS: OnceLock<Mutex<BTreeMap<String, Child>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LauncherApp {
    id: String,
    name: String,
    repo: String,
    description: String,
    accent: String,
    icon: String,
    #[serde(default = "default_category")]
    category: String,
    executable_path: Option<String>,
    installed_version: Option<String>,
    selected_version: Option<String>,
    #[serde(default)]
    installed_versions: Vec<InstalledVersion>,
    latest_version: Option<String>,
    release_url: Option<String>,
    release_checked_at: Option<String>,
    release_notes: Option<String>,
    #[serde(default)]
    release_options: Vec<ReleaseOption>,
    #[serde(default)]
    package_preference: PackagePreference,
    package_path: Option<String>,
    demo_url: Option<String>,
    #[serde(default)]
    status: ToolStatus,
    #[serde(default = "default_true")]
    visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstalledVersion {
    version: String,
    executable_path: Option<String>,
    package_path: Option<String>,
    installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum ToolStatus {
    Available,
    ComingSoon,
}

impl Default for ToolStatus {
    fn default() -> Self {
        Self::Available
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum PackagePreference {
    Portable,
    Installer,
}

impl Default for PackagePreference {
    fn default() -> Self {
        Self::Portable
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseOption {
    tag_name: String,
    html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    theme: String,
    compact_labels: bool,
    #[serde(default = "default_true")]
    use_remote_catalog: bool,
    #[serde(default)]
    agent_control_auto_start: bool,
    #[serde(default = "default_categories")]
    categories: Vec<String>,
    window_width: Option<u32>,
    window_height: Option<u32>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "neko-tron".to_string(),
            compact_labels: false,
            use_remote_catalog: true,
            agent_control_auto_start: false,
            categories: default_categories(),
            window_width: None,
            window_height: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControlCenterState {
    settings: AppSettings,
    apps: Vec<LauncherApp>,
    build_version: String,
    data_dir: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveSettingsRequest {
    theme: Option<String>,
    compact_labels: Option<bool>,
    use_remote_catalog: Option<bool>,
    agent_control_auto_start: Option<bool>,
    categories: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveExecutableRequest {
    app_id: String,
    executable_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveLayoutRequest {
    apps: Vec<LauncherApp>,
    categories: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LaunchRequest {
    app_id: String,
    version: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchResult {
    apps: Vec<LauncherApp>,
    relaunched: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadRequest {
    app_id: String,
    version: Option<String>,
    package_preference: Option<PackagePreference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum AgentControlAction {
    Status,
    Scan,
    Download,
    Update,
    Launch,
    OpenFolder,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentControlCommand {
    id: Option<String>,
    action: AgentControlAction,
    app_id: Option<String>,
    version: Option<String>,
    package_preference: Option<PackagePreference>,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadResult {
    apps: Vec<LauncherApp>,
    file_path: String,
    install_folder: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentControlInfo {
    root_dir: String,
    inbox_dir: String,
    outbox_dir: String,
    history_dir: String,
    state_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentControlResponse {
    id: Option<String>,
    action: Option<AgentControlAction>,
    app_id: Option<String>,
    ok: bool,
    message: String,
    processed_at: String,
    data: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentControlPollResult {
    processed_count: usize,
    apps: Vec<LauncherApp>,
    info: AgentControlInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentApiRegistryEntry {
    app_id: String,
    app_name: String,
    default_port: u16,
    bind_address: String,
    port: u16,
    enabled: bool,
    url: String,
    openapi_url: String,
    busy: bool,
    active_job_id: Option<String>,
    last_seen: Option<String>,
    note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentApiRegistry {
    updated_at: String,
    apps: Vec<AgentApiRegistryEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentApiPortConflict {
    port: u16,
    app_ids: Vec<String>,
    app_names: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentApiDashboard {
    registry_path: String,
    updated_at: String,
    apps: Vec<AgentApiRegistryEntry>,
    conflicts: Vec<AgentApiPortConflict>,
    next_available_port: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveAgentApiPortRequest {
    app_id: String,
    port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControlCenterUpdate {
    current_version: String,
    latest_version: Option<String>,
    release_url: Option<String>,
    release_notes: Option<String>,
    checked_at: String,
    update_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolsCatalog {
    #[serde(default = "default_catalog_version")]
    catalog_version: u32,
    updated_at: Option<String>,
    tools: Vec<LauncherApp>,
}

fn default_true() -> bool {
    true
}

fn default_catalog_version() -> u32 {
    14
}

fn default_category() -> String {
    "-= Released Work Stuff =-".to_string()
}

fn is_under_development_category(category: &str) -> bool {
    category
        .trim()
        .eq_ignore_ascii_case(UNDER_DEVELOPMENT_CATEGORY)
}

fn is_coming_soon_app(launcher_app: &LauncherApp) -> bool {
    launcher_app.status == ToolStatus::ComingSoon
        || is_under_development_category(&launcher_app.category)
}

fn normalize_development_status(launcher_app: &mut LauncherApp) {
    if is_coming_soon_app(launcher_app) {
        launcher_app.status = ToolStatus::ComingSoon;
        launcher_app.category = UNDER_DEVELOPMENT_CATEGORY.to_string();
    }
}

fn clear_coming_soon_release_state(launcher_app: &mut LauncherApp) {
    launcher_app.latest_version = None;
    launcher_app.release_url = None;
    launcher_app.release_checked_at = None;
    launcher_app.release_notes = Some("Coming soon.".to_string());
    launcher_app.release_options = Vec::new();
}

fn default_categories() -> Vec<String> {
    vec![
        "-= Released Work Stuff =-".to_string(),
        "Fun Stuff".to_string(),
        UNDER_DEVELOPMENT_CATEGORY.to_string(),
    ]
}

fn normalize_categories(categories: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for category in categories {
        let trimmed = category.trim();
        if !trimmed.is_empty() && !normalized.iter().any(|existing| existing == trimmed) {
            normalized.push(trimmed.to_string());
        }
    }
    if normalized.is_empty() {
        default_categories()
    } else {
        normalized
    }
}

fn default_apps() -> Vec<LauncherApp> {
    vec![
        app("asset-vault", "Asset Vault", "AssetVault", "Local-first library for AI-generated game assets: import, triage, dedupe, search, export.", "#c9a04e", "AV", "-= Released Work Stuff =-", ToolStatus::ComingSoon, None),
        app("batchlapse", "BatchLapse", "BatchLapse", "Batch video timelapse exporter for MP4, WebM, and GitHub-friendly GIFs.", "#5b8def", "BL", "-= Released Work Stuff =-", ToolStatus::Available, None),
        app("cutscene-converter", "Cutscene Converter", "CutsceneConverter", "Godot-friendly cutscene video converter for MP4, WebM, and OGV.", "#f06f48", "CC", "-= Released Work Stuff =-", ToolStatus::Available, None),
        app("depth-map-ai-generator", "DepthMap AI", "DepthMapAIGenerator", "Batch depth-map and WebP generator for local AI image workflows.", "#43b883", "DM", UNDER_DEVELOPMENT_CATEGORY, ToolStatus::ComingSoon, None),
        app("image-to-ascii-3d", "ASCII 3D", "ImageToASCII3D", "Image-to-ASCII converter with optional depth-map driven 3D parallax exports.", "#f0a848", "A3", UNDER_DEVELOPMENT_CATEGORY, ToolStatus::ComingSoon, None),
        app("image-to-3d", "Image to 3D", "ImageTo3D", "Local image-to-3D workflow for mesh, texture, and 3D asset generation.", "#8c65df", "I3", UNDER_DEVELOPMENT_CATEGORY, ToolStatus::Available, None),
        app("multi-angle-edit", "Multi-Angle Edit", "MultiAngleEdit", "Local multi-angle image editor: re-render a photo from a new camera angle with Qwen-Image-Edit + the Multiple-Angles LoRA on your own GPU.", "#b14bff", "MA", UNDER_DEVELOPMENT_CATEGORY, ToolStatus::ComingSoon, None),
        app("image-to-splat", "ImageToSplat", "ImageToSplat", "Local TripoSplat workflow for turning a single image into Gaussian splat and point-cloud 3D exports.", "#55c7f7", "IS", UNDER_DEVELOPMENT_CATEGORY, ToolStatus::ComingSoon, None),
        app("splatscape", "SplatScape", "SplatScape", "Portable FPS-style explorer for 3D Gaussian splat scenes with WASD and mouse-look navigation.", "#7adfbb", "SS", UNDER_DEVELOPMENT_CATEGORY, ToolStatus::ComingSoon, None),
        app("markrush", "MarkRush", "MarkRush", "Fast local Markdown viewer/editor built for huge files and folders.", "#e05d7b", "MR", "-= Released Work Stuff =-", ToolStatus::Available, None),
        app("opensplit", "OpenSplit", "OpenSplit", "Multi-pane terminal harness for AI coding agents, shells, and SSH sessions.", "#4fb6d8", "OS", "-= Released Work Stuff =-", ToolStatus::Available, None),
        app("seamless-image-edit", "Seamless Image Edit", "SeamlessImageEdit", "Local image tiling and seamless texture prep for game art workflows.", "#d889ff", "SI", "-= Released Work Stuff =-", ToolStatus::Available, None),
        app("venice-media-local", "Venice Media", "VeniceMediaLocal", "Local Venice API media workspace for images, video, music, voice, and cleanup.", "#34c6a3", "VM", "-= Released Work Stuff =-", ToolStatus::Available, None),
        app("purpleplanet", "PurplePlanet", "PurplePlanet", "Luminous Three.js planet motion art for live wallpapers and screensavers.", "#8c65df", "PP", "Fun Stuff", ToolStatus::Available, Some("https://nekolegends.com/res/projects/purplePlanet/")),
        app("stargaze", "StarGaze", "StarGaze", "Glittering Three.js starfield wallpaper and screensaver with tunable motion.", "#6b7cff", "SG", "Fun Stuff", ToolStatus::Available, Some("https://nekolegends.com/res/projects/starGaze/")),
    ]
}

fn app(
    id: &str,
    name: &str,
    repo: &str,
    description: &str,
    accent: &str,
    icon: &str,
    category: &str,
    status: ToolStatus,
    demo_url: Option<&str>,
) -> LauncherApp {
    LauncherApp {
        id: id.to_string(),
        name: name.to_string(),
        repo: repo.to_string(),
        description: description.to_string(),
        accent: accent.to_string(),
        icon: icon.to_string(),
        category: category.to_string(),
        executable_path: None,
        installed_version: None,
        selected_version: None,
        installed_versions: Vec::new(),
        latest_version: None,
        release_url: None,
        release_checked_at: None,
        release_notes: None,
        release_options: Vec::new(),
        package_preference: PackagePreference::Portable,
        package_path: None,
        demo_url: demo_url.map(str::to_string),
        status,
        visible: true,
    }
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|err| err.to_string())?;
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir)
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("settings.json"))
}

fn apps_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("apps.json"))
}

fn tools_catalog_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("tools-catalog.json"))
}

fn shared_neko_legends_dir() -> Result<PathBuf, String> {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join("Library").join("Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
    }
    .ok_or_else(|| "Unable to resolve user data folder.".to_string())?;

    let dir = base.join("NekoLegends");
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir)
}

fn agent_api_registry_path() -> Result<PathBuf, String> {
    Ok(shared_neko_legends_dir()?.join(AGENT_API_REGISTRY_FILE))
}

fn read_json_file<T>(path: &Path, fallback: T) -> T
where
    T: for<'de> Deserialize<'de>,
{
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn write_json_file<T>(path: &Path, value: &T) -> Result<(), String>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let raw = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;
    fs::write(path, raw).map_err(|err| err.to_string())
}

fn agent_api_url(bind_address: &str, port: u16) -> String {
    let host = if bind_address == "0.0.0.0" {
        "127.0.0.1"
    } else {
        bind_address
    };
    format!("http://{host}:{port}")
}

fn default_agent_api_entry(
    app_id: &str,
    app_name: &str,
    default_port: u16,
    bind_address: &str,
    note: &str,
) -> AgentApiRegistryEntry {
    let url = agent_api_url(bind_address, default_port);
    AgentApiRegistryEntry {
        app_id: app_id.to_string(),
        app_name: app_name.to_string(),
        default_port,
        bind_address: bind_address.to_string(),
        port: default_port,
        enabled: false,
        url: url.clone(),
        openapi_url: format!("{url}/openapi.json"),
        busy: false,
        active_job_id: None,
        last_seen: None,
        note: Some(note.to_string()),
    }
}

fn default_agent_api_entries() -> Vec<AgentApiRegistryEntry> {
    vec![
        default_agent_api_entry(
            "venice-media-local",
            "Venice Media Local",
            9876,
            "0.0.0.0",
            "Remote-control API with token flow.",
        ),
        default_agent_api_entry(
            "image-to-3d",
            "ImageTo3D",
            17333,
            "127.0.0.1",
            "Local Agent API.",
        ),
        default_agent_api_entry(
            "depth-map-ai-generator",
            "DepthMap AI Generator",
            17334,
            "127.0.0.1",
            "Local Agent API.",
        ),
        default_agent_api_entry(
            "seamless-image-edit",
            "Seamless Image Edit",
            17335,
            "127.0.0.1",
            "Local Agent API.",
        ),
        default_agent_api_entry(
            "batchlapse",
            "BatchLapse",
            17336,
            "127.0.0.1",
            "Local Agent API.",
        ),
        default_agent_api_entry(
            "cutscene-converter",
            "Cutscene Converter",
            17337,
            "127.0.0.1",
            "Local Agent API.",
        ),
        default_agent_api_entry(
            "asset-vault",
            "Asset Vault",
            17338,
            "127.0.0.1",
            "Library/search API.",
        ),
        default_agent_api_entry(
            "multi-angle-edit",
            "Multi-Angle Edit",
            17339,
            "127.0.0.1",
            "Local Agent API.",
        ),
        default_agent_api_entry(
            "image-to-splat",
            "ImageToSplat",
            17340,
            "127.0.0.1",
            "Local Agent API.",
        ),
        default_agent_api_entry(
            "sprite-atlas-packer",
            "Sprite Atlas Packer",
            9877,
            "127.0.0.1",
            "Pack/slice control API.",
        ),
    ]
}

fn read_agent_api_registry() -> Result<AgentApiRegistry, String> {
    let path = agent_api_registry_path()?;
    let fallback = AgentApiRegistry {
        updated_at: Utc::now().to_rfc3339(),
        apps: Vec::new(),
    };
    Ok(read_json_file(&path, fallback))
}

fn merged_agent_api_entries(registry: &AgentApiRegistry) -> Vec<AgentApiRegistryEntry> {
    let mut entries = default_agent_api_entries();

    for saved in &registry.apps {
        if let Some(existing) = entries
            .iter_mut()
            .find(|entry| entry.app_id == saved.app_id)
        {
            let mut merged = saved.clone();
            merged.app_name = if merged.app_name.trim().is_empty() {
                existing.app_name.clone()
            } else {
                merged.app_name
            };
            merged.default_port = existing.default_port;
            merged.bind_address = if merged.bind_address.trim().is_empty() {
                existing.bind_address.clone()
            } else {
                merged.bind_address
            };
            merged.url = agent_api_url(&merged.bind_address, merged.port);
            merged.openapi_url = format!("{}/openapi.json", merged.url);
            if merged.note.is_none() {
                merged.note = existing.note.clone();
            }
            *existing = merged;
        } else {
            entries.push(saved.clone());
        }
    }

    entries.sort_by_key(|entry| entry.default_port);
    entries
}

fn agent_api_conflicts(entries: &[AgentApiRegistryEntry]) -> Vec<AgentApiPortConflict> {
    let mut by_port: BTreeMap<u16, Vec<&AgentApiRegistryEntry>> = BTreeMap::new();
    for entry in entries {
        by_port.entry(entry.port).or_default().push(entry);
    }
    by_port
        .into_iter()
        .filter_map(|(port, matches)| {
            if matches.len() < 2 {
                return None;
            }
            Some(AgentApiPortConflict {
                port,
                app_ids: matches.iter().map(|entry| entry.app_id.clone()).collect(),
                app_names: matches.iter().map(|entry| entry.app_name.clone()).collect(),
            })
        })
        .collect()
}

fn next_available_agent_port(entries: &[AgentApiRegistryEntry]) -> u16 {
    let used = entries.iter().map(|entry| entry.port).collect::<Vec<_>>();
    (17333..=17499)
        .find(|port| !used.contains(port))
        .unwrap_or(17500)
}

fn write_agent_api_registry(
    entries: Vec<AgentApiRegistryEntry>,
) -> Result<AgentApiRegistry, String> {
    let registry = AgentApiRegistry {
        updated_at: Utc::now().to_rfc3339(),
        apps: entries,
    };
    let path = agent_api_registry_path()?;
    with_agent_api_registry_lock(&path, || write_json_file(&path, &registry))?;
    Ok(registry)
}

fn with_agent_api_registry_lock<F>(path: &Path, mut write: F) -> Result<(), String>
where
    F: FnMut() -> Result<(), String>,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let lock_path = path.with_extension("json.lock");
    for _ in 0..3 {
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(_) => {
                let result = write();
                let _ = fs::remove_file(&lock_path);
                return result;
            }
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                let stale = fs::metadata(&lock_path)
                    .and_then(|metadata| metadata.modified())
                    .ok()
                    .and_then(|modified| modified.elapsed().ok())
                    .map(|elapsed| elapsed > Duration::from_secs(2))
                    .unwrap_or(false);
                if stale {
                    let _ = fs::remove_file(&lock_path);
                } else {
                    thread::sleep(Duration::from_millis(100));
                }
            }
            Err(err) => return Err(err.to_string()),
        }
    }
    Err("Agent API registry is busy.".to_string())
}

fn agent_api_dashboard_from(registry: AgentApiRegistry) -> Result<AgentApiDashboard, String> {
    let entries = merged_agent_api_entries(&registry);
    let conflicts = agent_api_conflicts(&entries);
    let next_available_port = next_available_agent_port(&entries);
    Ok(AgentApiDashboard {
        registry_path: agent_api_registry_path()?.to_string_lossy().to_string(),
        updated_at: registry.updated_at,
        apps: entries,
        conflicts,
        next_available_port,
    })
}

fn builtin_tools_catalog() -> ToolsCatalog {
    ToolsCatalog {
        catalog_version: default_catalog_version(),
        updated_at: Some("built-in".to_string()),
        tools: default_apps(),
    }
}

fn clean_catalog_id(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty()
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
    {
        return None;
    }
    Some(value)
}

fn clean_catalog_repo(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches('/');
    let repo = trimmed
        .strip_prefix(&format!("{}/", GITHUB_OWNER))
        .unwrap_or(trimmed)
        .trim();
    if repo.is_empty()
        || repo.contains('/')
        || repo.contains('\\')
        || repo.contains("..")
        || !repo.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return None;
    }
    Some(repo.to_string())
}

fn clean_optional_https_url(value: Option<String>) -> Option<String> {
    value
        .map(|url| url.trim().to_string())
        .filter(|url| url.starts_with("https://"))
}

fn clean_hex_color(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.len() == 7
        && value.starts_with('#')
        && value
            .chars()
            .skip(1)
            .all(|character| character.is_ascii_hexdigit())
    {
        value.to_string()
    } else {
        fallback.to_string()
    }
}

fn default_category_if_empty(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        default_category()
    } else {
        value.to_string()
    }
}

fn clean_catalog_tool(mut launcher_app: LauncherApp) -> Option<LauncherApp> {
    launcher_app.id = clean_catalog_id(&launcher_app.id)?;
    launcher_app.repo = clean_catalog_repo(&launcher_app.repo)?;
    launcher_app.name = launcher_app.name.trim().to_string();
    if launcher_app.name.is_empty() {
        launcher_app.name = launcher_app.repo.clone();
    }
    launcher_app.description = launcher_app.description.trim().to_string();
    if launcher_app.description.is_empty() {
        launcher_app.description = "Neko Legends tool.".to_string();
    }
    launcher_app.accent = clean_hex_color(&launcher_app.accent, "#ff6a00");
    launcher_app.icon = launcher_app.icon.trim().chars().take(4).collect();
    if launcher_app.icon.is_empty() {
        launcher_app.icon = launcher_app
            .name
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .take(2)
            .collect::<String>()
            .to_ascii_uppercase();
    }
    launcher_app.category = default_category_if_empty(&launcher_app.category);
    normalize_development_status(&mut launcher_app);
    launcher_app.demo_url = clean_optional_https_url(launcher_app.demo_url);
    launcher_app.executable_path = None;
    launcher_app.installed_version = None;
    launcher_app.selected_version = None;
    launcher_app.installed_versions = Vec::new();
    launcher_app.latest_version = None;
    launcher_app.release_url = None;
    launcher_app.release_checked_at = None;
    launcher_app.release_notes = None;
    launcher_app.release_options = Vec::new();
    launcher_app.package_path = None;
    Some(launcher_app)
}

fn clean_tools_catalog(catalog: ToolsCatalog) -> Result<ToolsCatalog, String> {
    let mut tools = Vec::new();
    for launcher_app in catalog.tools {
        if let Some(launcher_app) = clean_catalog_tool(launcher_app) {
            if !tools
                .iter()
                .any(|existing: &LauncherApp| existing.id == launcher_app.id)
            {
                tools.push(launcher_app);
            }
        }
    }
    if tools.is_empty() {
        return Err("Tools catalog did not include any usable tools.".to_string());
    }
    Ok(ToolsCatalog {
        catalog_version: catalog.catalog_version,
        updated_at: catalog.updated_at,
        tools,
    })
}

fn read_tools_catalog(app: &AppHandle) -> ToolsCatalog {
    let fallback = builtin_tools_catalog();
    let catalog = tools_catalog_path(app)
        .map(|path| read_json_file(&path, fallback.clone()))
        .unwrap_or(fallback.clone());
    match clean_tools_catalog(catalog) {
        Ok(catalog) if catalog.catalog_version >= fallback.catalog_version => catalog,
        _ => fallback,
    }
}

fn read_local_tools_catalog() -> Option<ToolsCatalog> {
    let manifest_catalog = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("catalog")
        .join("tools.json");
    let mut candidates = vec![manifest_catalog];

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join("catalog").join("tools.json"));
        candidates.push(current_dir.join("..").join("catalog").join("tools.json"));
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join("catalog").join("tools.json"));
            candidates.push(exe_dir.join("..").join("catalog").join("tools.json"));
            candidates.push(
                exe_dir
                    .join("..")
                    .join("..")
                    .join("catalog")
                    .join("tools.json"),
            );
        }
    }

    for candidate in candidates {
        if !candidate.exists() {
            continue;
        }
        if let Ok(raw) = fs::read_to_string(&candidate) {
            if let Ok(catalog) = serde_json::from_str::<ToolsCatalog>(&raw) {
                if let Ok(catalog) = clean_tools_catalog(catalog) {
                    return Some(catalog);
                }
            }
        }
    }

    None
}

fn read_effective_tools_catalog(app: &AppHandle) -> ToolsCatalog {
    let settings = read_settings(app);
    if settings.use_remote_catalog {
        read_tools_catalog(app)
    } else {
        read_local_tools_catalog().unwrap_or_else(builtin_tools_catalog)
    }
}

fn read_settings(app: &AppHandle) -> AppSettings {
    let mut settings = settings_path(app)
        .map(|path| read_json_file(&path, AppSettings::default()))
        .unwrap_or_default();
    if settings.theme == "eva-dark" {
        settings.theme = "neko-tron".to_string();
    }
    settings.categories = normalize_categories(settings.categories);
    settings
}

fn read_apps(app: &AppHandle) -> Vec<LauncherApp> {
    let saved = apps_path(app)
        .map(|path| read_json_file(&path, Vec::<LauncherApp>::new()))
        .unwrap_or_default();
    let mut apps = merge_catalog_apps(saved, read_effective_tools_catalog(app).tools);
    auto_detect_installed_apps(&mut apps);
    apps
}

fn merge_catalog_apps(saved: Vec<LauncherApp>, defaults: Vec<LauncherApp>) -> Vec<LauncherApp> {
    let mut merged = Vec::new();

    for saved_app in saved {
        if let Some(default_app) = defaults
            .iter()
            .find(|candidate| candidate.id == saved_app.id)
        {
            let mut app = default_app.clone();
            app.executable_path = saved_app.executable_path;
            app.installed_version = saved_app.installed_version;
            app.selected_version = saved_app.selected_version;
            app.installed_versions = saved_app.installed_versions;
            app.latest_version = saved_app.latest_version;
            app.release_url = saved_app.release_url;
            app.release_checked_at = saved_app.release_checked_at;
            app.release_notes = saved_app.release_notes;
            app.release_options = saved_app.release_options;
            app.package_preference = saved_app.package_preference;
            app.package_path = saved_app.package_path;
            app.visible = saved_app.visible;
            if !saved_app.category.trim().is_empty() {
                app.category = saved_app.category;
            }
            normalize_development_status(&mut app);
            if app.status == ToolStatus::ComingSoon {
                clear_coming_soon_release_state(&mut app);
            }
            normalize_install_state(&mut app);
            merged.push(app);
        }
    }

    for mut default_app in defaults {
        if !merged
            .iter()
            .any(|candidate| candidate.id == default_app.id)
        {
            normalize_development_status(&mut default_app);
            if default_app.status == ToolStatus::ComingSoon {
                clear_coming_soon_release_state(&mut default_app);
            }
            normalize_install_state(&mut default_app);
            merged.push(default_app);
        }
    }

    merged
}

fn save_apps(app: &AppHandle, apps: &[LauncherApp]) -> Result<(), String> {
    let path = apps_path(app)?;
    write_json_file(&path, &apps)
}

#[derive(Debug, Clone)]
struct DetectedInstall {
    executable_path: Option<PathBuf>,
    package_path: Option<PathBuf>,
    version: Option<String>,
}

fn auto_detect_installed_apps(apps: &mut [LauncherApp]) {
    for launcher_app in apps.iter_mut() {
        if app_download_artifact_exists(launcher_app) {
            continue;
        }
        if let Some(install) =
            detect_local_install(launcher_app).or_else(|| detect_installed_app(launcher_app))
        {
            let detected_version = install.version.clone();
            if let Some(executable_path) = install.executable_path {
                launcher_app.executable_path = Some(executable_path.to_string_lossy().to_string());
            }
            if let Some(package_path) = install.package_path {
                launcher_app.package_path = Some(package_path.to_string_lossy().to_string());
            }
            if let Some(version) = detected_version {
                launcher_app.installed_version = Some(version.clone());
                let executable_path = launcher_app.executable_path.clone().map(PathBuf::from);
                let package_path = launcher_app.package_path.clone().map(PathBuf::from);
                add_installed_version(
                    launcher_app,
                    version,
                    executable_path.as_deref(),
                    package_path.as_deref(),
                );
            }
        }
    }
}

fn path_option_exists(path: &Option<String>) -> bool {
    path.as_deref().is_some_and(|path| Path::new(path).exists())
}

fn app_download_artifact_exists(launcher_app: &LauncherApp) -> bool {
    if launcher_app.demo_url.is_some() {
        path_option_exists(&launcher_app.package_path)
    } else {
        path_option_exists(&launcher_app.executable_path)
    }
}

fn installed_version_artifact_exists(
    launcher_app: &LauncherApp,
    installed_version: &InstalledVersion,
) -> bool {
    if launcher_app.demo_url.is_some() {
        path_option_exists(&installed_version.package_path)
    } else {
        path_option_exists(&installed_version.executable_path)
    }
}

fn apply_installed_version(launcher_app: &mut LauncherApp, installed_version: &InstalledVersion) {
    launcher_app.installed_version = Some(installed_version.version.clone());
    launcher_app.executable_path = installed_version.executable_path.clone();
    launcher_app.package_path = installed_version.package_path.clone();
}

fn add_installed_version(
    launcher_app: &mut LauncherApp,
    version: String,
    executable_path: Option<&Path>,
    package_path: Option<&Path>,
) {
    launcher_app
        .installed_versions
        .retain(|installed| installed.version != version);
    launcher_app.installed_versions.insert(
        0,
        InstalledVersion {
            version,
            executable_path: executable_path.map(|path| path.to_string_lossy().to_string()),
            package_path: package_path.map(|path| path.to_string_lossy().to_string()),
            installed_at: Utc::now().to_rfc3339(),
        },
    );
}

fn normalize_install_state(launcher_app: &mut LauncherApp) {
    let demo_app = launcher_app.demo_url.is_some();
    launcher_app.installed_versions.retain(|installed| {
        if demo_app {
            path_option_exists(&installed.package_path)
        } else {
            path_option_exists(&installed.executable_path)
        }
    });

    if let Some(installed_version) = launcher_app.installed_version.clone() {
        if app_download_artifact_exists(launcher_app)
            && !launcher_app
                .installed_versions
                .iter()
                .any(|installed| installed.version == installed_version)
        {
            let executable_path = launcher_app.executable_path.clone().map(PathBuf::from);
            let package_path = launcher_app.package_path.clone().map(PathBuf::from);
            add_installed_version(
                launcher_app,
                installed_version,
                executable_path.as_deref(),
                package_path.as_deref(),
            );
        }
    }

    let preferred_version = launcher_app
        .selected_version
        .as_ref()
        .or(launcher_app.installed_version.as_ref())
        .cloned();
    let selected_install = preferred_version
        .as_deref()
        .and_then(|version| {
            launcher_app
                .installed_versions
                .iter()
                .find(|installed| installed.version == version)
                .cloned()
        })
        .or_else(|| launcher_app.installed_versions.first().cloned());

    if let Some(installed) = selected_install {
        apply_installed_version(launcher_app, &installed);
    }

    if launcher_app.installed_versions.len() > MAX_INSTALLED_APP_VERSIONS {
        launcher_app
            .installed_versions
            .truncate(MAX_INSTALLED_APP_VERSIONS);
    }
}

fn select_installed_version(launcher_app: &mut LauncherApp, version: &str) -> Result<(), String> {
    let installed = launcher_app
        .installed_versions
        .iter()
        .find(|installed| installed.version == version)
        .cloned()
        .ok_or_else(|| "Selected version is not downloaded yet.".to_string())?;
    if !installed_version_artifact_exists(launcher_app, &installed) {
        return Err("Selected version is no longer on disk.".to_string());
    }
    apply_installed_version(launcher_app, &installed);
    launcher_app.selected_version = Some(version.to_string());
    Ok(())
}

fn detect_local_install(launcher_app: &LauncherApp) -> Option<DetectedInstall> {
    let root = default_install_dir().ok()?.join(&launcher_app.id);
    if !root.exists() {
        return None;
    }

    if launcher_app.demo_url.is_some() {
        let package_path = find_best_package_archive(&root, &launcher_app.id, &launcher_app.repo)
            .ok()
            .flatten()?;
        return Some(DetectedInstall {
            version: version_from_install_path(&root, &package_path),
            executable_path: None,
            package_path: Some(package_path),
        });
    }

    let launch_path = find_best_launch_path(&root, &launcher_app.id, &launcher_app.repo)
        .ok()
        .flatten()?;
    cleanup_redundant_archives_near_launch_path(&launch_path);

    Some(DetectedInstall {
        version: version_from_install_path(&root, &launch_path),
        executable_path: Some(launch_path),
        package_path: None,
    })
}

fn version_from_install_path(root: &Path, launch_path: &Path) -> Option<String> {
    let relative = launch_path.parent()?.strip_prefix(root).ok()?;
    relative
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

#[cfg(not(windows))]
fn detect_installed_app(_launcher_app: &LauncherApp) -> Option<DetectedInstall> {
    None
}

#[cfg(windows)]
fn detect_installed_app(launcher_app: &LauncherApp) -> Option<DetectedInstall> {
    let hives = [
        (
            RegKey::predef(HKEY_CURRENT_USER),
            "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ),
        (
            RegKey::predef(HKEY_LOCAL_MACHINE),
            "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ),
        (
            RegKey::predef(HKEY_LOCAL_MACHINE),
            "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        ),
    ];

    for (hive, uninstall_path) in hives {
        let Ok(uninstall_root) = hive.open_subkey(uninstall_path) else {
            continue;
        };

        for subkey_name in uninstall_root.enum_keys().flatten() {
            let Ok(app_key) = uninstall_root.open_subkey(subkey_name) else {
                continue;
            };
            let display_name: String = app_key.get_value("DisplayName").unwrap_or_default();
            if registry_display_name_matches_app(&display_name, launcher_app) {
                if let Some(install) = detected_install_from_registry_key(&app_key, launcher_app) {
                    return Some(install);
                }
            }
        }
    }

    None
}

#[cfg(windows)]
fn detected_install_from_registry_key(
    key: &RegKey,
    launcher_app: &LauncherApp,
) -> Option<DetectedInstall> {
    let version = key
        .get_value::<String, _>("DisplayVersion")
        .ok()
        .filter(|value| !value.trim().is_empty());

    for candidate in registry_executable_candidates(key, launcher_app) {
        if candidate.exists() {
            return Some(DetectedInstall {
                executable_path: Some(candidate),
                package_path: None,
                version,
            });
        }
    }

    None
}

#[cfg(windows)]
fn registry_executable_candidates(key: &RegKey, launcher_app: &LauncherApp) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(value) = key.get_value::<String, _>("DisplayIcon") {
        if let Some(path) = command_path_from_registry_value(&value) {
            if is_launchable_executable_path(&path) {
                candidates.push(path);
            }
        }
    }

    if let Ok(install_location) = key.get_value::<String, _>("InstallLocation") {
        if let Some(path) = command_path_from_registry_value(&install_location) {
            if is_launchable_executable_path(&path) {
                candidates.push(path);
            } else {
                candidates.extend(
                    expected_executable_names(launcher_app)
                        .into_iter()
                        .map(|name| path.join(name)),
                );
                if let Ok(Some(best)) =
                    find_best_executable(&path, &launcher_app.id, &launcher_app.repo)
                {
                    candidates.push(best);
                }
            }
        }
    }

    if let Ok(value) = key.get_value::<String, _>("UninstallString") {
        if let Some(path) = command_path_from_registry_value(&value) {
            if let Some(parent) = path.parent() {
                candidates.extend(
                    expected_executable_names(launcher_app)
                        .into_iter()
                        .map(|name| parent.join(name)),
                );
                if let Ok(Some(best)) =
                    find_best_executable(parent, &launcher_app.id, &launcher_app.repo)
                {
                    candidates.push(best);
                }
            }
        }
    }

    candidates
}

#[cfg(windows)]
fn registry_display_name_matches_app(display_name: &str, launcher_app: &LauncherApp) -> bool {
    let display_name = normalized_install_name(display_name);
    app_install_name_aliases(launcher_app)
        .into_iter()
        .any(|alias| normalized_install_name(&alias) == display_name)
}

#[cfg(windows)]
fn app_install_name_aliases(launcher_app: &LauncherApp) -> Vec<String> {
    let mut aliases = vec![
        launcher_app.name.clone(),
        launcher_app.repo.clone(),
        launcher_app.id.clone(),
    ];
    if launcher_app.id == VENICE_MEDIA_LOCAL_ID {
        aliases.push(VENICE_MEDIA_LOCAL_DISPLAY_NAME.to_string());
    }
    aliases
}

#[cfg(windows)]
fn expected_executable_names(launcher_app: &LauncherApp) -> Vec<String> {
    let mut names = vec![
        format!("{}.exe", launcher_app.id.to_ascii_lowercase()),
        format!("{}.exe", launcher_app.repo.to_ascii_lowercase()),
        format!("{}.exe", normalized_install_name(&launcher_app.repo)),
        format!("{}.exe", normalized_install_name(&launcher_app.name)),
    ];
    names.sort();
    names.dedup();
    names
}

fn is_launchable_executable_path(path: &Path) -> bool {
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        return false;
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    !(name.contains("setup")
        || name.contains("installer")
        || name.contains("uninstall")
        || name.contains("update"))
}

#[cfg(windows)]
fn normalized_install_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

#[cfg(windows)]
fn command_path_from_registry_value(value: &str) -> Option<PathBuf> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let command = if let Some(rest) = trimmed.strip_prefix('"') {
        rest.split_once('"')
            .map(|(path, _)| path)
            .unwrap_or(rest)
            .trim()
            .to_string()
    } else {
        trimmed
            .split_once(".exe")
            .map(|(path, _)| format!("{path}.exe"))
            .unwrap_or_else(|| trimmed.split(',').next().unwrap_or(trimmed).to_string())
            .trim()
            .to_string()
    };

    let command = command.trim_matches('"');
    if command.is_empty() {
        None
    } else {
        Some(PathBuf::from(command))
    }
}

fn default_install_dir() -> Result<PathBuf, String> {
    let exe_path = std::env::current_exe().map_err(|err| err.to_string())?;
    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| "Could not locate launcher folder".to_string())?;
    let apps_dir = exe_dir.join("apps");
    fs::create_dir_all(&apps_dir).map_err(|err| err.to_string())?;
    Ok(apps_dir)
}

fn safe_file_segment(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect();
    let trimmed = sanitized.trim_matches([' ', '.']);
    if trimmed.is_empty() {
        "download".to_string()
    } else {
        trimmed.to_string()
    }
}

fn stable_portable_exe_path(launcher_app: &LauncherApp, target_dir: &Path) -> PathBuf {
    target_dir.join(format!("{}.exe", safe_file_segment(&launcher_app.repo)))
}

fn move_portable_exe_to_stable_name(source_path: &Path, stable_path: &Path) -> Result<PathBuf, String> {
    if source_path == stable_path {
        return Ok(source_path.to_path_buf());
    }
    if stable_path.exists() {
        fs::remove_file(stable_path).map_err(|err| err.to_string())?;
    }
    match fs::rename(source_path, stable_path) {
        Ok(()) => Ok(stable_path.to_path_buf()),
        Err(_) => {
            fs::copy(source_path, stable_path).map_err(|err| err.to_string())?;
            fs::remove_file(source_path).map_err(|err| err.to_string())?;
            Ok(stable_path.to_path_buf())
        }
    }
}

fn prune_installed_versions(
    launcher_app: &mut LauncherApp,
    previous_version: Option<String>,
    new_version: &str,
) -> Result<(), String> {
    let mut keep_versions = vec![new_version.to_string()];
    if let Some(previous_version) = previous_version {
        if !previous_version.trim().is_empty() && previous_version != new_version {
            keep_versions.push(previous_version);
        }
    }

    launcher_app
        .installed_versions
        .retain(|installed| keep_versions.iter().any(|version| version == &installed.version));
    launcher_app
        .installed_versions
        .truncate(MAX_INSTALLED_APP_VERSIONS);

    let root = default_install_dir()?.join(&launcher_app.id);
    if root.exists() {
        let keep_dirs: Vec<String> = keep_versions
            .iter()
            .map(|version| safe_file_segment(version))
            .collect();
        for entry in fs::read_dir(&root).map_err(|err| err.to_string())? {
            let entry = entry.map_err(|err| err.to_string())?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let folder_name = entry.file_name().to_string_lossy().to_string();
            if !keep_dirs.iter().any(|keep| keep == &folder_name) {
                fs::remove_dir_all(&path).map_err(|err| err.to_string())?;
            }
        }
    }

    if let Some(selected_version) = launcher_app.selected_version.clone() {
        if !launcher_app
            .installed_versions
            .iter()
            .any(|installed| installed.version == selected_version)
        {
            launcher_app.selected_version = None;
        }
    }

    Ok(())
}

fn is_installer_asset_name(name: &str) -> bool {
    name.ends_with(".msi") || name.contains("setup") || name.contains("installer")
}

fn is_portable_asset_name(name: &str) -> bool {
    name.ends_with(".zip")
        || name.contains("portable")
        || name.contains("standalone")
        || (name.ends_with(".exe") && !is_installer_asset_name(name))
}

fn release_asset_score(asset: &GitHubReleaseAsset, package_preference: &PackagePreference) -> i32 {
    let name = asset.name.to_ascii_lowercase();
    if name.ends_with(".blockmap")
        || name.ends_with(".sig")
        || name.ends_with(".sha256")
        || name.ends_with(".sha512")
        || name.ends_with(".dmg")
        || name.ends_with(".appimage")
        || name.ends_with(".deb")
        || name.ends_with(".rpm")
        || name.contains("linux")
        || name.contains("mac")
        || name.contains("darwin")
    {
        return -100;
    }

    let mut score = 0;

    match package_preference {
        PackagePreference::Portable => {
            if !is_portable_asset_name(&name) {
                return -100;
            }
            if name.ends_with(".exe") {
                score += 80;
            }
            if name.ends_with(".zip") {
                score += 65;
            }
            if name.contains("portable") {
                score += 40;
            }
            if name.contains("standalone") {
                score += 35;
            }
            if is_installer_asset_name(&name) {
                score -= 120;
            }
        }
        PackagePreference::Installer => {
            if !is_installer_asset_name(&name) {
                return -100;
            }
            if name.ends_with(".msi") {
                score += 80;
            }
            if name.contains("setup") || name.contains("installer") {
                score += 70;
            }
            if name.ends_with(".exe") {
                score += 45;
            }
        }
    }

    if name.contains("win") || name.contains("windows") {
        score += 25;
    }
    if name.contains("x64") || name.contains("amd64") {
        score += 15;
    }
    score
}

fn best_release_asset<'a>(
    release: &'a GitHubRelease,
    package_preference: &PackagePreference,
) -> Option<&'a GitHubReleaseAsset> {
    release
        .assets
        .iter()
        .filter(|asset| !asset.browser_download_url.trim().is_empty())
        .max_by_key(|asset| release_asset_score(asset, package_preference))
        .filter(|asset| release_asset_score(asset, package_preference) > 0)
}

fn control_center_asset_score(asset: &GitHubReleaseAsset) -> i32 {
    let name = asset.name.to_ascii_lowercase();
    if name.contains("updater") || name.contains("uninstall") {
        return -100;
    }
    let base_score = release_asset_score(asset, &PackagePreference::Portable);
    if base_score <= 0 {
        return base_score;
    }

    let mut score = base_score;
    if name == "neko-legends-control-center-portable.exe" {
        score += 140;
    }
    if name.contains("control") || name.contains("nekolegends") || name.contains("neko-legends") {
        score += 50;
    }
    if name.contains("portable") {
        score += 50;
    }
    if name.ends_with(".exe") {
        score += 25;
    }
    if is_installer_asset_name(&name) {
        score -= 200;
    }
    score
}

fn best_control_center_asset(release: &GitHubRelease) -> Option<&GitHubReleaseAsset> {
    release
        .assets
        .iter()
        .filter(|asset| !asset.browser_download_url.trim().is_empty())
        .max_by_key(|asset| control_center_asset_score(asset))
        .filter(|asset| control_center_asset_score(asset) > 0)
}

fn github_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("NekoLegendsControlCenter/26.6.22")
        .build()
        .map_err(|err| err.to_string())
}

fn apply_release_metadata(
    launcher_app: &mut LauncherApp,
    release: &GitHubRelease,
    checked_at: &str,
) {
    launcher_app.latest_version = Some(release.tag_name.clone());
    launcher_app.release_url = Some(release.html_url.clone());
    launcher_app.release_notes = release
        .body
        .as_ref()
        .map(|body| body.chars().take(240).collect());
    launcher_app.release_checked_at = Some(checked_at.to_string());
}

fn release_options(releases: &[GitHubRelease]) -> Vec<ReleaseOption> {
    releases
        .iter()
        .map(|release| ReleaseOption {
            tag_name: release.tag_name.clone(),
            html_url: release.html_url.clone(),
        })
        .collect()
}

fn version_parts(version: &str) -> Vec<u32> {
    version
        .trim()
        .trim_start_matches('v')
        .split(|character: char| !character.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<u32>().unwrap_or(0))
        .collect()
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    let latest_parts = version_parts(latest);
    let current_parts = version_parts(current);
    let max_len = latest_parts.len().max(current_parts.len()).max(1);

    for index in 0..max_len {
        let latest_part = *latest_parts.get(index).unwrap_or(&0);
        let current_part = *current_parts.get(index).unwrap_or(&0);
        if latest_part > current_part {
            return true;
        }
        if latest_part < current_part {
            return false;
        }
    }

    false
}

async fn fetch_releases(
    client: &reqwest::Client,
    repo: &str,
) -> Result<Vec<GitHubRelease>, String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases?per_page=20",
        GITHUB_OWNER, repo
    );
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| err.to_string())?;
    if response.status().as_u16() == 404 {
        return Err("No public releases found yet.".to_string());
    }
    if !response.status().is_success() {
        return Err(format!("GitHub returned {}.", response.status()));
    }

    let releases = response
        .json::<Vec<GitHubRelease>>()
        .await
        .map_err(|err| err.to_string())?;
    if releases.is_empty() {
        Err("No public releases found yet.".to_string())
    } else {
        Ok(releases)
    }
}

fn executable_score(path: &Path, app_id: &str, repo: &str) -> i32 {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let app_id = app_id.to_ascii_lowercase();
    let repo = repo.to_ascii_lowercase();
    let mut score = 10;

    if name.contains(&app_id) || name.contains(&repo) {
        score += 40;
    }
    if name.contains("portable") {
        score += 20;
    }
    if name.contains("setup")
        || name.contains("installer")
        || name.contains("uninstall")
        || name.contains("update")
    {
        score -= 35;
    }
    score
}

fn find_best_executable(root: &Path, app_id: &str, repo: &str) -> Result<Option<PathBuf>, String> {
    let mut stack = vec![root.to_path_buf()];
    let mut best: Option<(i32, PathBuf)> = None;

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).map_err(|err| err.to_string())? {
            let entry = entry.map_err(|err| err.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
            {
                let score = executable_score(&path, app_id, repo);
                let should_replace = match best.as_ref() {
                    Some((best_score, _)) => score > *best_score,
                    None => true,
                };
                if should_replace {
                    best = Some((score, path));
                }
            }
        }
    }

    Ok(best.map(|(_, path)| path))
}

fn control_center_executable_score(path: &Path) -> i32 {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let compact_name: String = name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect();
    let mut score = 10;

    if name == "neko-legends-control-center-portable.exe" {
        score += 140;
    }
    if compact_name.contains("nekolegendscontrolcenter") {
        score += 80;
    }
    if name.contains("control") {
        score += 40;
    }
    if name.contains("portable") || name.contains("standalone") {
        score += 35;
    }
    score
}

fn find_control_center_update_executable(root: &Path) -> Result<Option<PathBuf>, String> {
    let mut stack = vec![root.to_path_buf()];
    let mut best: Option<(i32, PathBuf)> = None;

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).map_err(|err| err.to_string())? {
            let entry = entry.map_err(|err| err.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if is_launchable_executable_path(&path) {
                let score = control_center_executable_score(&path);
                let should_replace = match best.as_ref() {
                    Some((best_score, _)) => score > *best_score,
                    None => true,
                };
                if should_replace {
                    best = Some((score, path));
                }
            }
        }
    }

    Ok(best.filter(|(score, _)| *score > 20).map(|(_, path)| path))
}

fn is_web_launch_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm")
        })
}

fn is_launch_path(path: &Path) -> bool {
    is_launchable_executable_path(path) || is_web_launch_path(path)
}

fn web_launch_score(path: &Path, app_id: &str, repo: &str) -> i32 {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let app_id = app_id.to_ascii_lowercase();
    let repo = repo.to_ascii_lowercase();
    let mut score = 10;

    if name == "wallpaper.html" {
        score += 90;
    }
    if name == "index.html" {
        score += 70;
    }
    if name.contains(&app_id) || name.contains(&repo) {
        score += 30;
    }
    if name.contains("readme") || name.contains("license") {
        score -= 100;
    }
    score
}

fn find_lively_launch_path(root: &Path) -> Result<Option<PathBuf>, String> {
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).map_err(|err| err.to_string())? {
            let entry = entry.map_err(|err| err.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let is_lively_info = path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("LivelyInfo.json"));
            if !is_lively_info {
                continue;
            }

            let raw = fs::read_to_string(&path).map_err(|err| err.to_string())?;
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
                continue;
            };
            let Some(file_name) = value
                .get("FileName")
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
            else {
                continue;
            };
            let Some(parent) = path.parent() else {
                continue;
            };
            let candidate = parent.join(file_name);
            if candidate.exists() && is_launch_path(&candidate) {
                return Ok(Some(candidate));
            }
        }
    }

    Ok(None)
}

fn find_best_web_launch_path(
    root: &Path,
    app_id: &str,
    repo: &str,
) -> Result<Option<PathBuf>, String> {
    let mut stack = vec![root.to_path_buf()];
    let mut best: Option<(i32, PathBuf)> = None;

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).map_err(|err| err.to_string())? {
            let entry = entry.map_err(|err| err.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if is_web_launch_path(&path) {
                let score = web_launch_score(&path, app_id, repo);
                let should_replace = match best.as_ref() {
                    Some((best_score, _)) => score > *best_score,
                    None => true,
                };
                if should_replace {
                    best = Some((score, path));
                }
            }
        }
    }

    Ok(best.map(|(_, path)| path))
}

fn find_best_launch_path(root: &Path, app_id: &str, repo: &str) -> Result<Option<PathBuf>, String> {
    if let Some(executable_path) = find_best_executable(root, app_id, repo)? {
        return Ok(Some(executable_path));
    }
    if let Some(lively_path) = find_lively_launch_path(root)? {
        return Ok(Some(lively_path));
    }
    find_best_web_launch_path(root, app_id, repo)
}

fn package_archive_score(path: &Path, app_id: &str, repo: &str) -> i32 {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let app_id = app_id.to_ascii_lowercase();
    let repo = repo.to_ascii_lowercase();
    let mut score = 10;

    if name.contains(&app_id) || name.contains(&repo) {
        score += 50;
    }
    if name.contains("source") || name.contains("src") {
        score -= 40;
    }
    score
}

fn find_best_package_archive(
    root: &Path,
    app_id: &str,
    repo: &str,
) -> Result<Option<PathBuf>, String> {
    let mut stack = vec![root.to_path_buf()];
    let mut best: Option<(i32, PathBuf)> = None;

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).map_err(|err| err.to_string())? {
            let entry = entry.map_err(|err| err.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
            {
                let score = package_archive_score(&path, app_id, repo);
                let should_replace = match best.as_ref() {
                    Some((best_score, _)) => score > *best_score,
                    None => true,
                };
                if should_replace {
                    best = Some((score, path));
                }
            }
        }
    }

    Ok(best.map(|(_, path)| path))
}

fn cleanup_redundant_archives_near_launch_path(launch_path: &Path) {
    let Some(folder) = launch_path.parent() else {
        return;
    };
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
        {
            let _ = fs::remove_file(path);
        }
    }
}

fn cleanup_wallpaper_package_dir(target_dir: &Path, package_path: &Path) -> Result<(), String> {
    let package_path = package_path
        .canonicalize()
        .unwrap_or_else(|_| package_path.to_path_buf());
    for entry in fs::read_dir(target_dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        let comparable_path = path.canonicalize().unwrap_or_else(|_| path.clone());
        if comparable_path == package_path {
            continue;
        }
        if path.is_dir() {
            fs::remove_dir_all(&path).map_err(|err| err.to_string())?;
        } else {
            fs::remove_file(&path).map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}

fn extract_zip(zip_path: &Path, target_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|err| err.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|err| err.to_string())?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|err| err.to_string())?;
        let Some(enclosed_name) = entry.enclosed_name().map(|path| path.to_path_buf()) else {
            continue;
        };
        let output_path = target_dir.join(enclosed_name);

        if entry.is_dir() {
            fs::create_dir_all(&output_path).map_err(|err| err.to_string())?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }

        let mut output = fs::File::create(&output_path).map_err(|err| err.to_string())?;
        io::copy(&mut entry, &mut output).map_err(|err| err.to_string())?;
    }

    Ok(())
}

fn clamp_window_dimension(value: u32, min: u32, monitor_max: u32) -> u32 {
    let max = monitor_max.max(1);
    value.max(min.min(max)).min(max)
}

fn default_window_size_for_monitor(monitor_width: u32, monitor_height: u32) -> PhysicalSize<u32> {
    if monitor_width < monitor_height {
        let width = ((monitor_width as f32) * PORTRAIT_WINDOW_WIDTH_RATIO).round() as u32;
        let height = ((width as f32)
            * (FALLBACK_WINDOW_HEIGHT as f32 / FALLBACK_WINDOW_WIDTH as f32))
            .round() as u32;
        return PhysicalSize::new(width.max(1), height.max(1));
    }

    PhysicalSize::new(
        monitor_width.saturating_div(4).max(FALLBACK_WINDOW_WIDTH),
        monitor_height.saturating_div(4).max(FALLBACK_WINDOW_HEIGHT),
    )
}

fn preferred_window_size(
    settings: &AppSettings,
    monitor_size: Option<PhysicalSize<u32>>,
) -> PhysicalSize<u32> {
    let monitor_width = monitor_size
        .as_ref()
        .map(|size| size.width)
        .unwrap_or(FALLBACK_WINDOW_WIDTH);
    let monitor_height = monitor_size
        .as_ref()
        .map(|size| size.height)
        .unwrap_or(FALLBACK_WINDOW_HEIGHT);
    let default_size = default_window_size_for_monitor(monitor_width, monitor_height);
    let width = settings.window_width.unwrap_or(default_size.width);
    let height = settings.window_height.unwrap_or(default_size.height);

    PhysicalSize::new(
        clamp_window_dimension(width, MIN_WINDOW_WIDTH, monitor_width),
        clamp_window_dimension(height, MIN_WINDOW_HEIGHT, monitor_height),
    )
}

fn apply_initial_window_size(app: &AppHandle, window: &WebviewWindow) -> Result<(), String> {
    let settings = read_settings(app);
    let monitor_size = window
        .current_monitor()
        .map_err(|err| err.to_string())?
        .or_else(|| window.primary_monitor().ok().flatten())
        .map(|monitor| *monitor.size());
    let size = preferred_window_size(&settings, monitor_size);

    window
        .set_size(Size::Physical(size))
        .map_err(|err| err.to_string())?;
    let _ = window.center();
    Ok(())
}

fn persist_window_size(app: &AppHandle, size: PhysicalSize<u32>) -> Result<(), String> {
    if size.width == 0 || size.height == 0 {
        return Ok(());
    }
    let mut settings = read_settings(app);
    settings.window_width = Some(size.width);
    settings.window_height = Some(size.height);
    write_json_file(&settings_path(app)?, &settings)
}

#[tauri::command]
fn get_state(app: AppHandle) -> Result<ControlCenterState, String> {
    let data_dir = app_data_dir(&app)?.to_string_lossy().to_string();
    let apps = read_apps(&app);
    save_apps(&app, &apps)?;

    Ok(ControlCenterState {
        settings: read_settings(&app),
        apps,
        build_version: app.package_info().version.to_string(),
        data_dir,
    })
}

#[tauri::command]
async fn refresh_tools_catalog(app: AppHandle) -> Result<Vec<LauncherApp>, String> {
    if !read_settings(&app).use_remote_catalog {
        let apps = read_apps(&app);
        save_apps(&app, &apps)?;
        return Ok(apps);
    }

    let client = github_client()?;
    let response = client
        .get(TOOLS_CATALOG_URL)
        .send()
        .await
        .map_err(|err| err.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Tools catalog returned {}.", response.status()));
    }

    let bytes = response.bytes().await.map_err(|err| err.to_string())?;
    let catalog = serde_json::from_slice::<ToolsCatalog>(&bytes).map_err(|err| err.to_string())?;
    let mut catalog = clean_tools_catalog(catalog)?;
    if catalog.updated_at.is_none() {
        catalog.updated_at = Some(Utc::now().to_rfc3339());
    }
    write_json_file(&tools_catalog_path(&app)?, &catalog)?;
    let apps = read_apps(&app);
    save_apps(&app, &apps)?;
    Ok(apps)
}

#[tauri::command]
fn save_settings(app: AppHandle, request: SaveSettingsRequest) -> Result<AppSettings, String> {
    let mut settings = read_settings(&app);
    if let Some(theme) = request.theme {
        settings.theme = theme;
    }
    if let Some(compact_labels) = request.compact_labels {
        settings.compact_labels = compact_labels;
    }
    if let Some(use_remote_catalog) = request.use_remote_catalog {
        settings.use_remote_catalog = use_remote_catalog;
    }
    if let Some(agent_control_auto_start) = request.agent_control_auto_start {
        settings.agent_control_auto_start = agent_control_auto_start;
    }
    if let Some(categories) = request.categories {
        settings.categories = normalize_categories(categories);
    }
    write_json_file(&settings_path(&app)?, &settings)?;
    Ok(settings)
}

#[tauri::command]
fn save_executable(
    app: AppHandle,
    request: SaveExecutableRequest,
) -> Result<Vec<LauncherApp>, String> {
    let mut apps = read_apps(&app);
    let target = apps
        .iter_mut()
        .find(|candidate| candidate.id == request.app_id)
        .ok_or_else(|| "App was not found".to_string())?;
    let path = PathBuf::from(request.executable_path.trim());
    if !path.exists() {
        return Err("Launch path does not exist".to_string());
    }
    if !is_launch_path(&path) {
        return Err("Choose an .exe, .html, or .htm launch file.".to_string());
    }
    target.executable_path = Some(path.to_string_lossy().to_string());
    save_apps(&app, &apps)?;
    Ok(apps)
}

#[tauri::command]
fn get_default_install_dir() -> Result<String, String> {
    default_install_dir().map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
fn save_layout(app: AppHandle, request: SaveLayoutRequest) -> Result<ControlCenterState, String> {
    let mut next_apps = merge_catalog_apps(request.apps, read_effective_tools_catalog(&app).tools);
    let mut settings = read_settings(&app);
    if next_apps.iter().all(|candidate| !candidate.visible) {
        for candidate in next_apps.iter_mut() {
            candidate.visible = true;
        }
    }
    if let Some(categories) = request.categories {
        settings.categories = normalize_categories(categories);
        write_json_file(&settings_path(&app)?, &settings)?;
    }
    save_apps(&app, &next_apps)?;
    get_state(app)
}

#[tauri::command]
fn reset_layout(app: AppHandle) -> Result<ControlCenterState, String> {
    let current_apps = read_apps(&app);
    let mut settings = read_settings(&app);
    let mut reset_apps = Vec::new();

    for mut default_app in read_effective_tools_catalog(&app).tools {
        if let Some(existing) = current_apps
            .iter()
            .find(|candidate| candidate.id == default_app.id)
        {
            default_app.executable_path = existing.executable_path.clone();
            default_app.installed_version = existing.installed_version.clone();
            default_app.selected_version = existing.selected_version.clone();
            default_app.installed_versions = existing.installed_versions.clone();
            default_app.latest_version = existing.latest_version.clone();
            default_app.release_url = existing.release_url.clone();
            default_app.release_checked_at = existing.release_checked_at.clone();
            default_app.release_notes = existing.release_notes.clone();
            default_app.release_options = existing.release_options.clone();
            default_app.package_preference = existing.package_preference.clone();
            default_app.package_path = existing.package_path.clone();
        }
        default_app.visible = true;
        reset_apps.push(default_app);
    }

    save_apps(&app, &reset_apps)?;
    settings.categories = default_categories();
    write_json_file(&settings_path(&app)?, &settings)?;
    get_state(app)
}

fn running_apps() -> &'static Mutex<BTreeMap<String, Child>> {
    RUNNING_APPS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn launch_or_relaunch_executable(app_id: &str, path: &Path) -> Result<bool, String> {
    let mut running = running_apps()
        .lock()
        .map_err(|_| "Could not access running app state.".to_string())?;
    let mut relaunched = false;

    if let Some(child) = running.get_mut(app_id) {
        match child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                relaunched = true;
                let _ = child.kill();
                let _ = child.wait();
            }
            Err(_) => {}
        }
        running.remove(app_id);
    }

    let child = Command::new(path).spawn().map_err(|err| err.to_string())?;
    running.insert(app_id.to_string(), child);
    Ok(relaunched)
}

#[tauri::command]
fn launch_app(app: AppHandle, request: LaunchRequest) -> Result<LaunchResult, String> {
    let mut apps = read_apps(&app);
    let index = apps
        .iter()
        .position(|candidate| candidate.id == request.app_id)
        .ok_or_else(|| "App was not found".to_string())?;
    if let Some(version) = request.version.as_deref().filter(|version| !version.trim().is_empty())
    {
        select_installed_version(&mut apps[index], version)?;
    }
    let launcher_app = &apps[index];
    let Some(executable_path) = launcher_app.executable_path.as_deref() else {
        return Err("No launch file has been configured for this app".to_string());
    };
    let path = PathBuf::from(executable_path);
    if !path.exists() {
        return Err("Configured launch file no longer exists".to_string());
    }
    let relaunched = if is_launchable_executable_path(&path) {
        launch_or_relaunch_executable(&request.app_id, &path)?
    } else if is_web_launch_path(&path) {
        open::that(path).map_err(|err| err.to_string())?;
        false
    } else {
        return Err("Configured launch file is not supported".to_string());
    };
    save_apps(&app, &apps)?;
    Ok(LaunchResult { apps, relaunched })
}

fn headless_agent_args(app_id: &str, port: u16) -> Result<Vec<String>, String> {
    match app_id {
        "sprite-atlas-packer" => Ok(vec![
            "--headless".to_string(),
            "--serve".to_string(),
            "--port".to_string(),
            port.to_string(),
        ]),
        "image-to-splat" => Ok(vec![
            "--serve-agent-api".to_string(),
            "--agent-api-port".to_string(),
            port.to_string(),
        ]),
        "asset-vault"
        | "batchlapse"
        | "cutscene-converter"
        | "depth-map-ai-generator"
        | "image-to-3d"
        | "multi-angle-edit" => Ok(vec![
            "--headless".to_string(),
            "--agent-api-port".to_string(),
            port.to_string(),
        ]),
        _ => Err(
            "This app does not advertise a Control Center headless launch mode yet.".to_string(),
        ),
    }
}

#[tauri::command]
fn launch_agent_api_headless(app: AppHandle, request: LaunchRequest) -> Result<(), String> {
    let apps = read_apps(&app);
    let launcher_app = apps
        .iter()
        .find(|candidate| candidate.id == request.app_id)
        .ok_or_else(|| "App was not found".to_string())?;
    let Some(executable_path) = launcher_app.executable_path.as_deref() else {
        return Err("No launch file has been configured for this app".to_string());
    };
    let path = PathBuf::from(executable_path);
    if !path.exists() {
        return Err("Configured launch file no longer exists".to_string());
    }
    if !is_launchable_executable_path(&path) {
        return Err("Headless Agent API launch requires a Windows executable.".to_string());
    }

    let registry = read_agent_api_registry()?;
    let entries = merged_agent_api_entries(&registry);
    let entry = entries
        .iter()
        .find(|entry| entry.app_id == request.app_id)
        .ok_or_else(|| "This app is not in the Agent API registry.".to_string())?;
    let args = headless_agent_args(&request.app_id, entry.port)?;
    Command::new(path)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn open_release_url(app: AppHandle, request: LaunchRequest) -> Result<(), String> {
    let apps = read_apps(&app);
    let launcher_app = apps
        .iter()
        .find(|candidate| candidate.id == request.app_id)
        .ok_or_else(|| "App was not found".to_string())?;
    let url = launcher_app
        .release_url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            format!(
                "https://github.com/{}/{}/releases",
                GITHUB_OWNER, launcher_app.repo
            )
        });
    open::that(url).map_err(|err| err.to_string())
}

#[tauri::command]
fn open_repository_url(app: AppHandle, request: LaunchRequest) -> Result<(), String> {
    let apps = read_apps(&app);
    let launcher_app = apps
        .iter()
        .find(|candidate| candidate.id == request.app_id)
        .ok_or_else(|| "App was not found".to_string())?;
    let url = format!("https://github.com/{}/{}", GITHUB_OWNER, launcher_app.repo);
    open::that(url).map_err(|err| err.to_string())
}

#[tauri::command]
fn open_demo_url(app: AppHandle, request: LaunchRequest) -> Result<(), String> {
    let apps = read_apps(&app);
    let launcher_app = apps
        .iter()
        .find(|candidate| candidate.id == request.app_id)
        .ok_or_else(|| "App was not found".to_string())?;
    let url = launcher_app
        .demo_url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "This app does not have a hosted demo.".to_string())?;
    open::that(url).map_err(|err| err.to_string())
}

#[tauri::command]
async fn check_control_center_update(app: AppHandle) -> Result<ControlCenterUpdate, String> {
    let client = github_client()?;
    let current_version = app.package_info().version.to_string();
    let checked_at = Utc::now().to_rfc3339();

    match fetch_releases(&client, CONTROL_CENTER_REPO).await {
        Ok(releases) => {
            let latest = releases.first();
            let latest_version = latest.map(|release| release.tag_name.clone());
            let release_url = latest.map(|release| release.html_url.clone());
            let release_notes = latest.and_then(|release| {
                release
                    .body
                    .as_ref()
                    .map(|body| body.chars().take(240).collect())
            });
            let update_available = latest_version
                .as_deref()
                .is_some_and(|latest| is_newer_version(latest, &current_version));

            Ok(ControlCenterUpdate {
                current_version,
                latest_version,
                release_url,
                release_notes,
                checked_at,
                update_available,
            })
        }
        Err(error) => Err(error),
    }
}

#[tauri::command]
fn open_control_center_release(update: ControlCenterUpdate) -> Result<(), String> {
    let url = update
        .release_url
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            format!(
                "https://github.com/{}/{}/releases",
                GITHUB_OWNER, CONTROL_CENTER_REPO
            )
        });
    open::that(url).map_err(|err| err.to_string())
}

fn ensure_update_target_writable(target_exe: &Path) -> Result<PathBuf, String> {
    let target_dir = target_exe
        .parent()
        .ok_or_else(|| "Could not locate the Control Center folder.".to_string())?
        .to_path_buf();
    let probe_path = target_dir.join(format!(".nlcc-update-check-{}", std::process::id()));
    fs::write(&probe_path, b"ok").map_err(|_| {
        "This Control Center folder is protected. Download the latest portable build from the release page instead.".to_string()
    })?;
    let _ = fs::remove_file(probe_path);
    Ok(target_dir)
}

fn self_update_script() -> &'static str {
    r#"
param(
    [int]$ParentPid,
    [string]$SourceExe,
    [string]$TargetExe,
    [string]$RelaunchDir,
    [string]$LogPath
)

$ErrorActionPreference = 'Stop'

function Write-UpdateLog {
    param([string]$Message)
    try {
        $folder = Split-Path -Parent $LogPath
        if ($folder -and -not (Test-Path -LiteralPath $folder)) {
            New-Item -ItemType Directory -Path $folder -Force | Out-Null
        }
        Add-Content -LiteralPath $LogPath -Value ("{0} {1}" -f (Get-Date).ToString("s"), $Message)
    } catch {
    }
}

try {
    Write-UpdateLog "Waiting for Control Center to close."
    $deadline = (Get-Date).AddSeconds(60)
    while ((Get-Date) -lt $deadline) {
        $running = Get-Process -Id $ParentPid -ErrorAction SilentlyContinue
        if ($null -eq $running) {
            break
        }
        Start-Sleep -Milliseconds 250
    }

    if (-not (Test-Path -LiteralPath $SourceExe)) {
        throw "Downloaded update file is missing."
    }
    if (-not (Test-Path -LiteralPath $RelaunchDir)) {
        New-Item -ItemType Directory -Path $RelaunchDir -Force | Out-Null
    }

    $backup = Join-Path $RelaunchDir ((Split-Path -Leaf $TargetExe) + ".previous")
    Remove-Item -LiteralPath $backup -Force -ErrorAction SilentlyContinue

    $copied = $false
    for ($attempt = 0; $attempt -lt 120; $attempt++) {
        try {
            if (Test-Path -LiteralPath $TargetExe) {
                Copy-Item -LiteralPath $TargetExe -Destination $backup -Force -ErrorAction SilentlyContinue
            }
            Copy-Item -LiteralPath $SourceExe -Destination $TargetExe -Force
            $copied = $true
            break
        } catch {
            Start-Sleep -Milliseconds 500
        }
    }

    if (-not $copied) {
        if ((-not (Test-Path -LiteralPath $TargetExe)) -and (Test-Path -LiteralPath $backup)) {
            Move-Item -LiteralPath $backup -Destination $TargetExe -Force
        }
        if (Test-Path -LiteralPath $TargetExe) {
            Start-Process -FilePath $TargetExe -WorkingDirectory $RelaunchDir
        }
        throw "Could not replace the Control Center executable."
    }

    Write-UpdateLog "Launching updated Control Center."
    Start-Process -FilePath $TargetExe -WorkingDirectory $RelaunchDir
} catch {
    Write-UpdateLog ("Update failed: " + $_.Exception.Message)
    if (Test-Path -LiteralPath $TargetExe) {
        Start-Process -FilePath $TargetExe -WorkingDirectory $RelaunchDir
    }
}
"#
}

fn schedule_control_center_update(app: &AppHandle, source_exe: &Path) -> Result<(), String> {
    if !cfg!(windows) {
        return Err(
            "Automatic Control Center updates are currently supported on Windows portable builds."
                .to_string(),
        );
    }

    if !source_exe.exists() {
        return Err("Downloaded Control Center update file is missing.".to_string());
    }

    let target_exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let target_dir = ensure_update_target_writable(&target_exe)?;
    let updater_dir = app_data_dir(app)?.join("self-updater");
    fs::create_dir_all(&updater_dir).map_err(|err| err.to_string())?;
    let script_path = updater_dir.join("apply-control-center-update.ps1");
    let log_path = updater_dir.join("self-update.log");
    fs::write(&script_path, self_update_script()).map_err(|err| err.to_string())?;

    Command::new("powershell")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-WindowStyle")
        .arg("Hidden")
        .arg("-File")
        .arg(&script_path)
        .arg("-ParentPid")
        .arg(std::process::id().to_string())
        .arg("-SourceExe")
        .arg(source_exe)
        .arg("-TargetExe")
        .arg(&target_exe)
        .arg("-RelaunchDir")
        .arg(&target_dir)
        .arg("-LogPath")
        .arg(&log_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| err.to_string())?;

    app.exit(0);
    Ok(())
}

#[tauri::command]
async fn install_control_center_update(app: AppHandle) -> Result<(), String> {
    if !cfg!(windows) {
        return Err(
            "Automatic Control Center updates are currently supported on Windows portable builds."
                .to_string(),
        );
    }

    let client = github_client()?;
    let current_version = app.package_info().version.to_string();
    let releases = fetch_releases(&client, CONTROL_CENTER_REPO).await?;
    let release = releases
        .first()
        .ok_or_else(|| "No public Control Center releases found yet.".to_string())?;
    if !is_newer_version(&release.tag_name, &current_version) {
        return Err("Control Center is already up to date.".to_string());
    }
    let asset = best_control_center_asset(release).ok_or_else(|| {
        "The latest Control Center release does not have a portable Windows download yet."
            .to_string()
    })?;
    let staging_dir = app_data_dir(&app)?
        .join("self-updates")
        .join(safe_file_segment(&release.tag_name));
    if staging_dir.exists() {
        fs::remove_dir_all(&staging_dir).map_err(|err| err.to_string())?;
    }
    fs::create_dir_all(&staging_dir).map_err(|err| err.to_string())?;

    let asset_path = staging_dir.join(safe_file_segment(&asset.name));
    let response = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .map_err(|err| err.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Control Center update download failed with {}.",
            response.status()
        ));
    }
    let bytes = response.bytes().await.map_err(|err| err.to_string())?;
    fs::write(&asset_path, &bytes).map_err(|err| err.to_string())?;

    let is_zip_asset = asset_path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"));
    let source_exe = if is_launchable_executable_path(&asset_path) {
        asset_path
    } else if is_zip_asset {
        extract_zip(&asset_path, &staging_dir)?;
        find_control_center_update_executable(&staging_dir)?.ok_or_else(|| {
            "The Control Center update package did not include a launchable Windows app."
                .to_string()
        })?
    } else {
        return Err(
            "The latest Control Center download is not a portable Windows app.".to_string(),
        );
    };

    schedule_control_center_update(&app, &source_exe)
}

#[tauri::command]
fn open_install_folder(app: AppHandle, request: LaunchRequest) -> Result<(), String> {
    let apps = read_apps(&app);
    let launcher_app = apps
        .iter()
        .find(|candidate| candidate.id == request.app_id)
        .ok_or_else(|| "App was not found".to_string())?;
    let folder = launcher_app
        .executable_path
        .as_deref()
        .or(launcher_app.package_path.as_deref())
        .map(PathBuf::from)
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or(default_install_dir()?.join(&launcher_app.id));

    fs::create_dir_all(&folder).map_err(|err| err.to_string())?;
    open::that(folder).map_err(|err| err.to_string())
}

fn agent_control_root(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("agent-control"))
}

fn agent_control_layout(
    app: &AppHandle,
) -> Result<(AgentControlInfo, PathBuf, PathBuf, PathBuf, PathBuf), String> {
    let root = agent_control_root(app)?;
    let inbox = root.join("inbox");
    let outbox = root.join("outbox");
    let history = root.join("history");
    let state_path = root.join("state.json");
    fs::create_dir_all(&inbox).map_err(|err| err.to_string())?;
    fs::create_dir_all(&outbox).map_err(|err| err.to_string())?;
    fs::create_dir_all(&history).map_err(|err| err.to_string())?;
    let info = AgentControlInfo {
        root_dir: root.to_string_lossy().to_string(),
        inbox_dir: inbox.to_string_lossy().to_string(),
        outbox_dir: outbox.to_string_lossy().to_string(),
        history_dir: history.to_string_lossy().to_string(),
        state_path: state_path.to_string_lossy().to_string(),
    };
    Ok((info, inbox, outbox, history, state_path))
}

fn write_agent_control_state(path: &Path, apps: &[LauncherApp]) -> Result<(), String> {
    let value = serde_json::json!({
        "updatedAt": Utc::now().to_rfc3339(),
        "apps": apps,
    });
    write_json_file(path, &value)
}

fn agent_response_file_name(command_path: &Path) -> String {
    let stem = command_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("command");
    format!("{stem}.result.json")
}

fn agent_history_file_name(command_path: &Path) -> String {
    let stem = command_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("command");
    format!("{}-{stem}.json", Utc::now().format("%Y%m%dT%H%M%S%.3fZ"))
}

fn require_agent_app_id(command: &AgentControlCommand) -> Result<String, String> {
    command
        .app_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "Command requires appId.".to_string())
}

async fn execute_agent_control_command(
    app: AppHandle,
    command: &AgentControlCommand,
) -> Result<serde_json::Value, String> {
    match command.action.clone() {
        AgentControlAction::Status => {
            let apps = read_apps(&app);
            Ok(serde_json::json!({ "apps": apps }))
        }
        AgentControlAction::Scan => {
            let apps = scan_releases(app).await?;
            Ok(serde_json::json!({ "apps": apps }))
        }
        AgentControlAction::Download | AgentControlAction::Update => {
            let app_id = require_agent_app_id(command)?;
            let result = download_release(
                app,
                DownloadRequest {
                    app_id,
                    version: command.version.clone(),
                    package_preference: command.package_preference.clone(),
                },
            )
            .await?;
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        AgentControlAction::Launch => {
            let app_id = require_agent_app_id(command)?;
            launch_app(
                app,
                LaunchRequest {
                    app_id,
                    version: command.version.clone(),
                },
            )?;
            Ok(serde_json::json!({ "launched": true }))
        }
        AgentControlAction::OpenFolder => {
            let app_id = require_agent_app_id(command)?;
            open_install_folder(
                app,
                LaunchRequest {
                    app_id,
                    version: command.version.clone(),
                },
            )?;
            Ok(serde_json::json!({ "opened": true }))
        }
    }
}

#[tauri::command]
fn get_agent_control_info(app: AppHandle) -> Result<AgentControlInfo, String> {
    let (info, _, _, _, state_path) = agent_control_layout(&app)?;
    let apps = read_apps(&app);
    write_agent_control_state(&state_path, &apps)?;
    Ok(info)
}

#[tauri::command]
fn get_agent_api_dashboard() -> Result<AgentApiDashboard, String> {
    agent_api_dashboard_from(read_agent_api_registry()?)
}

#[tauri::command]
fn save_agent_api_port(request: SaveAgentApiPortRequest) -> Result<AgentApiDashboard, String> {
    if request.port == 0 {
        return Err("Agent API port must be between 1 and 65535.".to_string());
    }

    let registry = read_agent_api_registry()?;
    let mut entries = merged_agent_api_entries(&registry);
    let entry = entries
        .iter_mut()
        .find(|entry| entry.app_id == request.app_id)
        .ok_or_else(|| "Agent API app was not found.".to_string())?;
    entry.port = request.port;
    entry.url = agent_api_url(&entry.bind_address, entry.port);
    entry.openapi_url = format!("{}/openapi.json", entry.url);
    entry.last_seen = Some(Utc::now().to_rfc3339());

    let registry = write_agent_api_registry(entries)?;
    agent_api_dashboard_from(registry)
}

#[tauri::command]
async fn process_agent_control_commands(app: AppHandle) -> Result<AgentControlPollResult, String> {
    let (info, inbox, outbox, history, state_path) = agent_control_layout(&app)?;
    let mut command_paths = Vec::new();

    for entry in fs::read_dir(&inbox).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        {
            command_paths.push(path);
        }
    }

    command_paths.sort();
    let mut processed_count = 0usize;

    for command_path in command_paths {
        let raw = fs::read_to_string(&command_path).map_err(|err| err.to_string())?;
        let parsed = serde_json::from_str::<AgentControlCommand>(&raw);
        let processed_at = Utc::now().to_rfc3339();
        let response = match parsed {
            Ok(command) => {
                let result = execute_agent_control_command(app.clone(), &command).await;
                match result {
                    Ok(data) => AgentControlResponse {
                        id: command.id,
                        action: Some(command.action),
                        app_id: command.app_id,
                        ok: true,
                        message: "OK".to_string(),
                        processed_at,
                        data,
                    },
                    Err(error) => AgentControlResponse {
                        id: command.id,
                        action: Some(command.action),
                        app_id: command.app_id,
                        ok: false,
                        message: error,
                        processed_at,
                        data: serde_json::Value::Null,
                    },
                }
            }
            Err(error) => AgentControlResponse {
                id: None,
                action: None,
                app_id: None,
                ok: false,
                message: format!("Invalid command JSON: {error}"),
                processed_at,
                data: serde_json::Value::Null,
            },
        };

        let response_path = outbox.join(agent_response_file_name(&command_path));
        write_json_file(&response_path, &response)?;
        let history_path = history.join(agent_history_file_name(&command_path));
        let _ = fs::rename(&command_path, &history_path).or_else(|_| {
            fs::copy(&command_path, &history_path)?;
            fs::remove_file(&command_path)
        });
        processed_count += 1;
    }

    let apps = read_apps(&app);
    write_agent_control_state(&state_path, &apps)?;
    Ok(AgentControlPollResult {
        processed_count,
        apps,
        info,
    })
}

#[tauri::command]
async fn scan_releases(app: AppHandle) -> Result<Vec<LauncherApp>, String> {
    let _ = refresh_tools_catalog(app.clone()).await;
    let client = github_client()?;
    let mut apps = read_apps(&app);
    let checked_at = Utc::now().to_rfc3339();

    for launcher_app in apps.iter_mut() {
        if is_coming_soon_app(launcher_app) {
            normalize_development_status(launcher_app);
            launcher_app.latest_version = None;
            launcher_app.release_url = Some(format!(
                "https://github.com/{}/{}/releases",
                GITHUB_OWNER, launcher_app.repo
            ));
            launcher_app.release_notes = Some("Coming soon.".to_string());
            launcher_app.release_checked_at = Some(checked_at.clone());
            launcher_app.release_options = Vec::new();
            continue;
        }
        match fetch_releases(&client, &launcher_app.repo).await {
            Ok(releases) => {
                if let Some(release) = releases.first() {
                    apply_release_metadata(launcher_app, release, &checked_at);
                }
                launcher_app.release_options = release_options(&releases);
            }
            Err(error) if error == "No public releases found yet." => {
                launcher_app.latest_version = None;
                launcher_app.release_url = Some(format!(
                    "https://github.com/{}/{}/releases",
                    GITHUB_OWNER, launcher_app.repo
                ));
                launcher_app.release_notes = Some("No public releases found yet.".to_string());
                launcher_app.release_checked_at = Some(checked_at.clone());
                launcher_app.release_options = Vec::new();
            }
            Err(error) => {
                launcher_app.release_notes = Some(error);
                launcher_app.release_checked_at = Some(checked_at.clone());
            }
        }
    }

    save_apps(&app, &apps)?;
    Ok(apps)
}

#[tauri::command]
async fn download_release(
    app: AppHandle,
    request: DownloadRequest,
) -> Result<DownloadResult, String> {
    let client = github_client()?;
    let mut apps = read_apps(&app);
    let index = apps
        .iter()
        .position(|candidate| candidate.id == request.app_id)
        .ok_or_else(|| "App was not found".to_string())?;
    if is_coming_soon_app(&apps[index]) {
        return Err(format!("{} is coming soon.", apps[index].name));
    }
    let repo = apps[index].repo.clone();
    let previous_installed_version = apps[index].installed_version.clone();
    let package_preference = request
        .package_preference
        .clone()
        .unwrap_or_else(|| apps[index].package_preference.clone());
    let releases = fetch_releases(&client, &repo).await?;
    let release = request
        .version
        .as_deref()
        .and_then(|version| {
            releases
                .iter()
                .find(|candidate| candidate.tag_name == version)
        })
        .or_else(|| releases.first())
        .ok_or_else(|| "No public releases found yet.".to_string())?;
    let asset =
        best_release_asset(release, &package_preference).ok_or_else(
            || match package_preference {
                PackagePreference::Portable => {
                    "Selected release does not have a portable Windows download asset.".to_string()
                }
                PackagePreference::Installer => {
                    "Selected release does not have a Windows installer asset.".to_string()
                }
            },
        )?;
    let file_name = safe_file_segment(&asset.name);
    let tag_name = release.tag_name.clone();
    let target_dir = default_install_dir()?
        .join(&apps[index].id)
        .join(safe_file_segment(&tag_name));
    fs::create_dir_all(&target_dir).map_err(|err| err.to_string())?;
    let mut target_path = target_dir.join(file_name);

    let response = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .map_err(|err| err.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Download failed with {}.", response.status()));
    }
    let bytes = response.bytes().await.map_err(|err| err.to_string())?;
    fs::write(&target_path, &bytes).map_err(|err| err.to_string())?;

    let is_zip_asset = target_path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"));
    let is_wallpaper_package = apps[index].demo_url.is_some() && is_zip_asset;
    let mut extracted_archive = false;
    if is_wallpaper_package {
        cleanup_wallpaper_package_dir(&target_dir, &target_path)?;
    }
    if is_zip_asset && !is_wallpaper_package {
        extract_zip(&target_path, &target_dir)?;
        extracted_archive = true;
    }

    let mut downloaded_name = target_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(package_preference, PackagePreference::Portable)
        && target_path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        && !is_installer_asset_name(&downloaded_name)
    {
        let stable_path = stable_portable_exe_path(&apps[index], &target_dir);
        target_path = move_portable_exe_to_stable_name(&target_path, &stable_path)?;
        downloaded_name = target_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
    }
    let executable_path = match package_preference {
        PackagePreference::Portable
            if target_path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
                && !is_installer_asset_name(&downloaded_name) =>
        {
            Some(target_path.clone())
        }
        PackagePreference::Portable => find_best_launch_path(&target_dir, &apps[index].id, &repo)?,
        PackagePreference::Installer => None,
    };
    let package_path = if is_wallpaper_package {
        Some(target_path.clone())
    } else {
        None
    };
    if extracted_archive {
        if let Some(executable_path) = executable_path.as_deref() {
            cleanup_redundant_archives_near_launch_path(executable_path);
        } else {
            let _ = fs::remove_file(&target_path);
        }
    }
    let result_file_path = executable_path
        .as_ref()
        .or(package_path.as_ref())
        .unwrap_or(&target_path)
        .to_string_lossy()
        .to_string();

    if let Some(latest_release) = releases.first() {
        apply_release_metadata(&mut apps[index], latest_release, &Utc::now().to_rfc3339());
    }
    apps[index].release_options = release_options(&releases);
    apps[index].package_preference = package_preference;
    apps[index].installed_version = Some(tag_name.clone());
    apps[index].selected_version = Some(tag_name.clone());
    if let Some(executable_path) = executable_path.as_ref() {
        apps[index].executable_path = Some(executable_path.to_string_lossy().to_string());
    }
    if let Some(package_path) = package_path.as_ref() {
        apps[index].package_path = Some(package_path.to_string_lossy().to_string());
        if apps[index].demo_url.is_some() {
            apps[index].executable_path = None;
        }
    }
    add_installed_version(
        &mut apps[index],
        tag_name.clone(),
        executable_path.as_deref(),
        package_path.as_deref(),
    );
    prune_installed_versions(&mut apps[index], previous_installed_version, &tag_name)?;

    save_apps(&app, &apps)?;
    Ok(DownloadResult {
        apps,
        file_path: result_file_path,
        install_folder: target_dir.to_string_lossy().to_string(),
    })
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                if let Err(error) = apply_initial_window_size(&app_handle, &window) {
                    eprintln!("Failed to initialize window size: {error}");
                }
                let resize_app = app_handle.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::Resized(size) = event {
                        if let Err(error) = persist_window_size(&resize_app, *size) {
                            eprintln!("Failed to save window size: {error}");
                        }
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            refresh_tools_catalog,
            save_settings,
            save_executable,
            get_default_install_dir,
            save_layout,
            reset_layout,
            launch_app,
            launch_agent_api_headless,
            open_release_url,
            open_repository_url,
            open_demo_url,
            check_control_center_update,
            open_control_center_release,
            install_control_center_update,
            open_install_folder,
            get_agent_control_info,
            get_agent_api_dashboard,
            save_agent_api_port,
            process_agent_control_commands,
            scan_releases,
            download_release,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Neko Legends Control Center");
}
