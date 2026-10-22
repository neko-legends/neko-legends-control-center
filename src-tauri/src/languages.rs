//! Shared language packs.
//!
//! The Control Center downloads font packs for non-English languages into one
//! shared folder so every Neko Legends app can use them without bundling its
//! own copies:
//!
//! ```text
//! <shared NekoLegends dir>/languages/<id>/pack.json
//! <shared NekoLegends dir>/languages/<id>/<font>.ttf
//! <shared NekoLegends dir>/languages/<id>/OFL.txt
//! ```
//!
//! Apps only need to read `languages/*/pack.json` (see languages/README.md).

use super::{
    app_data_dir, github_client, is_newer_version, read_json_file, shared_neko_legends_dir,
    write_json_file,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
use tauri::{AppHandle, Emitter};

pub const LANGUAGE_MANIFEST_URL: &str =
    "https://nekolegends.com/res/nekoLegendsControlCenter/languages/languages.json";
const BUNDLED_MANIFEST: &str = include_str!("../../catalog/languages.json");
const CACHED_MANIFEST_FILE: &str = "language-manifest.json";
const PACK_FILE: &str = "pack.json";
const PACK_SCHEMA_VERSION: u32 = 1;
const PROGRESS_EVENT: &str = "language-pack-progress";

static INSTALLS_IN_FLIGHT: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageManifest {
    pub schema_version: u32,
    #[serde(default)]
    pub pack_version: String,
    #[serde(default)]
    pub updated_at: Option<String>,
    pub base_url: String,
    pub packs: Vec<ManifestPack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestPack {
    pub id: String,
    pub name: String,
    pub native_name: String,
    #[serde(default)]
    pub script: String,
    pub version: String,
    #[serde(default)]
    pub rtl: bool,
    #[serde(default)]
    pub complex_shaping: bool,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub size_bytes: u64,
    pub files: Vec<ManifestFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestFile {
    pub path: String,
    pub kind: String,
    #[serde(default)]
    pub family: Option<String>,
    pub size: u64,
    pub sha256: String,
}

/// The `pack.json` written into each installed pack folder. Apps read this.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPack {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub native_name: String,
    #[serde(default)]
    pub script: String,
    pub version: String,
    #[serde(default)]
    pub rtl: bool,
    #[serde(default)]
    pub complex_shaping: bool,
    #[serde(default)]
    pub license: String,
    pub fonts: Vec<InstalledFont>,
    #[serde(default)]
    pub installed_at: Option<String>,
    #[serde(default)]
    pub installed_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledFont {
    pub file: String,
    pub family: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguagePackInfo {
    pub id: String,
    pub name: String,
    pub native_name: String,
    pub script: String,
    pub version: Option<String>,
    pub size_bytes: u64,
    pub rtl: bool,
    pub complex_shaping: bool,
    pub available: bool,
    pub installed: bool,
    pub installed_version: Option<String>,
    pub update_available: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguagePacksState {
    pub packs: Vec<LanguagePackInfo>,
    pub languages_dir: String,
    pub manifest_source: String,
    pub manifest_updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LanguagePackProgress {
    id: String,
    received_bytes: u64,
    total_bytes: u64,
    done: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguagePackRequest {
    pub id: String,
}

pub fn languages_dir() -> Result<PathBuf, String> {
    let dir = shared_neko_legends_dir()?.join("languages");
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir)
}

fn is_safe_pack_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 32
        && id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

/// Only `<pack id>/<file name>` paths are accepted from a manifest.
fn manifest_file_name<'a>(pack_id: &str, path: &'a str) -> Option<&'a str> {
    let (dir, name) = path.split_once('/')?;
    let safe_name = !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ','));
    (dir == pack_id && safe_name && !name.eq_ignore_ascii_case(PACK_FILE)).then_some(name)
}

fn is_valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn clean_manifest(mut manifest: LanguageManifest) -> Result<LanguageManifest, String> {
    if manifest.schema_version != PACK_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported language manifest schema {}.",
            manifest.schema_version
        ));
    }
    if !manifest.base_url.starts_with("https://") {
        return Err("Language manifest base URL must use https.".to_string());
    }
    if !manifest.base_url.ends_with('/') {
        manifest.base_url.push('/');
    }
    manifest.packs.retain(|pack| {
        is_safe_pack_id(&pack.id)
            && !pack.files.is_empty()
            && pack.files.iter().any(|file| file.kind == "font")
            && pack.files.iter().all(|file| {
                manifest_file_name(&pack.id, &file.path).is_some() && is_valid_sha256(&file.sha256)
            })
    });
    Ok(manifest)
}

