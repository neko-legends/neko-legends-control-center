# Neko Legends Control Center Agent Guide

This app can expose a local file-based control channel for AI agents. It is off by default to keep normal startup light.

## Enable Agent Control

Open the Control Center settings and turn on:

- `Agent control`: enables command polling for the current app session.
- `Agent auto-on`: starts Agent control automatically when the Control Center opens.

The Control Center must be running for commands to be processed.

## Folders

When Agent control is enabled, the app creates:

```text
%APPDATA%\com.nekolegends.controlcenter\agent-control\
```

Important files and folders:

- `inbox\`: write command `.json` files here.
- `outbox\`: read `*.result.json` response files here.
- `history\`: processed command files are moved here.
- `state.json`: latest app/catalog state snapshot.

Control Center also writes `capability-provider-catalog.v1.json` atomically beside the existing AppData control surfaces. It is a credential-free discovery snapshot, not a command inbox and not a replacement for the files above. In particular, `apiConfigured` only reflects explicit registry configuration; `apiReachable` requires a real health check and remains false when no check occurred.

Use unique command filenames. For safer writes, create a `.tmp` file first, then rename it to `.json`.

## Commands

### Status

```json
{
  "id": "status-001",
  "action": "status"
}
```

### Scan GitHub Releases

```json
{
  "id": "scan-001",
  "action": "scan"
}
```

### Download An App

```json
{
  "id": "download-cutscene-001",
  "action": "download",
  "appId": "cutscene-converter",
  "packagePreference": "portable"
}
```

### Update An App

```json
{
  "id": "update-cutscene-001",
  "action": "update",
  "appId": "cutscene-converter",
  "packagePreference": "portable"
}
```

### Launch An App

```json
{
  "id": "launch-cutscene-001",
  "action": "launch",
  "appId": "cutscene-converter"
}
```

### Open An App Folder

```json
{
  "id": "folder-cutscene-001",
  "action": "openFolder",
  "appId": "cutscene-converter"
}
```

## Response Files

For `inbox\download-cutscene-001.json`, the app writes:

```text
outbox\download-cutscene-001.result.json
```

Response shape:

```json
{
  "id": "download-cutscene-001",
  "action": "download",
  "appId": "cutscene-converter",
  "ok": true,
  "message": "OK",
  "processedAt": "2026-06-10T00:00:00Z",
  "data": {}
}
```

If `ok` is `false`, read `message` for the error.

## App IDs

App IDs come from `catalog/tools.json`. Keep them stable; agents should use IDs, not display names.
