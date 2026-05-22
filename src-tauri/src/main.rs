#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{AppHandle, Manager, PhysicalSize, Size, WebviewWindow, WindowEvent};

const MIN_WINDOW_WIDTH: u32 = 520;
const MIN_WINDOW_HEIGHT: u32 = 390;
const FALLBACK_WINDOW_WIDTH: u32 = 720;
const FALLBACK_WINDOW_HEIGHT: u32 = 520;

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
    #[serde(default = "default_true")]
    visible: bool,
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
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_category() -> String {
    "Work Stuff".to_string()
}

fn default_categories() -> Vec<String> {
    vec!["Work Stuff".to_string(), "Fun Stuff".to_string()]
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
        app("batchlapse", "BatchLapse", "BatchLapse", "Batch tools for image and media workflows.", "#5b8def", "BL", "Work Stuff"),
        app("depth-map-ai-generator", "DepthMap AI", "DepthMapAIGenerator", "Depth map generation utilities.", "#43b883", "DM", "Work Stuff"),
        app("image-to-ascii-3d", "ASCII 3D", "ImageToASCII3D", "Image-to-ASCII 3D conversion.", "#f0a848", "A3", "Work Stuff"),
        app("markrush", "MarkRush", "MarkRush", "Markdown-focused writing and publishing tools.", "#e05d7b", "MR", "Work Stuff"),
        app("opensplit", "OpenSplit", "OpenSplit", "Split-screen and window workflow utility.", "#4fb6d8", "OS", "Work Stuff"),
        app("venice-media-local", "Venice Media", "VeniceMediaLocal", "Local Venice media generator.", "#34c6a3", "VM", "Work Stuff"),
        app("purpleplanet", "PurplePlanet", "PurplePlanet", "Creative app from the ForPublic collection.", "#8c65df", "PP", "Fun Stuff"),
        app("stargaze", "StarGaze", "StarGaze", "Astronomy and sky-oriented utility.", "#6b7cff", "SG", "Fun Stuff"),
    ]
}

fn app(id: &str, name: &str, repo: &str, description: &str, accent: &str, icon: &str, category: &str) -> LauncherApp {
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
    merge_default_apps(saved)
}

fn merge_default_apps(saved: Vec<LauncherApp>) -> Vec<LauncherApp> {
    let mut merged = Vec::new();
    let defaults = default_apps();

    for saved_app in saved {
        if let Some(default_app) = defaults.iter().find(|candidate| candidate.id == saved_app.id) {
            let mut app = default_app.clone();
            app.executable_path = saved_app.executable_path;
            app.installed_version = saved_app.installed_version;
            app.latest_version = saved_app.latest_version;
            app.release_url = saved_app.release_url;
            app.release_checked_at = saved_app.release_checked_at;
            app.release_notes = saved_app.release_notes;
            app.visible = saved_app.visible;
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
        return Err("Executable path does not exist".to_string());
    }
    target.executable_path = Some(path.to_string_lossy().to_string());
    save_apps(&app, &apps)?;
    Ok(apps)
}

#[tauri::command]
fn save_layout(app: AppHandle, request: SaveLayoutRequest) -> Result<ControlCenterState, String> {
    let mut next_apps = merge_default_apps(request.apps);
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

    for mut default_app in default_apps() {
        if let Some(existing) = current_apps.iter().find(|candidate| candidate.id == default_app.id) {
            default_app.executable_path = existing.executable_path.clone();
            default_app.installed_version = existing.installed_version.clone();
            default_app.latest_version = existing.latest_version.clone();
            default_app.release_url = existing.release_url.clone();
            default_app.release_checked_at = existing.release_checked_at.clone();
            default_app.release_notes = existing.release_notes.clone();
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
        return Err("No executable path has been configured for this app".to_string());
    };
    let path = PathBuf::from(executable_path);
    if !path.exists() {
        return Err("Configured executable no longer exists".to_string());
    }
    Command::new(path)
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
        .unwrap_or_else(|| format!("https://github.com/flashosophy/{}/releases", launcher_app.repo));
    open::that(url).map_err(|err| err.to_string())
}

#[tauri::command]
async fn scan_releases(app: AppHandle) -> Result<Vec<LauncherApp>, String> {
    let client = reqwest::Client::builder()
        .user_agent("NekoLegendsControlCenter/26.5.21")
        .build()
        .map_err(|err| err.to_string())?;
    let mut apps = read_apps(&app);
    let checked_at = Utc::now().to_rfc3339();

    for launcher_app in apps.iter_mut() {
        let url = format!(
            "https://api.github.com/repos/flashosophy/{}/releases/latest",
            launcher_app.repo
        );
        match client.get(url).send().await {
            Ok(response) if response.status().is_success() => {
                let release = response
                    .json::<GitHubRelease>()
                    .await
                    .map_err(|err| err.to_string())?;
                launcher_app.latest_version = Some(release.tag_name);
                launcher_app.release_url = Some(release.html_url);
                launcher_app.release_notes = release.body.map(|body| body.chars().take(240).collect());
                launcher_app.release_checked_at = Some(checked_at.clone());
            }
            Ok(response) if response.status().as_u16() == 404 => {
                launcher_app.latest_version = None;
                launcher_app.release_url = Some(format!(
                    "https://github.com/flashosophy/{}/releases",
                    launcher_app.repo
                ));
                launcher_app.release_notes = Some("No public releases found yet.".to_string());
                launcher_app.release_checked_at = Some(checked_at.clone());
            }
            Ok(response) => {
                launcher_app.release_notes = Some(format!("GitHub returned {}.", response.status()));
                launcher_app.release_checked_at = Some(checked_at.clone());
            }
            Err(error) => {
                launcher_app.release_notes = Some(error.to_string());
                launcher_app.release_checked_at = Some(checked_at.clone());
            }
        }
    }

    save_apps(&app, &apps)?;
    Ok(apps)
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
            save_settings,
            save_executable,
            save_layout,
            reset_layout,
            launch_app,
            open_release_url,
            scan_releases,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Neko Legends Control Center");
}