fn parse_manifest(raw: &[u8]) -> Result<LanguageManifest, String> {
    let manifest =
        serde_json::from_slice::<LanguageManifest>(raw).map_err(|err| err.to_string())?;
    clean_manifest(manifest)
}

fn bundled_manifest() -> LanguageManifest {
    parse_manifest(BUNDLED_MANIFEST.as_bytes()).unwrap_or(LanguageManifest {
        schema_version: PACK_SCHEMA_VERSION,
        pack_version: String::new(),
        updated_at: None,
        base_url: "https://nekolegends.com/res/nekoLegendsControlCenter/languages/".to_string(),
        packs: Vec::new(),
    })
}

fn cached_manifest_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join(CACHED_MANIFEST_FILE))
}

/// The cached remote manifest when it is at least as new as the bundled one.
fn effective_manifest(app: &AppHandle) -> (LanguageManifest, &'static str) {
    let bundled = bundled_manifest();
    let cached = cached_manifest_path(app)
        .ok()
        .and_then(|path| fs::read(path).ok())
        .and_then(|raw| parse_manifest(&raw).ok());
    match cached {
        Some(manifest) if !is_newer_version(&bundled.pack_version, &manifest.pack_version) => {
            (manifest, "remote")
        }
        _ => (bundled, "bundled"),
    }
}

pub fn read_installed_packs(root: &Path) -> Vec<InstalledPack> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut packs = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|entry| {
            let dir = entry.path();
            let pack: Option<InstalledPack> = read_json_file(&dir.join(PACK_FILE), None);
            pack.filter(|pack| {
                pack.fonts
                    .iter()
                    .all(|font| dir.join(&font.file).is_file())
            })
        })
        .collect::<Vec<_>>();
    packs.sort_by(|left, right| left.id.cmp(&right.id));
    packs
}

fn build_state(app: &AppHandle) -> Result<LanguagePacksState, String> {
    let root = languages_dir()?;
    let (manifest, source) = effective_manifest(app);
    let installed = read_installed_packs(&root);

    let mut packs = manifest
        .packs
        .iter()
        .map(|pack| {
            let installed_pack = installed.iter().find(|candidate| candidate.id == pack.id);
            let installed_version = installed_pack.map(|candidate| candidate.version.clone());
            LanguagePackInfo {
                id: pack.id.clone(),
                name: pack.name.clone(),
                native_name: pack.native_name.clone(),
                script: pack.script.clone(),
                version: Some(pack.version.clone()),
                size_bytes: pack.files.iter().map(|file| file.size).sum(),
                rtl: pack.rtl,
                complex_shaping: pack.complex_shaping,
                available: true,
                installed: installed_pack.is_some(),
                update_available: installed_version
                    .as_deref()
                    .is_some_and(|version| is_newer_version(&pack.version, version)),
                installed_version,
            }
        })
        .collect::<Vec<_>>();

    for pack in &installed {
        if packs.iter().any(|candidate| candidate.id == pack.id) {
            continue;
        }
        packs.push(LanguagePackInfo {
            id: pack.id.clone(),
            name: pack.name.clone(),
            native_name: pack.native_name.clone(),
            script: pack.script.clone(),
            version: None,
            size_bytes: 0,
            rtl: pack.rtl,
            complex_shaping: pack.complex_shaping,
            available: false,
            installed: true,
            installed_version: Some(pack.version.clone()),
            update_available: false,
        });
    }

    Ok(LanguagePacksState {
        packs,
        languages_dir: root.to_string_lossy().to_string(),
        manifest_source: source.to_string(),
        manifest_updated_at: manifest.updated_at.clone(),
    })
}

fn installs_in_flight() -> &'static Mutex<HashSet<String>> {
    INSTALLS_IN_FLIGHT.get_or_init(|| Mutex::new(HashSet::new()))
}

struct InstallGuard(String);

impl InstallGuard {
    fn acquire(id: &str) -> Result<Self, String> {
        let mut active = installs_in_flight()
            .lock()
            .map_err(|_| "Language install lock is unavailable.".to_string())?;
        if !active.insert(id.to_string()) {
            return Err(format!("{id} is already being installed."));
        }
        Ok(Self(id.to_string()))
    }
}

impl Drop for InstallGuard {
    fn drop(&mut self) {
        if let Ok(mut active) = installs_in_flight().lock() {
            active.remove(&self.0);
        }
    }
}

