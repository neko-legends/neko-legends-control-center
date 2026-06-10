#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tauri::{AppHandle, Manager, PhysicalSize, Size, WebviewWindow, WindowEvent};
#[cfg(windows)]
use winreg::{enums::*, RegKey};

const MIN_WINDOW_WIDTH: u32 = 520;
const MIN_WINDOW_HEIGHT: u32 = 390;
const FALLBACK_WINDOW_WIDTH: u32 = 720;
const FALLBACK_WINDOW_HEIGHT: u32 = 520;
const GITHUB_OWNER: &str = "neko-legends";
const CONTROL_CENTER_REPO: &str = "NekoLegendsControlCenter";
const TOOLS_CATALOG_URL: &str = "https://nekolegends.com/res/nekoLegendsControlCenter/tools.json";
const UNDER_DEVELOPMENT_CATEGORY: &str = "Under Development";
const VENICE_MEDIA_LOCAL_ID: &str = "venice-media-local";
const VENICE_MEDIA_LOCAL_DISPLAY_NAME: &str = "Venice Media Local";

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
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadRequest {
    app_id: String,
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
    3
}

fn default_category() -> String {
    "Work Stuff".to_string()
}

fn default_categories() -> Vec<String> {
    vec![
        "Work Stuff".to_string(),
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
        app("batchlapse", "BatchLapse", "BatchLapse", "Batch video timelapse exporter for MP4, WebM, and GitHub-friendly GIFs.", "#5b8def", "BL", "Work Stuff", ToolStatus::Available, None),
        app("depth-map-ai-generator", "DepthMap AI", "DepthMapAIGenerator", "Batch depth-map and WebP generator for local AI image workflows.", "#43b883", "DM", UNDER_DEVELOPMENT_CATEGORY, ToolStatus::ComingSoon, None),
        app("image-to-ascii-3d", "ASCII 3D", "ImageToASCII3D", "Image-to-ASCII converter with optional depth-map driven 3D parallax exports.", "#f0a848", "A3", UNDER_DEVELOPMENT_CATEGORY, ToolStatus::ComingSoon, None),
        app("markrush", "MarkRush", "MarkRush", "Fast local Markdown viewer/editor built for huge files and folders.", "#e05d7b", "MR", "Work Stuff", ToolStatus::Available, None),
        app("opensplit", "OpenSplit", "OpenSplit", "Multi-pane terminal harness for AI coding agents, shells, and SSH sessions.", "#4fb6d8", "OS", "Work Stuff", ToolStatus::Available, None),
        app("venice-media-local", "Venice Media", "VeniceMediaLocal", "Local Venice API media workspace for images, video, music, voice, and cleanup.", "#34c6a3", "VM", "Work Stuff", ToolStatus::Available, None),
        app("purpleplanet", "PurplePlanet", "PurplePlanet", "Luminous Three.js planet motion art for live wallpapers and screensavers.", "#8c65df", "PP", "Fun Stuff", ToolStatus::Available, Some("https://nekolegends.com/res/projects/purplePlanet/")),
        app("stargaze", "StarGaze", "StarGaze", "Glittering Three.js starfield wallpaper and screensaver with tunable motion.", "#6b7cff", "SG", "Fun Stuff", ToolStatus::Available, Some("https://nekolegends.com/res/projects/starGaze/")),
    ]
}

