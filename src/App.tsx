import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import {
  AppWindow,
  Download,
  ExternalLink,
  FolderOpen,
  GripVertical,
  Info,
  Loader2,
  Pencil,
  Play,
  Plus,
  RefreshCw,
  Settings2,
  Trash2,
} from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'

type ThemeId = 'neko-tron' | 'pearl-white' | 'abyss-teal' | 'ember' | 'mosswood' | 'rose-noir'
type PackagePreference = 'portable' | 'installer'
type ToolStatus = 'available' | 'comingSoon'

type LauncherApp = {
  id: string
  name: string
  repo: string
  description: string
  accent: string
  icon: string
  category: string
  executablePath: string | null
  installedVersion: string | null
  selectedVersion: string | null
  installedVersions: InstalledVersion[]
  latestVersion: string | null
  releaseUrl: string | null
  releaseCheckedAt: string | null
  releaseNotes: string | null
  releaseOptions: ReleaseOption[]
  packagePreference: PackagePreference
  packagePath: string | null
  demoUrl: string | null
  status: ToolStatus
  visible: boolean
}

type InstalledVersion = {
  version: string
  executablePath: string | null
  packagePath: string | null
  installedAt: string
}

type ReleaseOption = {
  tagName: string
  htmlUrl: string
}

type AppSettings = {
  theme: ThemeId
  compactLabels: boolean
  useRemoteCatalog: boolean
  agentControlAutoStart: boolean
  categories: string[]
}

type ControlCenterState = {
  settings: AppSettings
  apps: LauncherApp[]
  buildVersion: string
  dataDir: string
}

type DownloadResult = {
  apps: LauncherApp[]
  filePath: string
  installFolder: string
}

type LaunchResult = {
  apps: LauncherApp[]
  relaunched: boolean
}

type ControlCenterUpdate = {
  currentVersion: string
  latestVersion: string | null
  releaseUrl: string | null
  releaseNotes: string | null
  checkedAt: string
  updateAvailable: boolean
}

type AgentControlInfo = {
  rootDir: string
  inboxDir: string
  outboxDir: string
  historyDir: string
  statePath: string
}

type AgentControlPollResult = {
  processedCount: number
  apps: LauncherApp[]
  info: AgentControlInfo
}

type AgentApiRegistryEntry = {
  appId: string
  appName: string
  defaultPort: number
  bindAddress: string
  port: number
  enabled: boolean
  url: string
  openapiUrl: string
  busy: boolean
  activeJobId: string | null
  lastSeen: string | null
  note: string | null
}

type AgentApiPortConflict = {
  port: number
  appIds: string[]
  appNames: string[]
}

type AgentApiDashboard = {
  registryPath: string
  updatedAt: string
  apps: AgentApiRegistryEntry[]
  conflicts: AgentApiPortConflict[]
  nextAvailablePort: number
}

type ReleaseScanProgress = {
  current: number
  total: number
  appId: string
  appName: string
  status: 'checking' | 'checked'
}

type Theme = {
  id: ThemeId
  name: string
  colors: string[]
}

const themes: Theme[] = [
  { id: 'neko-tron', name: 'Neko Tron', colors: ['#050505', '#17100a', '#ff6a00'] },
  { id: 'pearl-white', name: 'Pearl', colors: ['#ffffff', '#f0f0f6', '#635bdc'] },
  { id: 'abyss-teal', name: 'Abyss', colors: ['#161d1f', '#1f2a2c', '#2ec4b6'] },
  { id: 'ember', name: 'Ember', colors: ['#241e18', '#352c24', '#f2a65a'] },
  { id: 'mosswood', name: 'Moss', colors: ['#18201b', '#233028', '#4cc38a'] },
  { id: 'rose-noir', name: 'Rose', colors: ['#100b12', '#241627', '#ff6f9e'] },
]

const underDevelopmentCategory = 'Under Development'
const releasedToolsCategory = 'Released Tools'
const legacyReleasedWorkStuffCategory = 'Released Work Stuff'
const funStuffCategory = 'Fun Stuff'
const defaultCategories = [releasedToolsCategory, funStuffCategory, underDevelopmentCategory]
const githubOwner = 'neko-legends'
const bootReleaseScanMaxAgeMs = 12 * 60 * 60 * 1000

const fallbackState: ControlCenterState = {
  settings: { theme: 'neko-tron', compactLabels: false, useRemoteCatalog: true, agentControlAutoStart: false, categories: defaultCategories },
  buildVersion: 'dev',
  dataDir: '',
  apps: [
    app('batchlapse', 'BatchLapse', 'BatchLapse', 'Batch video timelapse exporter for MP4, WebM, and GitHub-friendly GIFs.', '#5b8def', 'BL', releasedToolsCategory),
    app('cutscene-converter', 'Cutscene Converter', 'CutsceneConverter', 'Godot-friendly cutscene video converter for MP4, WebM, and OGV.', '#f06f48', 'CC', releasedToolsCategory),
    app('depth-map-ai-generator', 'DepthMap AI', 'DepthMapAIGenerator', 'Batch depth-map and WebP generator for local AI image workflows.', '#43b883', 'DM', 'Under Development', null, 'comingSoon'),
    app('image-to-ascii-3d', 'ASCII 3D', 'ImageToASCII3D', 'Image-to-ASCII converter with optional depth-map driven 3D parallax exports.', '#f0a848', 'A3', 'Under Development', null, 'comingSoon'),
    app('image-to-3d', 'Image to 3D', 'ImageTo3D', 'Local image-to-3D workflow for mesh, texture, and 3D asset generation.', '#8c65df', 'I3', 'Under Development'),
    app('multi-angle-edit', 'Multi-Angle Edit', 'multi-angle-edit', 'Local multi-angle image editor: re-render a photo from a new camera angle with Qwen-Image-Edit + the Multiple-Angles LoRA on your own GPU.', '#b14bff', 'MA', releasedToolsCategory),
    app('image-to-splat', 'ImageToSplat', 'ImageToSplat', 'Local TripoSplat workflow for turning a single image into Gaussian splat and point-cloud 3D exports.', '#55c7f7', 'IS', 'Under Development', null, 'comingSoon'),
    app('splatscape', 'SplatScape', 'SplatScape', 'Portable FPS-style explorer for 3D Gaussian splat scenes with WASD and mouse-look navigation.', '#7adfbb', 'SS', 'Under Development', null, 'comingSoon'),
    app('painterly-clouds-3d', 'Painterly Clouds 3D', 'painterly-clouds-3d', 'Painterly Three.js cloud scene for stylized skyboxes, wallpapers, and motion art.', '#7fb7ff', 'PC', 'Under Development', null, 'comingSoon'),
    app('markrush', 'MarkRush', 'MarkRush', 'Fast local Markdown viewer/editor built for huge files and folders.', '#e05d7b', 'MR', releasedToolsCategory),
    app('opensplit', 'OpenSplit', 'OpenSplit', 'Multi-pane terminal harness for AI coding agents, shells, and SSH sessions.', '#4fb6d8', 'OS', releasedToolsCategory),
    app('seamless-image-edit', 'Seamless Image Edit', 'SeamlessImageEdit', 'Local image tiling and seamless texture prep for game art workflows.', '#d889ff', 'SI', releasedToolsCategory),
    app('sprite-atlas-packer', 'Sprite Atlas Packer', 'sprite-atlas-packer', 'Turn loose sprite frames into deterministic TexturePacker-compatible PNG/WebP atlases.', '#39a7ff', 'SA', releasedToolsCategory),
    app('venice-media-local', 'Venice Media', 'VeniceMediaLocal', 'Local Venice API media workspace for images, video, music, voice, and cleanup.', '#34c6a3', 'VM', releasedToolsCategory),
    app('purpleplanet', 'PurplePlanet', 'PurplePlanet', 'Luminous Three.js planet motion art for live wallpapers and screensavers.', '#8c65df', 'PP', 'Fun Stuff', 'https://nekolegends.com/res/projects/purplePlanet/'),
    app('stargaze', 'StarGaze', 'StarGaze', 'Glittering Three.js starfield wallpaper and screensaver with tunable motion.', '#6b7cff', 'SG', 'Fun Stuff', 'https://nekolegends.com/res/projects/starGaze/'),
  ],
}

function app(id: string, name: string, repo: string, description: string, accent: string, icon: string, category: string, demoUrl: string | null = null, status: ToolStatus = 'available'): LauncherApp {
  const isUnderDevelopment = isUnderDevelopmentCategory(category)
  const effectiveStatus = status === 'comingSoon' || isUnderDevelopment ? 'comingSoon' : status
  const effectiveCategory = effectiveStatus === 'comingSoon' ? underDevelopmentCategory : category

  return {
    id,
    name,
    repo,
    description,
    accent,
    icon,
    category: effectiveCategory,
    executablePath: null,
    installedVersion: null,
    selectedVersion: null,
    installedVersions: [],
    latestVersion: null,
    releaseUrl: null,
    releaseCheckedAt: null,
    releaseNotes: null,
    releaseOptions: [],
    packagePreference: 'portable',
    packagePath: null,
    demoUrl,
    status: effectiveStatus,
    visible: true,
  }
}

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauriRuntime()) {
    throw new Error('Tauri runtime is not active')
  }
  return invoke<T>(command, args)
}

function classNames(...items: Array<string | false | null | undefined>): string {
  return items.filter(Boolean).join(' ')
}

function formatCheckedAt(value: string | null): string {
  if (!value) return 'Not checked'
  const parsed = new Date(value)
  if (Number.isNaN(parsed.getTime())) return 'Checked'
  return parsed.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })
}

function normalizePort(value: string): number | null {
  const port = Number(value)
  return Number.isInteger(port) && port >= 1 && port <= 65535 ? port : null
}

function conflictForPort(dashboard: AgentApiDashboard | null, port: number): AgentApiPortConflict | null {
  return dashboard?.conflicts.find((conflict) => conflict.port === port) ?? null
}

function supportsHeadlessAgentLaunch(appId: string): boolean {
  return new Set([
    'asset-vault',
    'batchlapse',
    'cutscene-converter',
    'depth-map-ai-generator',
    'image-to-3d',
    'image-to-splat',
    'multi-angle-edit',
    'sprite-atlas-packer',
  ]).has(appId)
}

function versionNumberParts(version: string | null): number[] {
  if (!version) return []
  return Array.from(version.matchAll(/\d+/g), (match) => Number(match[0]))
}