fn emit_progress(app: &AppHandle, id: &str, received_bytes: u64, total_bytes: u64, done: bool) {
    let _ = app.emit(
        PROGRESS_EVENT,
        LanguagePackProgress {
            id: id.to_string(),
            received_bytes,
            total_bytes,
            done,
        },
    );
}

fn remove_dir_if_exists(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|err| format!("Could not remove {}: {err}", path.display()))?;
    }
    Ok(())
}

/// Move a fully downloaded staging folder into place, keeping the previous
/// version until the swap succeeds.
fn swap_into_place(root: &Path, id: &str, staging: &Path) -> Result<PathBuf, String> {
    let target = root.join(id);
    let backup = root.join(format!(".{id}.old"));
    remove_dir_if_exists(&backup)?;
    if target.exists() {
        fs::rename(&target, &backup).map_err(|err| {
            format!("Could not replace the installed {id} pack (is a file in use?): {err}")
        })?;
    }
    if let Err(err) = fs::rename(staging, &target) {
        if backup.exists() {
            let _ = fs::rename(&backup, &target);
        }
        return Err(format!("Could not finish installing {id}: {err}"));
    }
    let _ = fs::remove_dir_all(&backup);
    Ok(target)
}

fn installed_pack_from_manifest(pack: &ManifestPack) -> InstalledPack {
    InstalledPack {
        schema_version: PACK_SCHEMA_VERSION,
        id: pack.id.clone(),
        name: pack.name.clone(),
        native_name: pack.native_name.clone(),
        script: pack.script.clone(),
        version: pack.version.clone(),
        rtl: pack.rtl,
        complex_shaping: pack.complex_shaping,
        license: pack.license.clone(),
        fonts: pack
            .files
            .iter()
            .filter(|file| file.kind == "font")
            .filter_map(|file| {
                Some(InstalledFont {
                    file: manifest_file_name(&pack.id, &file.path)?.to_string(),
                    family: file.family.clone().unwrap_or_else(|| pack.name.clone()),
                })
            })
            .collect(),
        installed_at: Some(Utc::now().to_rfc3339()),
        installed_by: Some(format!(
            "{} {}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        )),
    }
}

async fn download_pack(
    app: &AppHandle,
    manifest: &LanguageManifest,
    pack: &ManifestPack,
    staging: &Path,
) -> Result<(), String> {
    let client = github_client()?;
    let total_bytes: u64 = pack.files.iter().map(|file| file.size).sum();
    let mut received_bytes = 0_u64;
    emit_progress(app, &pack.id, 0, total_bytes, false);

    for file in &pack.files {
        let name = manifest_file_name(&pack.id, &file.path)
            .ok_or_else(|| format!("Unsafe file path in manifest: {}", file.path))?;
        let url = format!("{}{}", manifest.base_url, file.path);
        let mut response = client.get(&url).send().await.map_err(|err| err.to_string())?;
        if !response.status().is_success() {
            return Err(format!("{name} download failed with {}.", response.status()));
        }

        let target = staging.join(name);
        let mut output = fs::File::create(&target).map_err(|err| err.to_string())?;
        let mut hasher = Sha256::new();
        let mut file_bytes = 0_u64;
        while let Some(chunk) = response.chunk().await.map_err(|err| err.to_string())? {
            file_bytes += chunk.len() as u64;
            if file_bytes > file.size {
                return Err(format!("{name} is larger than the manifest says."));
            }
            hasher.update(&chunk);
            output.write_all(&chunk).map_err(|err| err.to_string())?;
            received_bytes += chunk.len() as u64;
            emit_progress(app, &pack.id, received_bytes, total_bytes, false);
        }
        output.flush().map_err(|err| err.to_string())?;
        drop(output);

        if file_bytes != file.size {
            return Err(format!("{name} download was incomplete."));
        }
        let digest = format!("{:x}", hasher.finalize());
        if !digest.eq_ignore_ascii_case(&file.sha256) {
            return Err(format!("{name} failed its checksum."));
        }
    }

    write_json_file(&staging.join(PACK_FILE), &installed_pack_from_manifest(pack))
}

#[tauri::command]
pub fn get_language_packs(app: AppHandle) -> Result<LanguagePacksState, String> {
    build_state(&app)
}

#[tauri::command]
pub async fn refresh_language_packs(app: AppHandle) -> Result<LanguagePacksState, String> {
    let client = github_client()?;
    let response = client
        .get(LANGUAGE_MANIFEST_URL)
        .send()
        .await
        .map_err(|err| err.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Language manifest returned {}.", response.status()));
    }
    let bytes = response.bytes().await.map_err(|err| err.to_string())?;
    let manifest = parse_manifest(&bytes)?;
    write_json_file(&cached_manifest_path(&app)?, &manifest)?;
    build_state(&app)
}

#[tauri::command]
pub async fn install_language_pack(
    app: AppHandle,
    request: LanguagePackRequest,
) -> Result<LanguagePacksState, String> {
    if !is_safe_pack_id(&request.id) {
        return Err("Unknown language pack.".to_string());
    }
    let _guard = InstallGuard::acquire(&request.id)?;
    let (manifest, _) = effective_manifest(&app);
    let pack = manifest
        .packs
        .iter()
        .find(|pack| pack.id == request.id)
        .cloned()
        .ok_or_else(|| format!("{} is not in the language catalog.", request.id))?;

    let root = languages_dir()?;
    let staging = root.join(format!(".{}.partial", pack.id));
    remove_dir_if_exists(&staging)?;
    fs::create_dir_all(&staging).map_err(|err| err.to_string())?;

    let result = download_pack(&app, &manifest, &pack, &staging).await;
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&staging);
        emit_progress(&app, &pack.id, 0, 0, true);
        return Err(error);
    }
    let swapped = swap_into_place(&root, &pack.id, &staging);
    emit_progress(&app, &pack.id, pack.size_bytes, pack.size_bytes, true);
    if let Err(error) = swapped {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    build_state(&app)
}

#[tauri::command]
pub fn remove_language_pack(
    app: AppHandle,
    request: LanguagePackRequest,
) -> Result<LanguagePacksState, String> {
    if !is_safe_pack_id(&request.id) {
        return Err("Unknown language pack.".to_string());
    }
    let _guard = InstallGuard::acquire(&request.id)?;
    remove_dir_if_exists(&languages_dir()?.join(&request.id))?;
    build_state(&app)
}

#[tauri::command]
pub fn open_languages_folder() -> Result<(), String> {
    open::that(languages_dir()?).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_manifest_parses() {
        let manifest = bundled_manifest();
        assert!(!manifest.packs.is_empty());
        for pack in &manifest.packs {
            assert!(is_safe_pack_id(&pack.id));
            assert!(!installed_pack_from_manifest(pack).fonts.is_empty());
        }
    }

    #[test]
    fn rejects_unsafe_manifest_paths() {
        assert_eq!(manifest_file_name("ja", "ja/Font.ttf"), Some("Font.ttf"));
        assert_eq!(manifest_file_name("ja", "ja/Noto,Thai.ttf"), Some("Noto,Thai.ttf"));
        assert_eq!(manifest_file_name("ja", "ko/Font.ttf"), None);
        assert_eq!(manifest_file_name("ja", "ja/../evil.ttf"), None);
        assert_eq!(manifest_file_name("ja", "ja/sub/evil.ttf"), None);
        assert_eq!(manifest_file_name("ja", "ja/pack.json"), None);
        assert_eq!(manifest_file_name("ja", "ja/.hidden"), None);
        assert!(!is_safe_pack_id("../ja"));
        assert!(is_safe_pack_id("zh-Hans"));
    }

    #[test]
    fn reads_installed_packs_and_skips_incomplete_ones() {
        let root = std::env::temp_dir().join(format!(
            "neko-language-packs-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let manifest = bundled_manifest();
        let pack = &manifest.packs[0];
        let complete = root.join(&pack.id);
        fs::create_dir_all(&complete).unwrap();
        let installed = installed_pack_from_manifest(pack);
        for font in &installed.fonts {
            fs::write(complete.join(&font.file), b"font").unwrap();
        }
        write_json_file(&complete.join(PACK_FILE), &installed).unwrap();

        let broken = root.join("broken");
        fs::create_dir_all(&broken).unwrap();
        let mut broken_pack = installed.clone();
        broken_pack.id = "broken".to_string();
        write_json_file(&broken.join(PACK_FILE), &broken_pack).unwrap();

        let staging = root.join(".ko.partial");
        fs::create_dir_all(&staging).unwrap();

        let packs = read_installed_packs(&root);
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].id, pack.id);

        let replacement = root.join(".swap.partial");
        fs::create_dir_all(&replacement).unwrap();
        fs::write(replacement.join("marker"), b"new").unwrap();
        let target = swap_into_place(&root, &pack.id, &replacement).unwrap();
        assert!(target.join("marker").exists());
        assert!(!root.join(format!(".{}.old", pack.id)).exists());

        let _ = fs::remove_dir_all(&root);
    }
}
