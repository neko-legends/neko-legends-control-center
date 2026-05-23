# Neko Legends Control Center

A compact desktop launcher and update hub for Neko Legends / ForPublic standalone apps.

## Features

- Giant app-icon launcher grid
- Neko Tron black-and-orange theme
- Per-app executable path selection
- Local app launching
- Manual GitHub release scanning
- Theme picker and compact-label mode

## Development

```powershell
npm install
npm run dev
```

## Desktop Build

```powershell
npm run build:portable
```

The portable executable is written to `src-tauri\target-portable\release\neko-legends-control-center-portable.exe`.
A distribution-ready folder is also written to `release\NekoLegendsControlCenter` with an `apps` folder beside the launcher.

The app is built with React, Vite, and Tauri.