function sameVersionNumbers(left: string | null, right: string | null): boolean {
  const leftParts = versionNumberParts(left)
  const rightParts = versionNumberParts(right)
  if (leftParts.length === 0 || rightParts.length === 0 || leftParts.length !== rightParts.length) {
    return false
  }
  return leftParts.every((part, index) => part === rightParts[index])
}

function versionStatus(appInfo: LauncherApp): 'ready' | 'update' | 'unknown' {
  if (!appInfo.latestVersion) return 'unknown'
  if (!appInfo.installedVersion) return 'ready'
  return sameVersionNumbers(appInfo.installedVersion, appInfo.latestVersion) ? 'ready' : 'update'
}

function appArtifactPath(appInfo: LauncherApp): string | null {
  return appInfo.demoUrl ? appInfo.packagePath : appInfo.executablePath
}

function isAppDownloaded(appInfo: LauncherApp): boolean {
  return Boolean(appArtifactPath(appInfo))
}

function installedVersionFor(appInfo: LauncherApp, version: string | null): InstalledVersion | null {
  if (!version) return null
  return (appInfo.installedVersions ?? []).find((installed) => installed.version === version) ?? null
}

function isVersionInstalled(appInfo: LauncherApp, version: string | null): boolean {
  return Boolean(installedVersionFor(appInfo, version))
}

function selectedLaunchPath(appInfo: LauncherApp, version: string | null): string | null {
  const installedVersion = installedVersionFor(appInfo, version)
  if (installedVersion) {
    return appInfo.demoUrl ? installedVersion.packagePath : installedVersion.executablePath
  }
  if (!version || version === appInfo.installedVersion) return appArtifactPath(appInfo)
  return null
}

function hasKnownRelease(appInfo: LauncherApp): boolean {
  return Boolean(appInfo.latestVersion || appInfo.releaseOptions.length > 0)
}

function isComingSoon(appInfo: LauncherApp): boolean {
  return appInfo.status === 'comingSoon' || isUnderDevelopmentCategory(appInfo.category)
}

function isUnderDevelopmentCategory(category: string): boolean {
  return category.trim().toLowerCase() === underDevelopmentCategory.toLowerCase()
}

function needsBootReleaseScan(appInfo: LauncherApp, now = Date.now()): boolean {
  if (!appInfo.releaseCheckedAt) return true
  const checkedAt = new Date(appInfo.releaseCheckedAt).getTime()
  if (Number.isNaN(checkedAt)) return true
  return now - checkedAt > bootReleaseScanMaxAgeMs
}

function hasNoPublicRelease(appInfo: LauncherApp): boolean {
  return appInfo.releaseNotes === 'No public releases found yet.'
}

function newlyReleasedIds(beforeApps: LauncherApp[], afterApps: LauncherApp[]): string[] {
  const beforeById = new Map(beforeApps.map((appInfo) => [appInfo.id, appInfo]))
  return afterApps
    .filter((appInfo) => {
      const before = beforeById.get(appInfo.id)
      return before && isComingSoon(before) && !isComingSoon(appInfo) && hasKnownRelease(appInfo)
    })
    .map((appInfo) => appInfo.id)
}

function installStatus(appInfo: LauncherApp): 'installed' | 'missing' {
  return isAppDownloaded(appInfo) ? 'installed' : 'missing'
}

type AppDisplayStatus = 'checking' | 'scanning' | 'coming-soon' | 'missing' | 'installed' | 'update' | 'downloading' | 'failed'

function fileName(path: string | null): string {
  if (!path) return ''
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path
}

function previousInstalledVersion(appInfo: LauncherApp): InstalledVersion | null {
  return appInfo.installedVersions.find((installed) => installed.version !== appInfo.installedVersion) ?? null
}

function categoryLabel(category: string): string {
  let value = category.trim()
  while (true) {
    const nextValue = value
      .replace(/^\s*-=\s*/, '')
      .replace(/\s*=-\s*$/, '')
      .trim()
    if (nextValue === value) break
    value = nextValue
  }
  value = value.replace(/\s+/g, ' ')
  const key = value.toLocaleLowerCase()
  if (key === releasedToolsCategory.toLocaleLowerCase()) return releasedToolsCategory
  if (key === legacyReleasedWorkStuffCategory.toLocaleLowerCase()) return releasedToolsCategory
  if (key === funStuffCategory.toLocaleLowerCase()) return funStuffCategory
  if (key === underDevelopmentCategory.toLocaleLowerCase()) return underDevelopmentCategory
  return value || releasedToolsCategory
}

function normalizeCategories(categories: string[]): string[] {
  const normalized: string[] = []
  for (const category of categories) {
    const value = categoryLabel(category)
    if (!normalized.includes(value)) {
      normalized.push(value)
    }
  }
  if (normalized.length === 0) return defaultCategories

  const orderedDefaults = defaultCategories.filter((category) => normalized.includes(category))
  const customCategories = normalized.filter((category) => !defaultCategories.includes(category))
  return [...orderedDefaults, ...customCategories]
}

function orderedCategories(apps: LauncherApp[], categories: string[]): string[] {
  const nextCategories = normalizeCategories(categories)
  for (const appInfo of apps) {
    const category = categoryLabel(appInfo.category)
    if (!nextCategories.includes(category)) {
      nextCategories.push(category)
    }
  }
  return nextCategories
}

type GridItem =
  | { kind: 'category'; id: string; label: string }
  | { kind: 'app'; app: LauncherApp }

function gridItems(apps: LauncherApp[], categories: string[]): GridItem[] {
  const items: GridItem[] = []

  for (const category of categories) {
    items.push({ kind: 'category', id: `category-${category}`, label: category })
    for (const appInfo of apps.filter((candidate) => categoryLabel(candidate.category) === category)) {
      items.push({ kind: 'app', app: appInfo })
    }
  }

  return items
}

function moveAppToApp(apps: LauncherApp[], fromId: string, target: LauncherApp): LauncherApp[] {
  return moveAppToAppSlot(apps, fromId, target)
}

function moveAppToAppSlot(apps: LauncherApp[], fromId: string, target: LauncherApp): LauncherApp[] {
  const toId = target.id
  if (fromId === toId) return apps
  const fromIndex = apps.findIndex((candidate) => candidate.id === fromId)
  const toIndex = apps.findIndex((candidate) => candidate.id === toId)
  if (fromIndex < 0 || toIndex < 0) return apps

  const nextApps = [...apps]
  const [moved] = nextApps.splice(fromIndex, 1)
  const insertIndex = Math.min(toIndex, nextApps.length)
  nextApps.splice(insertIndex, 0, { ...moved, category: categoryLabel(target.category) })
  return nextApps
}

function sameAppLayout(left: LauncherApp[], right: LauncherApp[]): boolean {
  if (left.length !== right.length) return false
  return left.every((appInfo, index) => appInfo.id === right[index]?.id && categoryLabel(appInfo.category) === categoryLabel(right[index]?.category ?? ''))
}

function moveAppToCategory(apps: LauncherApp[], appId: string, category: string): LauncherApp[] {
  const fromIndex = apps.findIndex((candidate) => candidate.id === appId)
  if (fromIndex < 0) return apps

  const nextApps = [...apps]
  const [moved] = nextApps.splice(fromIndex, 1)
  const normalizedCategory = categoryLabel(category)
  let lastCategoryIndex = -1
  for (let index = nextApps.length - 1; index >= 0; index -= 1) {
    if (categoryLabel(nextApps[index].category) === normalizedCategory) {
      lastCategoryIndex = index
      break
    }
  }
  nextApps.splice(lastCategoryIndex + 1, 0, { ...moved, category: normalizedCategory })
  return nextApps
}

function moveCategory(categories: string[], from: string, to: string): string[] {
  if (from === to) return categories
  const nextCategories = normalizeCategories(categories)
  const fromIndex = nextCategories.indexOf(from)
  const toIndex = nextCategories.indexOf(to)
  if (fromIndex < 0 || toIndex < 0) return categories
  const [moved] = nextCategories.splice(fromIndex, 1)
  const nextToIndex = nextCategories.indexOf(to)
  nextCategories.splice(nextToIndex, 0, moved)
  return nextCategories
}

type DragItem =
  | { kind: 'app'; id: string }
  | { kind: 'category'; label: string }

type PointerDrag = {
  item: DragItem
  pointerId: number
  startX: number
  startY: number
  offsetX: number
  offsetY: number
  width: number
  height: number
  active: boolean
}

type DragPointerEvent = {
  pointerId: number
  clientX: number
  clientY: number
  preventDefault: () => void
  stopPropagation: () => void
}

type DragListeners = {
  move: (event: PointerEvent) => void
  up: (event: PointerEvent) => void
  cancel: (event: PointerEvent) => void
}

type DragGhost = {
  item: DragItem
  x: number
  y: number
  width: number
  height: number
  icon: string
  title: string
  subtitle: string
  accent: string
}

type AppLayoutSlot = {
  appId: string
  category: string
  rect: DOMRect
  centerX: number
  centerY: number
}

type AppContextMenu = {
  appId: string
  x: number
  y: number
}

type PendingAppPreview = {
  appId: string
  clientX: number
  clientY: number
  directX: number
  directY: number
}

const dragDataType = 'application/x-neko-layout-item'
const layoutAnimationDurationMs = 190
const layoutAnimationMaxLockMs = 1000

