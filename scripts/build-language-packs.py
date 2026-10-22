#!/usr/bin/env python3
"""Build the shared Neko Legends language packs.

Reads languages/packs.json, turns each source font into a pack font
(variable fonts are trimmed to a Regular-default 400-700 weight range so
renderers that ignore variation axes still draw Regular, not Thin), writes
an OFL notice per pack, and produces the hosted manifest:

  release/language-packs/
    languages.json            <- upload with the pack folders
    ja/NotoSansJP-wght400-700.ttf
    ja/OFL.txt
    ...

It also refreshes catalog/languages.json, which the Control Center bundles
as its offline fallback list.

Requires: python 3.9+, fonttools  (pip install fonttools)

Examples:
  python scripts/build-language-packs.py
  python scripts/build-language-packs.py --only ja ko
  python scripts/build-language-packs.py --install   # also install into the shared folder for local testing
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PACKS_FILE = ROOT / "languages" / "packs.json"
OFL_BODY_FILE = ROOT / "languages" / "OFL-1.1.txt"
DEFAULT_OUT = ROOT / "release" / "language-packs"
CATALOG_COPY = ROOT / "catalog" / "languages.json"
CACHE_FILE_NAME = ".build-cache.json"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def shared_languages_dir() -> Path:
    """Must match shared_neko_legends_dir() in the Control Center and the apps."""
    if sys.platform.startswith("win"):
        base = Path(os.environ.get("APPDATA") or os.environ["USERPROFILE"])
    elif sys.platform == "darwin":
        base = Path.home() / "Library" / "Application Support"
    else:
        base = Path(os.environ.get("XDG_CONFIG_HOME") or (Path.home() / ".config"))
    return base / "NekoLegends" / "languages"


def axis_limits(axes: dict) -> dict:
    limits = {}
    for tag, value in axes.items():
        if isinstance(value, list):
            minimum, default, maximum = value
            limits[tag] = (minimum, default, maximum)
        else:
            limits[tag] = value
    return limits


def build_font(source: Path, target: Path, axes: dict | None) -> None:
    from fontTools.ttLib import TTFont

    font = TTFont(source)
    if axes and "fvar" in font:
        from fontTools.varLib import instancer

        font = instancer.instantiateVariableFont(font, axis_limits(axes))
    elif axes:
        print(f"    note: {source.name} is not variable; axes ignored")
    target.parent.mkdir(parents=True, exist_ok=True)
    tmp = target.with_suffix(target.suffix + ".tmp")
    font.save(tmp)
    os.replace(tmp, target)


def font_copyright(path: Path) -> str:
    from fontTools.ttLib import TTFont

    font = TTFont(path, lazy=True)
    value = font["name"].getDebugName(0) or ""
    font.close()
    return value.strip()


def write_ofl(pack_dir: Path, fonts: list[Path], ofl_body: str) -> Path:
    lines = []
    for font in fonts:
        notice = font_copyright(font)
        if notice and notice not in lines:
            lines.append(notice)
    header = "\n".join(lines)
    text = (
        f"{header}\n\n"
        "This Font Software is licensed under the SIL Open Font License, Version 1.1.\n"
        "This license is copied below, and is also available with a FAQ at:\n"
        "https://openfontlicense.org\n\n"
        "Variable fonts in this pack were modified from their originals only by\n"
        "limiting variation axes (default weight set to Regular). Font names are\n"
        "unchanged.\n\n"
        f"{ofl_body.strip()}\n"
    )
    target = pack_dir / "OFL.txt"
    target.write_text(text, encoding="utf-8", newline="\n")
    return target


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--source", help="Folder with the source fonts (default: sourceFontsDir in packs.json)")
    parser.add_argument("--out", default=str(DEFAULT_OUT), help="Output folder (default: release/language-packs)")
    parser.add_argument("--only", nargs="*", help="Only rebuild these pack ids (others are kept from the cache)")
    parser.add_argument("--force", action="store_true", help="Rebuild fonts even when the cache says they are current")
    parser.add_argument("--install", action="store_true", help="Also install the packs into the shared NekoLegends languages folder")
    parser.add_argument("--no-catalog", action="store_true", help="Do not refresh catalog/languages.json")
    args = parser.parse_args()

    config = json.loads(PACKS_FILE.read_text(encoding="utf-8"))
    source_dir = Path(args.source or config["sourceFontsDir"])
    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    ofl_body = OFL_BODY_FILE.read_text(encoding="utf-8")

    cache_path = out_dir / CACHE_FILE_NAME
    cache = json.loads(cache_path.read_text(encoding="utf-8")) if cache_path.exists() else {}

    manifest_packs = []
    for pack in config["packs"]:
        pack_id = pack["id"]
        pack_dir = out_dir / pack_id
        selected = not args.only or pack_id in args.only
        print(f"[{pack_id}] {pack['name']}")

        built_fonts = []
        for font in pack["fonts"]:
            target = pack_dir / font["output"]
            source = source_dir / font["source"]
            cache_key = f"{pack_id}/{font['output']}"
            recipe = json.dumps({"source": font["source"], "axes": font.get("axes")}, sort_keys=True)

            if not selected:
                if not target.exists():
                    print(f"    {font['output']} not built yet; skipping pack")
                    built_fonts = []
                    break
                print(f"    keeping previous build of {font['output']}")
            elif source.exists():
                source_hash = sha256_file(source)
                cached = cache.get(cache_key, {})
                current = (
                    target.exists()
                    and cached.get("sourceSha256") == source_hash
                    and cached.get("recipe") == recipe
                    and cached.get("outputSha256") == sha256_file(target)
                )
                if args.force or not current:
                    print(f"    building {font['output']} from {font['source']} ...", flush=True)
                    build_font(source, target, font.get("axes"))
                    cache[cache_key] = {
                        "sourceSha256": source_hash,
                        "recipe": recipe,
                        "outputSha256": sha256_file(target),
                    }
                    cache_path.write_text(json.dumps(cache, indent=2), encoding="utf-8")
                else:
                    print(f"    {font['output']} is current")
            elif not target.exists():
                print(f"    missing source {source} and no previous build; skipping pack")
                built_fonts = []
                break
            else:
                print(f"    source missing, keeping previous build of {font['output']}")
            built_fonts.append((font, target))

        if not built_fonts:
            continue

        ofl = write_ofl(pack_dir, [target for _, target in built_fonts], ofl_body)
        files = []
        for font, target in built_fonts:
            files.append({
                "path": f"{pack_id}/{target.name}",
                "kind": "font",
                "family": font["family"],
                "size": target.stat().st_size,
                "sha256": sha256_file(target),
            })
        files.append({
            "path": f"{pack_id}/OFL.txt",
            "kind": "license",
            "size": ofl.stat().st_size,
            "sha256": sha256_file(ofl),
        })

        manifest_packs.append({
            "id": pack_id,
            "name": pack["name"],
            "nativeName": pack["nativeName"],
            "script": pack.get("script", ""),
            "version": config["packVersion"],
            "rtl": bool(pack.get("rtl")),
            "complexShaping": bool(pack.get("complexShaping")),
            "license": "OFL-1.1",
            "sizeBytes": sum(item["size"] for item in files),
            "files": files,
        })

    manifest = {
        "schemaVersion": config["schemaVersion"],
        "packVersion": config["packVersion"],
        "updatedAt": dt.date.today().isoformat(),
        "baseUrl": config["baseUrl"],
        "packs": manifest_packs,
    }
    manifest_text = json.dumps(manifest, indent=2, ensure_ascii=False) + "\n"
    (out_dir / "languages.json").write_text(manifest_text, encoding="utf-8", newline="\n")
    print(f"Wrote {out_dir / 'languages.json'} ({len(manifest_packs)} packs)")
    if not args.no_catalog:
        CATALOG_COPY.write_text(manifest_text, encoding="utf-8", newline="\n")
        print(f"Wrote {CATALOG_COPY}")

    if args.install:
        install_root = shared_languages_dir()
        for pack in manifest_packs:
            pack_dir = install_root / pack["id"]
            pack_dir.mkdir(parents=True, exist_ok=True)
            fonts = []
            for item in pack["files"]:
                source = out_dir / item["path"]
                shutil.copy2(source, pack_dir / source.name)
                if item["kind"] == "font":
                    fonts.append({"file": source.name, "family": item["family"]})
            installed = {key: pack[key] for key in ("id", "name", "nativeName", "script", "version", "rtl", "complexShaping", "license")}
            installed.update({
                "schemaVersion": 1,
                "fonts": fonts,
                "installedAt": dt.datetime.now(dt.timezone.utc).isoformat(),
                "installedBy": "build-language-packs.py",
            })
            (pack_dir / "pack.json").write_text(json.dumps(installed, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
            print(f"Installed {pack['id']} -> {pack_dir}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