fn app(id: &str, name: &str, repo: &str, description: &str, accent: &str, icon: &str, category: &str, status: ToolStatus, demo_url: Option<&str>) -> LauncherApp {
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
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
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
        || !repo
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
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
    if launcher_app.status == ToolStatus::ComingSoon {
        launcher_app.category = UNDER_DEVELOPMENT_CATEGORY.to_string();
    }
    launcher_app.demo_url = clean_optional_https_url(launcher_app.demo_url);
    launcher_app.executable_path = None;
    launcher_app.installed_version = None;
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
            if !tools.iter().any(|existing: &LauncherApp| existing.id == launcher_app.id) {
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
    let mut apps = merge_catalog_apps(saved, read_tools_catalog(app).tools);
    auto_detect_installed_apps(&mut apps);
    apps
}

fn merge_catalog_apps(saved: Vec<LauncherApp>, defaults: Vec<LauncherApp>) -> Vec<LauncherApp> {
    let mut merged = Vec::new();

    for saved_app in saved {
        if let Some(default_app) = defaults.iter().find(|candidate| candidate.id == saved_app.id) {
            let mut app = default_app.clone();
            app.executable_path = saved_app.executable_path;
            app.installed_version = saved_app.installed_version;
            app.latest_version = saved_app.latest_version;
            app.release_url = saved_app.release_url;
            app.release_checked_at = saved_app.release_checked_at;
            app.release_notes = saved_app.release_notes;
            app.release_options = saved_app.release_options;
            app.package_preference = saved_app.package_preference;
            app.package_path = saved_app.package_path;
            app.visible = saved_app.visible;
            if app.status == ToolStatus::ComingSoon {
                app.latest_version = None;
                app.release_url = None;
                app.release_checked_at = None;
                app.release_notes = Some("Coming soon.".to_string());
                app.release_options = Vec::new();
            }
            if !saved_app.category.trim().is_empty() {
                app.category = saved_app.category;
            }
            merged.push(app);
        }
    }

    for default_app in defaults {
        if !merged.iter().any(|candidate| candidate.id == default_app.id) {
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
        if let Some(install) = detect_local_install(launcher_app).or_else(|| detect_installed_app(launcher_app)) {
            if let Some(executable_path) = install.executable_path {
                launcher_app.executable_path = Some(executable_path.to_string_lossy().to_string());
            }
            if let Some(package_path) = install.package_path {
                launcher_app.package_path = Some(package_path.to_string_lossy().to_string());
            }
            if install.version.is_some() {
                launcher_app.installed_version = install.version;
            }
        }
    }
}

fn path_option_exists(path: &Option<String>) -> bool {
    path
        .as_deref()
        .is_some_and(|path| Path::new(path).exists())
}

fn app_download_artifact_exists(launcher_app: &LauncherApp) -> bool {
    if launcher_app.demo_url.is_some() {
        path_option_exists(&launcher_app.package_path)
    } else {
        path_option_exists(&launcher_app.executable_path)
    }
}

fn detect_local_install(launcher_app: &LauncherApp) -> Option<DetectedInstall> {
    let root = default_install_dir().ok()?.join(&launcher_app.id);
    if !root.exists() {
        return None;
    }

    if launcher_app.demo_url.is_some() {
        let package_path = find_best_package_archive(&root, &launcher_app.id, &launcher_app.repo).ok().flatten()?;
        return Some(DetectedInstall {
            version: version_from_install_path(&root, &package_path),
            executable_path: None,
            package_path: Some(package_path),
        });
    }

    let launch_path = find_best_launch_path(&root, &launcher_app.id, &launcher_app.repo).ok().flatten()?;
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
        (RegKey::predef(HKEY_CURRENT_USER), "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
        (RegKey::predef(HKEY_LOCAL_MACHINE), "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
        (RegKey::predef(HKEY_LOCAL_MACHINE), "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
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
fn detected_install_from_registry_key(key: &RegKey, launcher_app: &LauncherApp) -> Option<DetectedInstall> {
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
                candidates.extend(expected_executable_names(launcher_app).into_iter().map(|name| path.join(name)));
                if let Ok(Some(best)) = find_best_executable(&path, &launcher_app.id, &launcher_app.repo) {
                    candidates.push(best);
                }
            }
        }
    }

    if let Ok(value) = key.get_value::<String, _>("UninstallString") {
        if let Some(path) = command_path_from_registry_value(&value) {
            if let Some(parent) = path.parent() {
                candidates.extend(expected_executable_names(launcher_app).into_iter().map(|name| parent.join(name)));
                if let Ok(Some(best)) = find_best_executable(parent, &launcher_app.id, &launcher_app.repo) {
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
    !(name.contains("setup") || name.contains("installer") || name.contains("uninstall") || name.contains("update"))
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
        .user_agent("NekoLegendsControlCenter/26.6.9")
        .build()
        .map_err(|err| err.to_string())
}

fn apply_release_metadata(launcher_app: &mut LauncherApp, release: &GitHubRelease, checked_at: &str) {
    launcher_app.latest_version = Some(release.tag_name.clone());
    launcher_app.release_url = Some(release.html_url.clone());
    launcher_app.release_notes = release.body.as_ref().map(|body| body.chars().take(240).collect());
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

async fn fetch_releases(client: &reqwest::Client, repo: &str) -> Result<Vec<GitHubRelease>, String> {
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
    if name.contains("setup") || name.contains("installer") || name.contains("uninstall") || name.contains("update") {
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

    Ok(best
        .filter(|(score, _)| *score > 20)
        .map(|(_, path)| path))
}

fn is_web_launch_path(path: &Path) -> bool {
    path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm"))
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

fn find_best_web_launch_path(root: &Path, app_id: &str, repo: &str) -> Result<Option<PathBuf>, String> {
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

fn find_best_package_archive(root: &Path, app_id: &str, repo: &str) -> Result<Option<PathBuf>, String> {
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
    let package_path = package_path.canonicalize().unwrap_or_else(|_| package_path.to_path_buf());
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

fn preferred_window_size(settings: &AppSettings, monitor_size: Option<PhysicalSize<u32>>) -> PhysicalSize<u32> {
    let monitor_width = monitor_size
        .as_ref()
        .map(|size| size.width)
        .unwrap_or(FALLBACK_WINDOW_WIDTH);
    let monitor_height = monitor_size
        .as_ref()
        .map(|size| size.height)
        .unwrap_or(FALLBACK_WINDOW_HEIGHT);
    let width = settings
        .window_width
        .unwrap_or_else(|| monitor_width.saturating_div(4).max(FALLBACK_WINDOW_WIDTH));
    let height = settings
        .window_height
        .unwrap_or_else(|| monitor_height.saturating_div(4).max(FALLBACK_WINDOW_HEIGHT));

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
    if let Some(categories) = request.categories {
        settings.categories = normalize_categories(categories);
    }
    write_json_file(&settings_path(&app)?, &settings)?;
    Ok(settings)
}

#[tauri::command]
fn save_executable(app: AppHandle, request: SaveExecutableRequest) -> Result<Vec<LauncherApp>, String> {
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
    let mut next_apps = merge_catalog_apps(request.apps, read_tools_catalog(&app).tools);
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

    for mut default_app in read_tools_catalog(&app).tools {
        if let Some(existing) = current_apps.iter().find(|candidate| candidate.id == default_app.id) {
            default_app.executable_path = existing.executable_path.clone();
            default_app.installed_version = existing.installed_version.clone();
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

#[tauri::command]
fn launch_app(app: AppHandle, request: LaunchRequest) -> Result<(), String> {
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
    if is_launchable_executable_path(&path) {
        Command::new(path)
            .spawn()
            .map(|_| ())
            .map_err(|err| err.to_string())
    } else if is_web_launch_path(&path) {
        open::that(path).map_err(|err| err.to_string())
    } else {
        Err("Configured launch file is not supported".to_string())
    }
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
        .unwrap_or_else(|| format!("https://github.com/{}/{}/releases", GITHUB_OWNER, launcher_app.repo));
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
            let release_notes = latest.and_then(|release| release.body.as_ref().map(|body| body.chars().take(240).collect()));
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
        .unwrap_or_else(|| format!("https://github.com/{}/{}/releases", GITHUB_OWNER, CONTROL_CENTER_REPO));
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
        return Err("Automatic Control Center updates are currently supported on Windows portable builds.".to_string());
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
        return Err("Automatic Control Center updates are currently supported on Windows portable builds.".to_string());
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
    let asset = best_control_center_asset(release)
        .ok_or_else(|| "The latest Control Center release does not have a portable Windows download yet.".to_string())?;
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
        return Err(format!("Control Center update download failed with {}.", response.status()));
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
        find_control_center_update_executable(&staging_dir)?
            .ok_or_else(|| "The Control Center update package did not include a launchable Windows app.".to_string())?
    } else {
        return Err("The latest Control Center download is not a portable Windows app.".to_string());
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

#[tauri::command]
async fn scan_releases(app: AppHandle) -> Result<Vec<LauncherApp>, String> {
    let _ = refresh_tools_catalog(app.clone()).await;
    let client = github_client()?;
    let mut apps = read_apps(&app);
    let checked_at = Utc::now().to_rfc3339();

    for launcher_app in apps.iter_mut() {
        if launcher_app.status == ToolStatus::ComingSoon {
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
    if apps[index].status == ToolStatus::ComingSoon {
        return Err(format!("{} is coming soon.", apps[index].name));
    }
    let repo = apps[index].repo.clone();
    let package_preference = request
        .package_preference
        .clone()
        .unwrap_or_else(|| apps[index].package_preference.clone());
    let releases = fetch_releases(&client, &repo).await?;
    let release = request
        .version
        .as_deref()
        .and_then(|version| releases.iter().find(|candidate| candidate.tag_name == version))
        .or_else(|| releases.first())
        .ok_or_else(|| "No public releases found yet.".to_string())?;
    let asset = best_release_asset(release, &package_preference)
        .ok_or_else(|| match package_preference {
            PackagePreference::Portable => "Selected release does not have a portable Windows download asset.".to_string(),
            PackagePreference::Installer => "Selected release does not have a Windows installer asset.".to_string(),
        })?;
    let file_name = safe_file_segment(&asset.name);
    let tag_name = release.tag_name.clone();
    let target_dir = default_install_dir()?
        .join(&apps[index].id)
        .join(safe_file_segment(&tag_name));
    fs::create_dir_all(&target_dir).map_err(|err| err.to_string())?;
    let target_path = target_dir.join(file_name);

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

    let downloaded_name = target_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let executable_path = match package_preference {
        PackagePreference::Portable if target_path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
            && !is_installer_asset_name(&downloaded_name) => Some(target_path.clone()),
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
    apps[index].installed_version = Some(tag_name);
    if let Some(executable_path) = executable_path {
        apps[index].executable_path = Some(executable_path.to_string_lossy().to_string());
    }
    if let Some(package_path) = package_path {
        apps[index].package_path = Some(package_path.to_string_lossy().to_string());
        if apps[index].demo_url.is_some() {
            apps[index].executable_path = None;
        }
    }

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
            open_release_url,
            open_repository_url,
            open_demo_url,
            check_control_center_update,
            open_control_center_release,
            install_control_center_update,
            open_install_folder,
            scan_releases,
            download_release,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Neko Legends Control Center");
}