export default function App() {
  const [state, setState] = useState<ControlCenterState>(fallbackState)
  const [busy, setBusy] = useState(false)
  const [notice, setNotice] = useState('Ready')
  const [selectedId, setSelectedId] = useState<string>('venice-media-local')
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [draggedItem, setDraggedItem] = useState<DragItem | null>(null)
  const [dragGhost, setDragGhost] = useState<DragGhost | null>(null)
  const [selectedReleaseTags, setSelectedReleaseTags] = useState<Record<string, string>>({})
  const [runningAppIds, setRunningAppIds] = useState<string[]>([])
  const [activeDownloads, setActiveDownloads] = useState<string[]>([])
  const [failedDownloads, setFailedDownloads] = useState<Record<string, boolean>>({})
  const [contextMenu, setContextMenu] = useState<AppContextMenu | null>(null)
  const [controlCenterUpdate, setControlCenterUpdate] = useState<ControlCenterUpdate | null>(null)
  const [controlCenterUpdating, setControlCenterUpdating] = useState(false)
  const [agentControlEnabled, setAgentControlEnabled] = useState(false)
  const [agentControlInfo, setAgentControlInfo] = useState<AgentControlInfo | null>(null)
  const [agentApiDashboard, setAgentApiDashboard] = useState<AgentApiDashboard | null>(null)
  const [agentApiPortEdits, setAgentApiPortEdits] = useState<Record<string, string>>({})
  const [scanProgress, setScanProgress] = useState<ReleaseScanProgress | null>(null)
  const [newReleaseIds, setNewReleaseIds] = useState<string[]>([])
  const draggedItemRef = useRef<DragItem | null>(null)
  const pointerDragRef = useRef<PointerDrag | null>(null)
  const dragListenersRef = useRef<DragListeners | null>(null)
  const suppressClickRef = useRef(false)
  const gridRef = useRef<HTMLElement | null>(null)
  const appsRef = useRef<LauncherApp[]>(fallbackState.apps)
  const categoriesRef = useRef<string[]>(defaultCategories)
  const layoutTweenIdRef = useRef(0)
  const layoutAnimationLockedRef = useRef(false)
  const layoutAnimationRunIdRef = useRef(0)
  const layoutAnimationTimerRef = useRef<number | null>(null)
  const pendingAppPreviewRef = useRef<PendingAppPreview | null>(null)
  const didApplyAgentAutoStartRef = useRef(false)

  const visibleApps = useMemo(() => state.apps.filter((candidate) => candidate.visible), [state.apps])
  const selectedApp = useMemo(
    () => visibleApps.find((candidate) => candidate.id === selectedId) ?? visibleApps[0] ?? state.apps[0],
    [selectedId, state.apps, visibleApps],
  )

  const configuredCount = visibleApps.filter((candidate) => isAppDownloaded(candidate)).length
  const updateCount = visibleApps.filter((candidate) => versionStatus(candidate) === 'update').length
  const missingCount = visibleApps.filter((candidate) => hasKnownRelease(candidate) && !isAppDownloaded(candidate)).length
  const hiddenCount = state.apps.length - visibleApps.length
  const layoutCategories = useMemo(() => orderedCategories(state.apps, state.settings.categories), [state.apps, state.settings.categories])
  const visibleGridItems = useMemo(() => gridItems(visibleApps, layoutCategories), [layoutCategories, visibleApps])
  const contextMenuApp = useMemo(
    () => contextMenu ? state.apps.find((candidate) => candidate.id === contextMenu.appId) ?? null : null,
    [contextMenu, state.apps],
  )
  const agentApiConflictCount = agentApiDashboard?.conflicts.length ?? 0
  const selectedAppUpdateAvailable = selectedApp ? isAppDownloaded(selectedApp) && versionStatus(selectedApp) === 'update' : false
  const selectedPreviousVersion = selectedApp ? previousInstalledVersion(selectedApp) : null

  appsRef.current = state.apps
  categoriesRef.current = layoutCategories

  function selectedVersionFor(appInfo: LauncherApp): string | null {
    return selectedReleaseTags[appInfo.id]
      ?? appInfo.selectedVersion
      ?? appInfo.latestVersion
      ?? appInfo.installedVersion
      ?? appInfo.releaseOptions[0]?.tagName
      ?? null
  }

  function rememberNewReleases(beforeApps: LauncherApp[], afterApps: LauncherApp[]): string[] {
    const ids = newlyReleasedIds(beforeApps, afterApps)
    if (ids.length > 0) {
      setNewReleaseIds((current) => Array.from(new Set([...current, ...ids])))
    }
    return ids
  }

  function selectedVersionInstalled(appInfo: LauncherApp): boolean {
    return isVersionInstalled(appInfo, selectedVersionFor(appInfo))
  }

  function canLaunchSelectedVersion(appInfo: LauncherApp): boolean {
    return Boolean(selectedLaunchPath(appInfo, selectedVersionFor(appInfo)))
  }

  function isAppRunning(appInfo: LauncherApp): boolean {
    return runningAppIds.includes(appInfo.id)
  }

  function displayStatus(appInfo: LauncherApp): AppDisplayStatus {
    if (scanProgress?.appId === appInfo.id && scanProgress.status === 'checking') return 'scanning'
    if (activeDownloads.includes(appInfo.id)) return 'downloading'
    if (failedDownloads[appInfo.id]) return 'failed'
    if (!isAppDownloaded(appInfo) && isComingSoon(appInfo)) return 'coming-soon'
    if (!isAppDownloaded(appInfo) && !hasKnownRelease(appInfo) && !appInfo.releaseCheckedAt) return 'checking'
    if (!isAppDownloaded(appInfo) && !hasKnownRelease(appInfo) && !hasNoPublicRelease(appInfo)) return 'failed'
    if (!isAppDownloaded(appInfo)) return hasKnownRelease(appInfo) ? 'missing' : 'coming-soon'
    if (versionStatus(appInfo) === 'update') return 'update'
    return 'installed'
  }

  function displayStatusLabel(status: AppDisplayStatus): string {
    if (status === 'downloading') return 'Downloading'
    if (status === 'failed') return 'Failed'
    if (status === 'update') return 'Update Ready'
    if (status === 'installed') return 'Installed'
    if (status === 'scanning') return 'Scanning'
    if (status === 'checking') return 'Checking'
    if (status === 'coming-soon') return 'Coming Soon'
    return 'Missing'
  }

  useEffect(() => {
    void loadState()
    void checkControlCenterUpdate(false)
    void refreshAgentApiDashboard()
  }, [])

  useEffect(() => {
    if (!isTauriRuntime()) return

    let cancelled = false
    let unlisten: (() => void) | null = null
    void listen<ReleaseScanProgress>('release-scan-progress', (event) => {
      if (cancelled) return
      setScanProgress(event.payload)
      if (event.payload.status === 'checking') {
        setNotice(`Scanning ${event.payload.current}/${event.payload.total}: ${event.payload.appName}`)
      }
    }).then((cleanup) => {
      if (cancelled) {
        cleanup()
      } else {
        unlisten = cleanup
      }
    }).catch((error) => {
      if (!cancelled) setNotice(error instanceof Error ? error.message : String(error))
    })

    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [])

  useEffect(() => {
    if (!agentControlEnabled || !isTauriRuntime()) return

    let cancelled = false
    let polling = false

    async function pollAgentCommands() {
      if (polling) return
      polling = true
      try {
        const result = await call<AgentControlPollResult>('process_agent_control_commands')
        if (cancelled) return
        setAgentControlInfo(result.info)
        if (result.processedCount > 0) {
          setState((current) => ({ ...current, apps: result.apps }))
          setNotice(`Agent control processed ${result.processedCount} command${result.processedCount === 1 ? '' : 's'}`)
        }
      } catch (error) {
        if (!cancelled) {
          setNotice(error instanceof Error ? error.message : String(error))
        }
      } finally {
        polling = false
      }
    }

    void call<AgentControlInfo>('get_agent_control_info').then((info) => {
      if (!cancelled) setAgentControlInfo(info)
    }).catch((error) => {
      if (!cancelled) setNotice(error instanceof Error ? error.message : String(error))
    })
    void pollAgentCommands()
    const interval = window.setInterval(() => void pollAgentCommands(), 2000)
    return () => {
      cancelled = true
      window.clearInterval(interval)
    }
  }, [agentControlEnabled])

  useEffect(() => {
    if (!contextMenu) return

    function closeMenu() {
      setContextMenu(null)
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        closeMenu()
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    window.addEventListener('resize', closeMenu)
    return () => {
      window.removeEventListener('keydown', handleKeyDown)
      window.removeEventListener('resize', closeMenu)
    }
  }, [contextMenu])

  async function loadState() {
    if (!isTauriRuntime()) {
      setNotice('Browser preview. Launching and saved paths work in the desktop runtime.')
      return
    }

    try {
      await call<LauncherApp[]>('refresh_tools_catalog').catch(() => null)
      const nextState = await call<ControlCenterState>('get_state')
      setState(nextState)
      if (!didApplyAgentAutoStartRef.current) {
        setAgentControlEnabled(nextState.settings.agentControlAutoStart)
        didApplyAgentAutoStartRef.current = true
      }
      const nextVisibleApps = nextState.apps.filter((candidate) => candidate.visible)
      setSelectedId((current) => nextVisibleApps.some((candidate) => candidate.id === current) ? current : nextVisibleApps[0]?.id ?? nextState.apps[0]?.id ?? '')
      const bootScanStartedAt = Date.now()
      if (nextState.apps.some((appInfo) => needsBootReleaseScan(appInfo, bootScanStartedAt))) {
        setNotice('Refreshing release data in staggered batches...')
        const apps = await call<LauncherApp[]>('scan_releases')
        rememberNewReleases(nextState.apps, apps)
        setState((current) => ({ ...current, apps }))
        setScanProgress(null)
      }
      setNotice('Ready')
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error))
    }
  }

  async function refreshAgentApiDashboard() {
    if (!isTauriRuntime()) return
    try {
      const dashboard = await call<AgentApiDashboard>('get_agent_api_dashboard')
      setAgentApiDashboard(dashboard)
      setAgentApiPortEdits(
        Object.fromEntries(dashboard.apps.map((entry) => [entry.appId, String(entry.port)])),
      )
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error))
    }
  }

  async function saveAgentApiPort(appId: string, value: string) {
    const port = normalizePort(value)
    if (port === null) {
      setNotice('Choose a port between 1 and 65535.')
      return
    }

    try {
      const dashboard = await call<AgentApiDashboard>('save_agent_api_port', { request: { appId, port } })
      setAgentApiDashboard(dashboard)
      setAgentApiPortEdits(
        Object.fromEntries(dashboard.apps.map((entry) => [entry.appId, String(entry.port)])),
      )
      const conflict = conflictForPort(dashboard, port)
      setNotice(conflict ? `Port ${port} is assigned to ${conflict.appNames.length} apps` : `Agent API port saved: ${port}`)
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error))
    }
  }

  async function assignNextAgentApiPort(appId: string) {
    if (!agentApiDashboard) return
    await saveAgentApiPort(appId, String(agentApiDashboard.nextAvailablePort))
  }

  async function runReleaseScan(previousApps: LauncherApp[] = appsRef.current): Promise<{ update: ControlCenterUpdate | null; promotedCount: number }> {
    setScanProgress(null)
    await call<LauncherApp[]>('refresh_tools_catalog').catch(() => null)
    const apps = await call<LauncherApp[]>('scan_releases')
    const update = await checkControlCenterUpdate(false)
    const promotedIds = rememberNewReleases(previousApps, apps)
    setState((current) => ({ ...current, apps }))
    setScanProgress(null)
    return { update, promotedCount: promotedIds.length }
  }

  async function scanForUpdates() {
    setBusy(true)
    setNotice('Scanning GitHub releases in staggered batches...')
    try {
      if (!isTauriRuntime()) {
        setNotice('Browser preview. Release scans run in the desktop runtime.')
        return
      }
      const { update, promotedCount } = await runReleaseScan()
      const releaseSummary = promotedCount > 0 ? ` - ${promotedCount} newly released` : ''
      setNotice(update?.updateAvailable ? `Release scan complete${releaseSummary} - Control Center update available` : `Release scan complete${releaseSummary}`)
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error))
    } finally {
      setScanProgress(null)
      setBusy(false)
    }
  }

  async function checkControlCenterUpdate(showNotice: boolean): Promise<ControlCenterUpdate | null> {
    if (!isTauriRuntime()) return null
    if (showNotice) setNotice('Checking Control Center release...')
    try {
      const update = await call<ControlCenterUpdate>('check_control_center_update')
      setControlCenterUpdate(update)
      if (showNotice) {
        setNotice(update.updateAvailable ? `Control Center ${update.latestVersion} is available` : 'Control Center is up to date')
      }
      return update
    } catch (error) {
      if (showNotice) {
        setNotice(error instanceof Error ? error.message : String(error))
      }
      return null
    }
  }

  async function installControlCenterUpdate() {
    if (!controlCenterUpdate) return
    setBusy(true)
    setControlCenterUpdating(true)
    setNotice(`Downloading Control Center ${controlCenterUpdate.latestVersion ?? 'update'}...`)
    try {
      await call<void>('install_control_center_update')
      setNotice('Restarting Control Center to finish the update...')
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      try {
        await call<void>('open_control_center_release', { update: controlCenterUpdate })
        setNotice(`${message} Opening the release page instead.`)
      } catch {
        window.open(controlCenterUpdate.releaseUrl ?? 'https://github.com/neko-legends/neko-legends-control-center/releases', '_blank', 'noopener')
        setNotice(message)
      }
      setControlCenterUpdating(false)
      setBusy(false)
    }
  }

  async function launchSelected() {
    if (!selectedApp) return
    if (selectedApp.demoUrl) {
      await viewDemo(selectedApp)
      return
    }
    await launchApp(selectedApp)
  }

  async function launchApp(appInfo: LauncherApp) {
    const version = selectedVersionFor(appInfo)
    if (!selectedLaunchPath(appInfo, version)) {
      setNotice(version ? `${appInfo.name} ${version} is not downloaded yet` : `${appInfo.name} is not downloaded yet`)
      return
    }
    setBusy(true)
    setNotice(`${isAppRunning(appInfo) ? 'Re-launching' : 'Launching'} ${appInfo.name}...`)
    try {
      const result = await call<LaunchResult>('launch_app', { request: { appId: appInfo.id, version } })
      setState((current) => ({ ...current, apps: result.apps }))
      setRunningAppIds((current) => current.includes(appInfo.id) ? current : [...current, appInfo.id])
      setNotice(result.relaunched ? `${appInfo.name} relaunched` : `${appInfo.name} launched`)
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(false)
    }
  }

  async function selectInstalledVersion(appInfo: LauncherApp, version: string, label: string) {
    setBusy(true)
    setNotice(`${label} ${appInfo.name} to ${version}...`)
    try {
      const apps = await call<LauncherApp[]>('select_app_version', { request: { appId: appInfo.id, version } })
      setState((current) => ({ ...current, apps }))
      setSelectedReleaseTags((current) => ({ ...current, [appInfo.id]: version }))
      setNotice(`${appInfo.name} is using ${version}`)
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(false)
    }
  }

  async function launchHeadlessAgentApi(entry: AgentApiRegistryEntry) {
    setBusy(true)
    setNotice(`Starting ${entry.appName} headless API...`)
    try {
      await call<void>('launch_agent_api_headless', { request: { appId: entry.appId } })
      setNotice(`${entry.appName} headless API starting on port ${entry.port}`)
      window.setTimeout(() => void refreshAgentApiDashboard(), 800)
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(false)
    }
  }

  async function viewDemo(appInfo: LauncherApp) {
    if (!appInfo.demoUrl) {
      setNotice(`${appInfo.name} does not have a hosted demo`)
      return
    }

    setBusy(true)
    setNotice(`Opening ${appInfo.name} demo...`)
    try {
      if (isTauriRuntime()) {
        await call<void>('open_demo_url', { request: { appId: appInfo.id } })
      } else {
        window.open(appInfo.demoUrl, '_blank', 'noopener')
      }
      setNotice(`${appInfo.name} demo opened`)
    } catch (error) {
      window.open(appInfo.demoUrl, '_blank', 'noopener')
      setNotice(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(false)
    }
  }

  async function chooseExecutable(appInfo: LauncherApp) {
    if (!isTauriRuntime()) {
      setNotice('Launch file selection is available in the desktop runtime.')
      return
    }

    let defaultPath = appInfo.executablePath ?? undefined
    if (!defaultPath) {
      try {
        defaultPath = await call<string>('get_default_install_dir')
      } catch {
        defaultPath = undefined
      }
    }

    const selected = await open({
      multiple: false,
      directory: false,
      defaultPath,
      filters: [
        { name: 'Launch files', extensions: ['exe', 'html', 'htm'] },
        { name: 'Windows executable', extensions: ['exe'] },
        { name: 'Web wallpaper', extensions: ['html', 'htm'] },
      ],
    })
    if (typeof selected !== 'string') return

    try {
      const apps = await call<LauncherApp[]>('save_executable', {
        request: { appId: appInfo.id, executablePath: selected },
      })
      setState((current) => ({ ...current, apps }))
      setNotice(`${appInfo.name} launch file saved`)
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error))
    }
  }

  async function openRelease(appInfo: LauncherApp) {
    try {
      await call<void>('open_release_url', { request: { appId: appInfo.id } })
    } catch (error) {
      window.open(appInfo.releaseUrl ?? `https://github.com/${githubOwner}/${appInfo.repo}/releases`, '_blank', 'noopener')
      setNotice(error instanceof Error ? error.message : String(error))
    }
  }

  async function openRepository(appInfo: LauncherApp) {
    try {
      await call<void>('open_repository_url', { request: { appId: appInfo.id } })
      setNotice(`${appInfo.name} info opened`)
    } catch (error) {
      window.open(`https://github.com/${githubOwner}/${appInfo.repo}`, '_blank', 'noopener')
      setNotice(error instanceof Error ? error.message : String(error))
    }
  }

  async function downloadRelease(appInfo: LauncherApp, version: string | null, manageBusy: boolean): Promise<boolean> {
    if (isComingSoon(appInfo)) {
      setNotice(`${appInfo.name} is coming soon`)
      return false
    }
    if (manageBusy) setBusy(true)
    setActiveDownloads((current) => current.includes(appInfo.id) ? current : [...current, appInfo.id])
    setFailedDownloads((current) => {
      const next = { ...current }
      delete next[appInfo.id]
      return next
    })
    setNotice(`Downloading ${appInfo.name}...`)
    try {
      const result = await call<DownloadResult>('download_release', {
        request: { appId: appInfo.id, version, packagePreference: appInfo.packagePreference },
      })
      setState((current) => ({ ...current, apps: result.apps }))
      const downloadedApp = result.apps.find((candidate) => candidate.id === appInfo.id)
      const downloadedVersion = downloadedApp?.selectedVersion ?? downloadedApp?.installedVersion ?? version
      if (downloadedVersion) {
        setSelectedReleaseTags((current) => ({ ...current, [appInfo.id]: downloadedVersion }))
      }
      setNotice(`${appInfo.name} downloaded to ${fileName(result.installFolder) || fileName(result.filePath)}`)
      return true
    } catch (error) {
      setFailedDownloads((current) => ({ ...current, [appInfo.id]: true }))
      setNotice(error instanceof Error ? error.message : String(error))
      return false
    } finally {
      setActiveDownloads((current) => current.filter((id) => id !== appInfo.id))
      if (manageBusy) setBusy(false)
    }
  }

  async function downloadAllMissing() {
    const targets = visibleApps.filter((appInfo) => hasKnownRelease(appInfo) && !isAppDownloaded(appInfo))
    if (targets.length === 0) return
    setBusy(true)
    let successCount = 0
    for (const appInfo of targets) {
      if (await downloadRelease(appInfo, appInfo.latestVersion ?? appInfo.releaseOptions[0]?.tagName ?? null, false)) {
        successCount += 1
      }
    }
    setBusy(false)
    setNotice(`Downloaded ${successCount}/${targets.length} missing apps`)
  }

  async function updateAll() {
    const targets = visibleApps.filter((appInfo) => versionStatus(appInfo) === 'update')
    if (targets.length === 0) return
    setBusy(true)
    let successCount = 0
    for (const appInfo of targets) {
      if (await downloadRelease(appInfo, appInfo.latestVersion ?? null, false)) {
        successCount += 1
      }
    }
    setBusy(false)
    setNotice(`Updated ${successCount}/${targets.length} apps`)
  }

  async function openInstallFolder(appInfo: LauncherApp) {
    try {
      await call<void>('open_install_folder', { request: { appId: appInfo.id } })
      setNotice(`${appInfo.name} folder opened`)
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error))
    }
  }

  function openAppContextMenu(event: React.MouseEvent<HTMLElement>, appInfo: LauncherApp) {
    event.preventDefault()
    event.stopPropagation()
    clearDrag()
    setSelectedId(appInfo.id)
    setContextMenu({
      appId: appInfo.id,
      x: Math.min(event.clientX, Math.max(12, window.innerWidth - 190)),
      y: Math.min(event.clientY, Math.max(12, window.innerHeight - 150)),
    })
  }

  function closeContextMenu() {
    setContextMenu(null)
  }

  async function setTheme(theme: ThemeId) {
    setState((current) => ({ ...current, settings: { ...current.settings, theme } }))
    if (!isTauriRuntime()) return
    try {
      const settings = await call<AppSettings>('save_settings', { request: { theme } })
      setState((current) => ({ ...current, settings }))
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error))
    }
  }

  async function setCompactLabels(compactLabels: boolean) {
    setState((current) => ({ ...current, settings: { ...current.settings, compactLabels } }))
    if (!isTauriRuntime()) return
    try {
      const settings = await call<AppSettings>('save_settings', { request: { compactLabels } })
      setState((current) => ({ ...current, settings }))
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error))
    }
  }

  async function setUseRemoteCatalog(useRemoteCatalog: boolean) {
    setState((current) => ({ ...current, settings: { ...current.settings, useRemoteCatalog } }))
    if (!isTauriRuntime()) {
      setNotice(useRemoteCatalog ? 'Remote catalog enabled' : 'Local catalog enabled')
      return
    }
    setBusy(true)
    try {
      await call<AppSettings>('save_settings', { request: { useRemoteCatalog } })
      await loadState()
      setNotice(useRemoteCatalog ? 'Using remote tools catalog' : 'Using local tools catalog')
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(false)
    }
  }

  async function setAgentControlRuntime(enabled: boolean) {
    setAgentControlEnabled(enabled)
    if (!isTauriRuntime()) {
      setNotice(enabled ? 'Agent control enabled' : 'Agent control disabled')
      return
    }

    try {
      if (enabled) {
        const info = await call<AgentControlInfo>('get_agent_control_info')
        setAgentControlInfo(info)
        setNotice(`Agent control enabled: ${fileName(info.inboxDir)}`)
      } else {
        setNotice('Agent control disabled')
      }
    } catch (error) {
      setAgentControlEnabled(false)
      setNotice(error instanceof Error ? error.message : String(error))
    }
  }

  async function setAgentControlAutoStart(agentControlAutoStart: boolean) {
    setState((current) => ({ ...current, settings: { ...current.settings, agentControlAutoStart } }))
    if (!isTauriRuntime()) return
    try {
      const settings = await call<AppSettings>('save_settings', { request: { agentControlAutoStart } })
      setState((current) => ({ ...current, settings }))
      setNotice(agentControlAutoStart ? 'Agent control will start with the app' : 'Agent control will stay off at startup')
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error))
    }
  }

  async function persistLayout(apps: LauncherApp[], message: string, categories = layoutCategories) {
    const nextApps = apps.map((appInfo) => ({ ...appInfo, category: categoryLabel(appInfo.category) }))
    const nextCategories = normalizeCategories(categories)
    setState((current) => ({ ...current, apps: nextApps, settings: { ...current.settings, categories: nextCategories } }))
    const nextVisibleApps = nextApps.filter((candidate) => candidate.visible)
    setSelectedId((current) => nextVisibleApps.some((candidate) => candidate.id === current) ? current : nextVisibleApps[0]?.id ?? nextApps[0]?.id ?? '')

    if (!isTauriRuntime()) {
      setNotice(message)
      return
    }

    try {
      const savedState = await call<ControlCenterState>('save_layout', { request: { apps: nextApps, categories: nextCategories } })
      setState(savedState)
      setNotice(message)
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error))
    }
  }

  function setPackagePreference(appId: string, packagePreference: PackagePreference) {
    const apps = state.apps.map((candidate) => candidate.id === appId ? { ...candidate, packagePreference } : candidate)
    void persistLayout(apps, packagePreference === 'portable' ? 'Portable downloads selected' : 'Installer downloads selected')
  }

  function getDragItem(event: React.DragEvent): DragItem | null {
    if (draggedItemRef.current) return draggedItemRef.current
    const rawItem = event.dataTransfer.getData(dragDataType)
    if (!rawItem) return null
    try {
      const item = JSON.parse(rawItem) as DragItem
      if (item.kind === 'app' || item.kind === 'category') {
        return item
      }
    } catch {
      return null
    }
    return null
  }

  function clearDrag() {
    clearLayoutAnimationQueue()
    if (dragListenersRef.current) {
      window.removeEventListener('pointermove', dragListenersRef.current.move)
      window.removeEventListener('pointerup', dragListenersRef.current.up)
      window.removeEventListener('pointercancel', dragListenersRef.current.cancel)
      dragListenersRef.current = null
    }
    pointerDragRef.current = null
    draggedItemRef.current = null
    setDraggedItem(null)
    setDragGhost(null)
  }

  function dragPosition(pointerDrag: PointerDrag, clientX: number, clientY: number): { x: number; y: number } {
    return {
      x: clientX - pointerDrag.offsetX,
      y: clientY - pointerDrag.offsetY,
    }
  }

  function dragCenter(pointerDrag: PointerDrag, clientX: number, clientY: number): { x: number; y: number } {
    const position = dragPosition(pointerDrag, clientX, clientY)
    return {
      x: position.x + pointerDrag.width / 2,
      y: position.y + pointerDrag.height / 2,
    }
  }

  function dragGhostForItem(item: DragItem, x: number, y: number, width: number, height: number): DragGhost | null {
    if (item.kind === 'category') {
      return { item, x, y, width, height, icon: '', title: item.label, subtitle: 'Category', accent: '#ff6a00' }
    }

    const appInfo = appsRef.current.find((candidate) => candidate.id === item.id)
    if (!appInfo) return null
    return {
      item,
      x,
      y,
      width,
      height,
      icon: appInfo.icon,
      title: appInfo.name,
      subtitle: displayStatusLabel(displayStatus(appInfo)),
      accent: appInfo.accent,
    }
  }

  function setLocalLayout(apps: LauncherApp[], categories = categoriesRef.current) {
    appsRef.current = apps
    categoriesRef.current = normalizeCategories(categories)
    setState((current) => ({
      ...current,
      apps,
      settings: { ...current.settings, categories: categoriesRef.current },
    }))
  }

  function gridAnimationItems(): HTMLElement[] {
    return Array.from(gridRef.current?.querySelectorAll<HTMLElement>('[data-layout-id]') ?? [])
  }

  function gridAnimationRects(items = gridAnimationItems()): Map<string, DOMRect> {
    const rects = new Map<string, DOMRect>()
    for (const item of items) {
      const id = item.dataset.layoutId
      if (id) {
        rects.set(id, item.getBoundingClientRect())
      }
    }
    return rects
  }

  function cancelGridAnimations(items = gridAnimationItems()) {
    for (const item of items) {
      for (const animation of item.getAnimations()) {
        animation.cancel()
      }
    }
  }

  function clearLayoutAnimationTimer() {
    if (layoutAnimationTimerRef.current !== null) {
      window.clearTimeout(layoutAnimationTimerRef.current)
      layoutAnimationTimerRef.current = null
    }
  }

  function clearLayoutAnimationQueue() {
    layoutAnimationRunIdRef.current += 1
    layoutAnimationLockedRef.current = false
    pendingAppPreviewRef.current = null
    clearLayoutAnimationTimer()
  }

  function finishLayoutAnimation(runId: number) {
    if (layoutAnimationRunIdRef.current !== runId || !layoutAnimationLockedRef.current) return

    layoutAnimationLockedRef.current = false
    clearLayoutAnimationTimer()

    const pendingPreview = pendingAppPreviewRef.current
    pendingAppPreviewRef.current = null

    if (
      pendingPreview
      && draggedItemRef.current?.kind === 'app'
      && draggedItemRef.current.id === pendingPreview.appId
    ) {
      previewAppSort(
        pendingPreview.appId,
        pendingPreview.clientX,
        pendingPreview.clientY,
        pendingPreview.directX,
        pendingPreview.directY,
      )
    }
  }

  function animateGridLayout(applyLayout: () => void, onComplete?: () => void) {
    const tweenId = layoutTweenIdRef.current + 1
    layoutTweenIdRef.current = tweenId
    const currentItems = gridAnimationItems()
    const before = gridAnimationRects(currentItems)
    cancelGridAnimations(currentItems)
    applyLayout()

    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => {
        if (layoutTweenIdRef.current !== tweenId) return

        const items = gridAnimationItems()
        const animations: Animation[] = []
        for (const item of items) {
          if (item.dataset.placeholderAppId) continue

          const id = item.dataset.layoutId
          const previous = id ? before.get(id) : null
          if (!previous) continue

          const next = item.getBoundingClientRect()
          const deltaX = previous.left - next.left
          const deltaY = previous.top - next.top
          if (Math.abs(deltaX) < 1 && Math.abs(deltaY) < 1) continue

          animations.push(
            item.animate(
              [
                { transform: `translate(${deltaX}px, ${deltaY}px)` },
                { transform: 'translate(0, 0)' },
              ],
              {
                duration: layoutAnimationDurationMs,
                easing: 'cubic-bezier(0.2, 0, 0, 1)',
                fill: 'none',
              },
            ),
          )
        }

        if (animations.length === 0) {
          onComplete?.()
          return
        }

        void Promise.allSettled(animations.map((animation) => animation.finished)).then(() => onComplete?.())
      })
    })
  }

  function startLayoutPreviewAnimation(apps: LauncherApp[]) {
    const runId = layoutAnimationRunIdRef.current + 1
    layoutAnimationRunIdRef.current = runId
    layoutAnimationLockedRef.current = true
    clearLayoutAnimationTimer()
    layoutAnimationTimerRef.current = window.setTimeout(() => finishLayoutAnimation(runId), layoutAnimationMaxLockMs)
    animateGridLayout(() => setLocalLayout(apps), () => finishLayoutAnimation(runId))
  }

  function categoryFromY(clientY: number): string {
    const rows = Array.from(gridRef.current?.querySelectorAll<HTMLElement>('.category-row[data-category]') ?? [])
    let selectedCategory = rows[0]?.dataset.category ?? layoutCategories[0] ?? defaultCategories[0]
    for (const row of rows) {
      if (clientY >= row.getBoundingClientRect().top) {
        selectedCategory = row.dataset.category ?? selectedCategory
      }
    }
    return categoryLabel(selectedCategory)
  }

  function categoryFromDropPosition(event: React.DragEvent<HTMLElement>): string {
    return categoryFromY(event.clientY)
  }

  function appSlots(): AppLayoutSlot[] {
    const items = Array.from(
      gridRef.current?.querySelectorAll<HTMLElement>('.app-tile[data-app-id], .app-placeholder[data-placeholder-app-id]') ?? [],
    )

    return items.map((item) => {
      const rect = item.getBoundingClientRect()
      return {
        appId: item.dataset.appId ?? item.dataset.placeholderAppId ?? '',
        category: categoryLabel(item.dataset.category ?? ''),
        rect,
        centerX: rect.left + rect.width / 2,
        centerY: rect.top + rect.height / 2,
      }
    }).filter((slot) => slot.appId && slot.category)
  }

  function categoryAtPoint(clientX: number, clientY: number): string | null {
    const row = Array.from(gridRef.current?.querySelectorAll<HTMLElement>('.category-row[data-category]') ?? []).find((candidate) => {
      const rect = candidate.getBoundingClientRect()
      return clientX >= rect.left && clientX <= rect.right && clientY >= rect.top && clientY <= rect.bottom
    })
    return row?.dataset.category ? categoryLabel(row.dataset.category) : null
  }

  function nearestAppSlot(clientX: number, clientY: number, directX = clientX, directY = clientY): AppLayoutSlot | null {
    const slots = appSlots()
    if (slots.length === 0) return null

    const directSlot = slots.find((slot) => {
      const { rect } = slot
      return directX >= rect.left && directX <= rect.right && directY >= rect.top && directY <= rect.bottom
    })
    if (directSlot) return directSlot

    const activeCategory = categoryFromY(clientY)
    const categorySlots = slots.filter((slot) => slot.category === activeCategory)
    const candidates = categorySlots.length > 0 ? categorySlots : slots

    return candidates.reduce<AppLayoutSlot | null>((nearest, slot) => {
      const dx = clientX - slot.centerX
      const dy = clientY - slot.centerY
      const distance = dx * dx + dy * dy * 1.35
      if (!nearest) return slot

      const nearestDx = clientX - nearest.centerX
      const nearestDy = clientY - nearest.centerY
      const nearestDistance = nearestDx * nearestDx + nearestDy * nearestDy * 1.35
      return distance < nearestDistance ? slot : nearest
    }, null)
  }

  function appLayoutForPointer(appId: string, clientX: number, clientY: number, directX = clientX, directY = clientY): LauncherApp[] {
    const categoryTarget = categoryAtPoint(directX, directY)
    if (categoryTarget) {
      return moveAppToCategory(appsRef.current, appId, categoryTarget)
    }

    const slotTarget = nearestAppSlot(clientX, clientY, directX, directY)
    if (slotTarget?.appId === appId) return appsRef.current

    const currentApps = appsRef.current

    if (slotTarget?.appId) {
      const targetApp = currentApps.find((candidate) => candidate.id === slotTarget.appId)
      if (targetApp) {
        return moveAppToAppSlot(currentApps, appId, targetApp)
      }
    }

    return moveAppToCategory(currentApps, appId, categoryFromY(clientY))
  }

  function previewAppSort(appId: string, clientX: number, clientY: number, directX = clientX, directY = clientY) {
    if (layoutAnimationLockedRef.current) {
      pendingAppPreviewRef.current = { appId, clientX, clientY, directX, directY }
      return
    }

    const currentApps = appsRef.current
    const apps = appLayoutForPointer(appId, clientX, clientY, directX, directY)

    if (!sameAppLayout(currentApps, apps)) {
      startLayoutPreviewAnimation(apps)
    }
  }

  function applyDropItemToApp(item: DragItem, target: LauncherApp) {
    const nextCategories = orderedCategories(state.apps, state.settings.categories)
    if (item.kind === 'category') {
      const categories = moveCategory(nextCategories, item.label, categoryLabel(target.category))
      if (categories !== nextCategories) {
        void persistLayout(state.apps, 'Category moved', categories)
      }
      return
    }

    const apps = moveAppToApp(state.apps, item.id, target)
    if (apps !== state.apps) {
      void persistLayout(apps, 'Layout saved')
    }
  }

  function applyDropItemToCategory(item: DragItem, category: string) {
    const nextCategories = orderedCategories(state.apps, state.settings.categories)
    if (item.kind === 'category') {
      const categories = moveCategory(nextCategories, item.label, category)
      if (categories !== nextCategories) {
        void persistLayout(state.apps, 'Category moved', categories)
      }
      return
    }

    const apps = moveAppToCategory(state.apps, item.id, category)
    if (apps !== state.apps) {
      void persistLayout(apps, 'App moved')
    }
  }

  function handleDropOnApp(event: React.DragEvent, target: LauncherApp) {
    const item = getDragItem(event)
    if (!item) return
    clearDrag()
    applyDropItemToApp(item, target)
  }

  function handleDropOnCategory(event: React.DragEvent, category: string) {
    const item = getDragItem(event)
    if (!item) return
    clearDrag()
    applyDropItemToCategory(item, category)
  }

  function handleDropOnGrid(event: React.DragEvent<HTMLElement>) {
    event.preventDefault()
    const item = getDragItem(event)
    if (!item) return
    handleDropOnCategory(event, categoryFromDropPosition(event))
  }

  function startPointerDrag(event: React.PointerEvent<HTMLElement>, item: DragItem) {
    if (event.button !== 0) return
    clearDrag()
    const rect = event.currentTarget.getBoundingClientRect()
    pointerDragRef.current = {
      item,
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      offsetX: event.clientX - rect.left,
      offsetY: event.clientY - rect.top,
      width: rect.width,
      height: rect.height,
      active: false,
    }
    dragListenersRef.current = {
      move: (nextEvent) => movePointerDrag(nextEvent),
      up: (nextEvent) => endPointerDrag(nextEvent),
      cancel: (nextEvent) => {
        if (pointerDragRef.current?.pointerId === nextEvent.pointerId) {
          clearDrag()
        }
      },
    }
    window.addEventListener('pointermove', dragListenersRef.current.move)
    window.addEventListener('pointerup', dragListenersRef.current.up)
    window.addEventListener('pointercancel', dragListenersRef.current.cancel)
  }

  function movePointerDrag(event: DragPointerEvent) {
    const pointerDrag = pointerDragRef.current
    if (!pointerDrag || pointerDrag.pointerId !== event.pointerId) return
    const distance = Math.hypot(event.clientX - pointerDrag.startX, event.clientY - pointerDrag.startY)
    if (!pointerDrag.active && distance > 6) {
      pointerDrag.active = true
      draggedItemRef.current = pointerDrag.item
      setDraggedItem(pointerDrag.item)
      const position = dragPosition(pointerDrag, event.clientX, event.clientY)
      setDragGhost(dragGhostForItem(pointerDrag.item, position.x, position.y, pointerDrag.width, pointerDrag.height))
    }
    if (pointerDrag.active) {
      event.preventDefault()
      const position = dragPosition(pointerDrag, event.clientX, event.clientY)
      setDragGhost((current) => (
        current
          ? { ...current, x: position.x, y: position.y }
          : dragGhostForItem(pointerDrag.item, position.x, position.y, pointerDrag.width, pointerDrag.height)
      ))
      if (pointerDrag.item.kind === 'app') {
        const center = dragCenter(pointerDrag, event.clientX, event.clientY)
        previewAppSort(pointerDrag.item.id, center.x, center.y, event.clientX, event.clientY)
      }
    }
  }

  function endPointerDrag(event: DragPointerEvent) {
    const pointerDrag = pointerDragRef.current
    if (!pointerDrag || pointerDrag.pointerId !== event.pointerId) return

    if (!pointerDrag.active) {
      clearDrag()
      return
    }

    event.preventDefault()
    event.stopPropagation()
    suppressClickRef.current = true
    window.setTimeout(() => {
      suppressClickRef.current = false
    }, 0)

    if (pointerDrag.item.kind === 'app') {
      const center = dragCenter(pointerDrag, event.clientX, event.clientY)
      const finalApps = appLayoutForPointer(pointerDrag.item.id, center.x, center.y, event.clientX, event.clientY)
      if (!sameAppLayout(appsRef.current, finalApps)) {
        setLocalLayout(finalApps)
      }
      clearDrag()
      void persistLayout(appsRef.current, 'Layout saved', categoriesRef.current)
      return
    }

    const dropTarget = document.elementFromPoint(event.clientX, event.clientY) as HTMLElement | null
    const appTarget = dropTarget?.closest<HTMLElement>('.app-tile[data-app-id]')
    const categoryTarget = dropTarget?.closest<HTMLElement>('[data-category]')
    const targetApp = appTarget?.dataset.appId ? state.apps.find((candidate) => candidate.id === appTarget.dataset.appId) : null
    const targetCategory = categoryTarget?.dataset.category ?? categoryFromY(event.clientY)

    clearDrag()
    if (targetApp) {
      applyDropItemToApp(pointerDrag.item, targetApp)
      return
    }
    applyDropItemToCategory(pointerDrag.item, targetCategory)
  }

  function addCategory() {
    const rawName = window.prompt('Category name', 'New Category')
    if (rawName === null) return
    const name = categoryLabel(rawName)
    if (layoutCategories.includes(name)) {
      setNotice('Category already exists')
      return
    }
    void persistLayout(state.apps, 'Category added', [...layoutCategories, name])
  }

  function renameCategory(category: string) {
    const rawName = window.prompt('Category name', category)
    if (rawName === null) return
    const name = categoryLabel(rawName)
    if (name === category) return
    if (layoutCategories.includes(name)) {
      setNotice('Category already exists')
      return
    }
    const categories = layoutCategories.map((candidate) => candidate === category ? name : candidate)
    const apps = state.apps.map((appInfo) => categoryLabel(appInfo.category) === category ? { ...appInfo, category: name } : appInfo)
    void persistLayout(apps, 'Category renamed', categories)
  }

  function deleteCategory(category: string) {
    if (layoutCategories.length <= 1) {
      setNotice('At least one category must stay visible')
      return
    }
    if (!window.confirm(`Delete ${category}? Apps in it will move to the first category.`)) return
    const categories = layoutCategories.filter((candidate) => candidate !== category)
    const fallbackCategory = categories[0] ?? defaultCategories[0]
    const apps = state.apps.map((appInfo) => categoryLabel(appInfo.category) === category ? { ...appInfo, category: fallbackCategory } : appInfo)
    void persistLayout(apps, 'Category deleted', categories)
  }

  function setAppVisible(appId: string, visible: boolean) {
    const visibleCount = state.apps.filter((candidate) => candidate.visible).length
    if (!visible && visibleCount <= 1) {
      setNotice('At least one app must stay visible')
      return
    }
    const apps = state.apps.map((candidate) => candidate.id === appId ? { ...candidate, visible } : candidate)
    void persistLayout(apps, visible ? 'App shown' : 'App hidden')
  }

  async function resetLayout() {
    setBusy(true)
    try {
      if (!isTauriRuntime()) {
        const apps = fallbackState.apps.map((candidate) => ({ ...candidate, visible: true }))
        setState((current) => ({ ...current, apps, settings: { ...current.settings, categories: defaultCategories } }))
        setSelectedId(apps[0]?.id ?? '')
        setNotice('Layout reset')
        return
      }

      const nextState = await call<ControlCenterState>('reset_layout')
      setState(nextState)
      setSelectedId(nextState.apps.find((candidate) => candidate.visible)?.id ?? nextState.apps[0]?.id ?? '')
      setNotice('Layout reset. Scanning releases...')
      const { update, promotedCount } = await runReleaseScan(nextState.apps)
      const releaseSummary = promotedCount > 0 ? ` - ${promotedCount} newly released` : ''
      setNotice(update?.updateAvailable ? `Layout reset and scan complete${releaseSummary} - Control Center update available` : `Layout reset and scan complete${releaseSummary}`)
    } catch (error) {
      setNotice(error instanceof Error ? error.message : String(error))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div
      className={classNames('app-shell', `theme-${state.settings.theme}`, state.settings.compactLabels && 'compact-labels', draggedItem && 'dragging-layout')}
      onClick={closeContextMenu}
    >
      <header className="topbar">
        <div className="brand-lockup">
          <div>
            <h1>Neko Legends Control Center</h1>
            <p>{configuredCount}/{visibleApps.length} apps wired · {updateCount} updates{hiddenCount > 0 ? ` · ${hiddenCount} hidden` : ''}</p>
          </div>
        </div>
        <div className="control-update-slot">
          {controlCenterUpdate?.updateAvailable && (
            <button
              className="control-update-button"
              type="button"
              onClick={() => void installControlCenterUpdate()}
              disabled={busy || controlCenterUpdating}
              title={`Update available: Control Center ${controlCenterUpdate.latestVersion ?? ''}. Download, restart, and install the latest portable build.`}
            >
              {controlCenterUpdating ? <Loader2 className="spin" size={16} /> : <Download size={16} />}
              {controlCenterUpdating ? 'Installing Update' : 'Update Control Center'}
              <span>{controlCenterUpdate.latestVersion}</span>
            </button>
          )}
        </div>
        <div className="topbar-actions">
          <button className="icon-button" type="button" onClick={() => setSettingsOpen((open) => !open)} title="Theme settings">
            <Settings2 size={17} />
          </button>
          <button className="scan-button" type="button" onClick={scanForUpdates} disabled={busy} title="Scan GitHub releases">
            {busy ? <Loader2 className="spin" size={17} /> : <RefreshCw size={17} />}
            Scan
          </button>
          <button className="scan-button" type="button" onClick={() => void downloadAllMissing()} disabled={busy || missingCount === 0} title="Download missing apps">
            <Download size={17} />
            Get Missing
          </button>
          <button className="scan-button" type="button" onClick={() => void updateAll()} disabled={busy || updateCount === 0} title="Download available updates">
            <RefreshCw size={17} />
            Update All
          </button>
        </div>
      </header>

      {settingsOpen && (
        <section className="settings-band">
          <div className="settings-main">
            <div className="theme-row">
              {themes.map((theme) => (
                <button
                  className={classNames('theme-button', state.settings.theme === theme.id && 'active')}
                  type="button"
                  key={theme.id}
                  onClick={() => void setTheme(theme.id)}
                  title={theme.name}
                >
                  <span className="swatches">
                    {theme.colors.map((color) => <i key={color} style={{ background: color }} />)}
                  </span>
                  <strong>{theme.name}</strong>
                </button>
              ))}
            </div>
            <div className="visibility-row" aria-label="Visible apps">
              {state.apps.map((appInfo) => (
                <label className={classNames('app-toggle', !appInfo.visible && 'hidden')} key={appInfo.id} title={appInfo.name}>
                  <input
                    type="checkbox"
                    checked={appInfo.visible}
                    onChange={(event) => setAppVisible(appInfo.id, event.currentTarget.checked)}
                  />
                  <span>{appInfo.icon}</span>
                </label>
              ))}
            </div>
            <div className="category-tools" aria-label="Categories">
              {layoutCategories.map((category) => (
                <span
                  className="category-chip"
                  key={category}
                  onPointerDown={(event) => startPointerDrag(event, { kind: 'category', label: category })}
                  onDragOver={(event) => {
                    event.preventDefault()
                    event.dataTransfer.dropEffect = 'move'
                  }}
                  onDragEnd={clearDrag}
                  onDrop={(event) => {
                    event.preventDefault()
                    event.stopPropagation()
                    handleDropOnCategory(event, category)
                  }}
                >
                  <GripVertical size={12} />
                  <span>{category}</span>
                  <button type="button" onPointerDown={(event) => event.stopPropagation()} onClick={() => renameCategory(category)} title={`Rename ${category}`}>
                    <Pencil size={12} />
                  </button>
                  <button type="button" onPointerDown={(event) => event.stopPropagation()} onClick={() => deleteCategory(category)} disabled={layoutCategories.length <= 1} title={`Delete ${category}`}>
                    <Trash2 size={12} />
                  </button>
                </span>
              ))}
              <button className="category-add" type="button" onClick={addCategory} title="Add category">
                <Plus size={14} />
                <span>Category</span>
              </button>
            </div>
            <div className="agent-api-panel" aria-label="Agent API ports">
              <div className="agent-api-heading">
                <strong>Agent APIs</strong>
                <span>{agentApiConflictCount > 0 ? `${agentApiConflictCount} conflict${agentApiConflictCount === 1 ? '' : 's'}` : 'Ports clear'}</span>
                <button type="button" onClick={() => void refreshAgentApiDashboard()} title={agentApiDashboard?.registryPath ?? 'Refresh Agent API registry'}>
                  <RefreshCw size={13} />
                </button>
              </div>
              <div className="agent-api-list">
                {(agentApiDashboard?.apps ?? []).map((entry) => {
                  const conflict = conflictForPort(agentApiDashboard, entry.port)
                  const portValue = agentApiPortEdits[entry.appId] ?? String(entry.port)
                  const canLaunchHeadless = supportsHeadlessAgentLaunch(entry.appId)
                  return (
                    <div className={classNames('agent-api-row', conflict && 'conflict')} key={entry.appId}>
                      <div className="agent-api-name">
                        <strong>{entry.appName}</strong>
                        <span>{entry.enabled ? (entry.busy ? 'Busy' : 'On') : 'Off'}</span>
                      </div>
                      <label className="agent-api-port">
                        <span>Port</span>
                        <input
                          type="number"
                          min="1"
                          max="65535"
                          value={portValue}
                          onChange={(event) => setAgentApiPortEdits((current) => ({ ...current, [entry.appId]: event.currentTarget.value }))}
                          onKeyDown={(event) => {
                            if (event.key === 'Enter') {
                              void saveAgentApiPort(entry.appId, portValue)
                            }
                          }}
                        />
                      </label>
                      <div className="agent-api-url" title={entry.openapiUrl}>
                        <span>{entry.url}</span>
                        {conflict ? <small>{conflict.appNames.join(', ')}</small> : <small>Default {entry.defaultPort}</small>}
                      </div>
                      <div className="agent-api-actions">
                        <button
                          type="button"
                          onClick={() => void launchHeadlessAgentApi(entry)}
                          disabled={!canLaunchHeadless}
                          title={canLaunchHeadless ? `Start ${entry.appName} headless API` : `${entry.appName} does not support headless launch yet`}
                        >
                          Headless
                        </button>
                        <button type="button" onClick={() => void saveAgentApiPort(entry.appId, portValue)} title={`Save ${entry.appName} port`}>
                          Save
                        </button>
                        <button type="button" onClick={() => void assignNextAgentApiPort(entry.appId)} title={`Use ${agentApiDashboard?.nextAvailablePort ?? 'next free port'}`}>
                          Next
                        </button>
                      </div>
                    </div>
                  )
                })}
              </div>
            </div>
          </div>
          <div className="settings-actions">
            <label className="toggle-row">
              <input
                type="checkbox"
                checked={state.settings.compactLabels}
                onChange={(event) => void setCompactLabels(event.currentTarget.checked)}
              />
              <span>Compact labels</span>
            </label>
            <label className="toggle-row" title="Use the hosted tools.json catalog. Turn off to test the local catalog file.">
              <input
                type="checkbox"
                checked={state.settings.useRemoteCatalog}
                onChange={(event) => void setUseRemoteCatalog(event.currentTarget.checked)}
                disabled={busy}
              />
              <span>Remote catalog</span>
            </label>
            <label className="toggle-row" title={agentControlInfo ? `Inbox: ${agentControlInfo.inboxDir}` : 'Allow local AI agents to control downloads, updates, and launches'}>
              <input
                type="checkbox"
                checked={agentControlEnabled}
                onChange={(event) => void setAgentControlRuntime(event.currentTarget.checked)}
                disabled={busy}
              />
              <span>Agent control</span>
            </label>
            <label className="toggle-row" title="Start Agent control automatically when the Control Center opens">
              <input
                type="checkbox"
                checked={state.settings.agentControlAutoStart}
                onChange={(event) => void setAgentControlAutoStart(event.currentTarget.checked)}
                disabled={busy}
              />
              <span>Agent auto-on</span>
            </label>
            <button className="secondary-action reset-action" type="button" onClick={resetLayout} disabled={busy}>
              Reset layout
            </button>
          </div>
        </section>
      )}

      <main className="launcher-layout">
        <section
          className="app-grid"
          aria-label="App launcher"
          ref={gridRef}
          style={{ '--visible-count': visibleApps.length } as React.CSSProperties}
          onDragOver={(event) => {
            if (draggedItemRef.current) {
              event.preventDefault()
              event.dataTransfer.dropEffect = 'move'
            }
          }}
          onDrop={handleDropOnGrid}
        >
          {visibleGridItems.map((item) => {
            if (item.kind === 'category') {
              return (
                <div
                  className="category-row"
                  key={item.id}
                  data-category={item.label}
                  onPointerDown={(event) => startPointerDrag(event, { kind: 'category', label: item.label })}
                  onDragOver={(event) => {
                    event.preventDefault()
                    event.dataTransfer.dropEffect = 'move'
                  }}
                  onDragEnd={clearDrag}
                  onDrop={(event) => {
                    event.preventDefault()
                    event.stopPropagation()
                    handleDropOnCategory(event, item.label)
                  }}
                >
                  <GripVertical size={13} />
                  <span>{`-= ${item.label} =-`}</span>
                </div>
              )
            }

            const appInfo = item.app
            const selected = appInfo.id === selectedApp?.id
            const status = versionStatus(appInfo)
            const tileStatus = displayStatus(appInfo)
            const isNewRelease = newReleaseIds.includes(appInfo.id)
            const isDraggedApp = draggedItem?.kind === 'app' && draggedItem.id === appInfo.id

            if (isDraggedApp) {
              return (
                <div
                  className="app-placeholder"
                  key={appInfo.id}
                  data-category={categoryLabel(appInfo.category)}
                  data-placeholder-app-id={appInfo.id}
                  data-layout-id={appInfo.id}
                  aria-hidden="true"
                />
              )
            }

            return (
              <button
                key={appInfo.id}
                type="button"
                className={classNames('app-tile', selected && 'selected')}
                style={{ '--app-accent': appInfo.accent } as React.CSSProperties}
                data-category={categoryLabel(appInfo.category)}
                data-app-id={appInfo.id}
                data-layout-id={appInfo.id}
                onPointerDown={(event) => startPointerDrag(event, { kind: 'app', id: appInfo.id })}
                onDragOver={(event) => {
                  event.preventDefault()
                  event.dataTransfer.dropEffect = 'move'
                }}
                onDragEnd={clearDrag}
                onDrop={(event) => {
                  event.preventDefault()
                  event.stopPropagation()
                  handleDropOnApp(event, appInfo)
                }}
                onContextMenu={(event) => openAppContextMenu(event, appInfo)}
                onClick={(event) => {
                  if (suppressClickRef.current) {
                    event.preventDefault()
                    return
                  }
                  setSelectedId(appInfo.id)
                }}
                onDoubleClick={(event) => {
                  if (suppressClickRef.current) {
                    event.preventDefault()
                    return
                  }
                  appInfo.demoUrl
                    ? void viewDemo(appInfo)
                    : canLaunchSelectedVersion(appInfo)
                      ? void launchApp(appInfo)
                      : void chooseExecutable(appInfo)
                }}
              >
                <span className="app-icon">{appInfo.icon}</span>
                <span className="app-copy">
                  <strong>{appInfo.name}</strong>
                  <small className={classNames('app-status', tileStatus)}>
                    {displayStatusLabel(tileStatus)}
                  </small>
                </span>
                {status === 'update' && <span className="update-dot" />}
                {isNewRelease && <span className="new-release-badge">New</span>}
              </button>
            )
          })}
        </section>

        {selectedApp && (
          <aside className="detail-panel">
            <div className="detail-heading">
              <div className="detail-icon" style={{ '--app-accent': selectedApp.accent } as React.CSSProperties}>
                {selectedApp.icon}
              </div>
              <div>
                <h2>
                  <span>{selectedApp.name}</span>
                  {newReleaseIds.includes(selectedApp.id) && <span className="new-release-inline">New release</span>}
                </h2>
                <p>{selectedApp.description}</p>
              </div>
            </div>

            <div className="detail-meta">
              <div>
                <span>Repository</span>
                <strong>{githubOwner}/{selectedApp.repo}</strong>
              </div>
              <div>
                <span>Latest</span>
                <strong>{selectedApp.latestVersion ?? 'Unknown'}</strong>
              </div>
              <div>
                <span>Installed</span>
                <strong>{selectedApp.installedVersion ?? 'None'}</strong>
              </div>
              <div>
                <span>Checked</span>
                <strong>{formatCheckedAt(selectedApp.releaseCheckedAt)}</strong>
              </div>
              <div>
                <span>{selectedApp.demoUrl ? 'Package' : 'Launch file'}</span>
                <strong>{appArtifactPath(selectedApp) ? fileName(appArtifactPath(selectedApp) ?? '') : 'Not set'}</strong>
              </div>
            </div>

            <label className="version-picker">
              <span>Version</span>
              <div className="version-picker-row">
                <select
                  value={selectedVersionFor(selectedApp) ?? ''}
                  disabled={busy || selectedApp.releaseOptions.length === 0}
                  onChange={(event) => setSelectedReleaseTags((current) => ({ ...current, [selectedApp.id]: event.currentTarget.value }))}
                >
                  {selectedApp.releaseOptions.length === 0 && <option value="">Scan releases</option>}
                  {selectedApp.releaseOptions.map((release) => (
                    <option value={release.tagName} key={release.tagName}>
                      {release.tagName}{isVersionInstalled(selectedApp, release.tagName) ? ' (Installed)' : ''}
                    </option>
                  ))}
                </select>
                {selectedVersionInstalled(selectedApp) && <strong className="installed-version-badge">(Installed)</strong>}
              </div>
            </label>

            {selectedApp.installedVersions.length > 0 && (
              <div className="rollback-panel">
                <div>
                  <span>Installed builds</span>
                  <strong>{selectedApp.installedVersions.length}</strong>
                </div>
                <div className="rollback-list">
                  {selectedApp.installedVersions.map((installed, index) => {
                    const current = installed.version === selectedApp.installedVersion
                    return (
                      <button
                        type="button"
                        key={installed.version}
                        className={classNames(current && 'active')}
                        onClick={() => void selectInstalledVersion(selectedApp, installed.version, index === 1 ? 'Rolling back' : 'Switching')}
                        disabled={busy || current}
                        title={current ? `${installed.version} is active` : `Use ${installed.version}`}
                      >
                        <span>{current ? 'Current' : index === 1 ? 'Rollback' : 'Use'}</span>
                        <strong>{installed.version}</strong>
                      </button>
                    )
                  })}
                </div>
                {selectedPreviousVersion && <small>Previous: {selectedPreviousVersion.version}</small>}
              </div>
            )}

            <div className="package-picker" aria-label="Download type">
              <span>Download type</span>
              <div className="segmented-control">
                <button
                  className={classNames(selectedApp.packagePreference === 'portable' && 'active')}
                  type="button"
                  onClick={() => setPackagePreference(selectedApp.id, 'portable')}
                  title="Prefer portable or standalone downloads that can launch directly"
                >
                  Portable
                </button>
                <button
                  className={classNames(selectedApp.packagePreference === 'installer' && 'active')}
                  type="button"
                  onClick={() => setPackagePreference(selectedApp.id, 'installer')}
                  title="Prefer setup or MSI installer downloads"
                >
                  Installer
                </button>
              </div>
            </div>

            <div className="detail-actions">
              {selectedApp.demoUrl ? (
                <button className="primary-action" type="button" onClick={() => void viewDemo(selectedApp)} disabled={busy}>
                  <ExternalLink size={17} />
                  View Demo
                </button>
              ) : (
                <button className="primary-action" type="button" onClick={launchSelected} disabled={busy || !canLaunchSelectedVersion(selectedApp)}>
                  <Play size={17} />
                  {isAppRunning(selectedApp) ? 'Re-Launch' : 'Launch'}
                </button>
              )}
              <button className="secondary-action" type="button" onClick={() => void openRepository(selectedApp)} title="Open GitHub README">
                <Info size={17} />
                Info
              </button>
              {!selectedApp.demoUrl && (
                <button className="secondary-action" type="button" onClick={() => void chooseExecutable(selectedApp)}>
                  <FolderOpen size={17} />
                  Path
                </button>
              )}
              <button className="secondary-action" type="button" onClick={() => void openInstallFolder(selectedApp)}>
                <FolderOpen size={17} />
                Folder
              </button>
              <button
                className={selectedAppUpdateAvailable ? 'primary-action' : 'secondary-action'}
                type="button"
                onClick={() => void downloadRelease(
                  selectedApp,
                  selectedAppUpdateAvailable ? selectedApp.latestVersion ?? null : selectedVersionFor(selectedApp),
                  true,
                )}
                disabled={busy || isComingSoon(selectedApp) || (selectedAppUpdateAvailable && !selectedApp.latestVersion)}
                title={selectedAppUpdateAvailable ? `Update to ${selectedApp.latestVersion ?? 'the latest release'}` : 'Download selected release'}
              >
                {selectedAppUpdateAvailable ? <RefreshCw size={17} /> : <Download size={17} />}
                {selectedAppUpdateAvailable ? 'Update' : 'Download'}
              </button>
              <button className="secondary-action" type="button" onClick={() => void openRelease(selectedApp)}>
                <ExternalLink size={17} />
                Release
              </button>
            </div>

            <div className="release-note">
              <AppWindow size={16} />
              <span>{selectedApp.releaseNotes ?? 'Scan releases when you want the hub to check GitHub.'}</span>
            </div>
          </aside>
        )}
      </main>

      <footer className="statusbar">
        <span>{notice}</span>
        <span>v{state.buildVersion}</span>
      </footer>

      {contextMenu && contextMenuApp && (
        <div
          className="app-context-menu"
          style={{ left: contextMenu.x, top: contextMenu.y } as React.CSSProperties}
          onClick={(event) => event.stopPropagation()}
          onContextMenu={(event) => event.preventDefault()}
        >
          <div className="context-menu-title">{contextMenuApp.name}</div>
          {contextMenuApp.demoUrl && (
            <button
              type="button"
              onClick={() => {
                closeContextMenu()
                void viewDemo(contextMenuApp)
              }}
              disabled={busy}
            >
              <ExternalLink size={15} />
              View Demo
            </button>
          )}
          {!contextMenuApp.demoUrl && canLaunchSelectedVersion(contextMenuApp) && (
            <button
              type="button"
              onClick={() => {
                closeContextMenu()
                void launchApp(contextMenuApp)
              }}
              disabled={busy}
            >
              <Play size={15} />
              {isAppRunning(contextMenuApp) ? 'Re-Launch' : 'Launch'}
            </button>
          )}
          <button
            type="button"
            title="Open GitHub README"
            onClick={() => {
              closeContextMenu()
              void openRepository(contextMenuApp)
            }}
          >
            <Info size={15} />
            Info
          </button>
          {!isAppDownloaded(contextMenuApp) && !isComingSoon(contextMenuApp) && (
            <button
              type="button"
              onClick={() => {
                closeContextMenu()
                void downloadRelease(contextMenuApp, selectedVersionFor(contextMenuApp), true)
              }}
              disabled={busy}
            >
              <Download size={15} />
              Download
            </button>
          )}
          {isAppDownloaded(contextMenuApp) && versionStatus(contextMenuApp) === 'update' && (
            <button
              type="button"
              onClick={() => {
                closeContextMenu()
                void downloadRelease(contextMenuApp, contextMenuApp.latestVersion ?? null, true)
              }}
              disabled={busy}
            >
              <RefreshCw size={15} />
              Update
            </button>
          )}
        </div>
      )}

      {dragGhost && (
        <div
          className={classNames('drag-ghost', dragGhost.item.kind === 'category' && 'category-ghost')}
          style={{
            '--app-accent': dragGhost.accent,
            left: dragGhost.x,
            top: dragGhost.y,
            width: dragGhost.width,
            minHeight: dragGhost.height,
          } as React.CSSProperties}
        >
          {dragGhost.icon && <span className="app-icon">{dragGhost.icon}</span>}
          <span className="app-copy">
            <strong>{dragGhost.title}</strong>
            <small>{dragGhost.subtitle}</small>
          </span>
        </div>
      )}
    </div>
  )
}
