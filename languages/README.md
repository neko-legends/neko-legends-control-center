# Shared Language Packs

Neko Legends apps ship with English only. Fonts for other languages are
downloaded once by the Control Center (Settings → Languages, or the 文A
button in the top bar) into a shared folder that every app reads:

| OS      | Folder                                                    |
| ------- | --------------------------------------------------------- |
| Windows | `%APPDATA%\NekoLegends\languages\`                        |
| macOS   | `~/Library/Application Support/NekoLegends/languages/`    |
| Linux   | `$XDG_CONFIG_HOME/NekoLegends/languages/` (or `~/.config`) |

This is the same `NekoLegends` folder that already holds
`agent-api-registry.json`.

## Packs

| id        | Language             | Font                      |
| --------- | -------------------- | ------------------------- |
| `ja`      | Japanese             | Noto Sans JP (400–700)    |
| `zh-Hans` | Chinese (Simplified) | Noto Sans SC (400–700)    |
| `ko`      | Korean               | Noto Sans KR (400–700)    |
| `ru`      | Russian / Cyrillic   | Noto Sans (400–700)       |
| `th`      | Thai                 | Noto Sans Thai (400–700)  |
| `ar`      | Arabic               | Changa Medium             |

The variable fonts are trimmed so their default instance is Regular (the
originals default to Thin) and they keep a 400–700 weight range for bold.
All fonts are SIL OFL 1.1; each pack folder carries its `OFL.txt`.

## Reading packs from an app

Scan `languages/*/pack.json`. Skip folders that start with `.` (in-progress
installs) and packs whose font files are missing.

```json
{
  "schemaVersion": 1,
  "id": "ja",
  "name": "Japanese",
  "nativeName": "日本語",
  "script": "Japanese (Kana + Kanji)",
  "version": "26.10.21",
  "rtl": false,
  "complexShaping": false,
  "license": "OFL-1.1",
  "fonts": [{ "file": "NotoSansJP-wght400-700.ttf", "family": "Noto Sans JP" }],
  "installedAt": "2026-10-21T05:00:00Z"
}
```

`fonts[].file` is relative to the pack folder. Use the fonts as fallbacks
after your normal UI font so English keeps its look and other scripts stop
showing boxes. Apps keep their own UI translations (they are small and app
specific); a UI language should only be offered when its pack is installed.

`rtl` and `complexShaping` warn that a renderer without text shaping (egui,
for example) will draw that script imperfectly. Web-based apps (Tauri,
Electron, browsers) shape these correctly.

Re-scan when your window regains focus so packs installed while the app is
open show up without a restart.

## Building and publishing

```text
pip install fonttools
python scripts/build-language-packs.py            # -> release/language-packs + catalog/languages.json
python scripts/build-language-packs.py --install  # also install locally for testing
scripts/upload-language-packs.local.ps1           # -> nekolegends.com/res/nekoLegendsControlCenter/languages/
```

Sources and settings live in `languages/packs.json`. Bump `packVersion` when
fonts change so installed packs show **Update**. `catalog/languages.json` is
bundled into the Control Center as the offline list.
