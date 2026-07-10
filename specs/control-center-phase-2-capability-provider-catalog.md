# Control Center Phase 2 Capability Provider Catalog

Status: implementation spec

## Scope

Control Center writes `capability-provider-catalog.v1.json` in its existing Tauri AppData directory. The file is an additive, credential-free discovery snapshot. It does not replace `tools.json`, `tools-catalog.json`, `apps.json`, `agent-api-registry.json`, or the Agent control inbox/outbox/history protocol.

## Inputs

- The effective validated app catalog selected by the existing Remote catalog setting.
- The merged saved app state after existing install detection and normalization.
- The merged Agent API registry, including built-in entries and saved overrides.
- A static list of the file-command actions already accepted by `AgentControlAction`.
- Static descriptors for Control Center's own currently implemented `app.*` capabilities.

Hosted catalog data remains metadata only. It cannot add commands, capabilities, invocation arguments, or executable behavior.

## Truthful State

Each merged app entry reports these states separately:

- `cataloged`: the app is present in the effective validated tool catalog.
- `installed`: an existing executable or package artifact was found by current install-state logic.
- `apiConfigured`: the Agent API registry entry is explicitly enabled.
- `apiReachable`: always `false` in this phase because Control Center performs no health probe.
- `providerReady`: `installed && apiConfigured && apiReachable`, and therefore remains false until verified reachability exists.

A configured port, URL, OpenAPI URL, `lastSeen`, `busy`, or active job ID is retained as registry metadata but never treated as proof of health. No endpoint is probed as part of catalog generation.

## Output And Lifecycle

- Serialize to a same-directory temporary file and atomically replace the destination.
- Generate on startup and after app/catalog/settings/Agent API changes that pass through existing state-write paths.
- Expose a read-only Tauri debug command that regenerates and returns the same document.
- Never include bearer tokens, API keys, cookies, environment secrets, or provider-local credentials.
- Never emit absolute registry or Agent control paths. `sources` uses opaque `sourceId` values and well-known `relativeName` values only.
- Keep deterministic ordering: apps by ID, Agent API metadata merged by app ID, and static capability/action order.

## Canonical Shape

The emitted and eva-core-consumed v1 shape is one document with `schemaVersion`, `catalogKind`, `generatedAt`, `provider`, `sources`, `apps[]`, `fileCommands[]`, and `capabilities[]`. `applications` and the aliases `reachable`/`ready` are not part of this contract.

`sources.agentControl` reports emitted `configured`, `reachable`, and `available` as false because Control Center does not know eva-core's trusted mount. Eva-core may derive its own configured/available adapter state only from its separately configured trusted root while preserving emitted reachability.

## Control Center Capabilities

Advertise only implemented behavior:

- `app.catalog.list`
- `app.status.read`
- `app.release.scan`
- `app.install`
- `app.update`
- `app.launch`
- `app.folder.open`
- `app.agent-api-launch`

Descriptors declare actual available transports, side effects, risk, no remote progress stream, and no provider-side cancellation. File-command action names remain exactly `status`, `scan`, `download`, `update`, `launch`, and `openFolder`; `app.agent-api-launch` is Tauri-only.

## Verification

- `tests/fixtures/capability-provider-catalog.v1.json` is the canonical serialized fixture consumed by both Control Center Rust tests and eva-core adapter tests.
- Rust tests validate the five independent state flags, union merging, static capability/action IDs, credential-free serialization, fixture shape, and atomic replacement.
- `cargo test`, `cargo check`, and the existing frontend build must pass where the host environment permits.
