import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { updateStore } from '../stores/updateStore'
import type { UpdateStatus, UpdateState } from '../stores/updateStore'
import { get } from 'svelte/store'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface UpdateCheckResult {
  available: boolean
  current_version: string
  available_version: string | null
}

// ---------------------------------------------------------------------------
// Local state (not reactive — used for fast checks and guards)
// ---------------------------------------------------------------------------

/** Tracks the most recent update check result version. */
let _lastAvailableVersion: string | null = null

/**
 * Guards against:
 * - Multiple simultaneous checks
 * - Multiple simultaneous downloads
 * - Repeated installation attempts
 */
let _checkInProgress = false
let _downloadInProgress = false
let _installInProgress = false

/**
 * Guard flag so the close handler does not loop. Set right before
 * the update install IPC call and cleared on process exit.
 */
let _updateShutdownInProgress = false

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function log(...args: unknown[]) {
  console.info('[SilentUpdater]', ...args)
}

function warn(...args: unknown[]) {
  console.warn('[SilentUpdater]', ...args)
}

function setStatus(status: UpdateStatus, extra?: Partial<UpdateState>) {
  updateStore.update((s) => ({ ...s, status, ...extra }))
}

function setError(error: string) {
  log('Update error:', error)
  updateStore.update((s) => ({ ...s, status: 'error' as UpdateStatus, error }))
}

// ---------------------------------------------------------------------------
// API wrappers
// ---------------------------------------------------------------------------

async function checkForUpdate(): Promise<UpdateCheckResult | null> {
  try {
    return await invoke<UpdateCheckResult>('check_for_update')
  } catch (e) {
    warn('check_for_update failed:', e)
    return null
  }
}

async function downloadUpdate(): Promise<string | null> {
  try {
    return await invoke<string>('download_update')
  } catch (e) {
    warn('download_update failed:', e)
    return null
  }
}

async function installReadyUpdate(): Promise<boolean> {
  try {
    await invoke('install_ready_update')
    return true
  } catch (e) {
    warn('install_ready_update failed:', e)
    return false
  }
}

async function isUpdateReadyOnBackend(): Promise<boolean> {
  try {
    return await invoke<boolean>('cmd_is_update_ready')
  } catch {
    return false
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Called once after the application UI is fully loaded.
 *
 * 1. Reads `auto_update` from config.
 * 2. If disabled, does nothing.
 * 3. If enabled, waits 10 seconds then silently checks for/downloads an update.
 * 4. Does NOT block startup.
 */
export async function initializeSilentUpdater(): Promise<void> {
  log('Silent updater initialized')

  try {
    // Load the application config from the backend directly
    const cfg = await invoke<{ auto_update?: boolean }>('cmd_get_config').catch(() => null)
    if (!cfg || !cfg.auto_update) {
      log('Automatic updates disabled')
      setStatus('idle')
      return
    }
  } catch {
    warn('Could not read config for auto_update setting, assuming enabled')
  }

  // Wait before checking — gives the app time to settle
  const delayMs = import.meta.env.DEV ? 3_000 : 10_000
  log(`Waiting ${delayMs}ms before update check`)

  await new Promise((resolve) => setTimeout(resolve, delayMs))

  void checkAndDownloadSilently().catch((error) => {
    warn('Silent updater background check failed:', error)
  })
}

/**
 * Checks for an update and, if available, downloads it silently.
 *
 * State transitions:
 *   idle -> checking -> downloading -> ready
 *   checking -> error
 *   downloading -> error
 */
export async function checkAndDownloadSilently(): Promise<void> {
  if (_checkInProgress || _downloadInProgress) {
    log('Check or download already in progress, skipping')
    return
  }

  _checkInProgress = true
  setStatus('checking')

  try {
    log('Checking for updates')
    const result = await checkForUpdate()

    if (!result) {
      log('Update check returned no result (network error?)')
      setError('Update check failed')
      _checkInProgress = false
      return
    }

    if (!result.available) {
      log('No update available')
      setStatus('idle')
      _checkInProgress = false
      return
    }

    log(`Update available: ${result.available_version}`)
    _lastAvailableVersion = result.available_version

    _checkInProgress = false
    _downloadInProgress = true
    setStatus('downloading', { availableVersion: result.available_version })

    log('Downloading update')
    const version = await downloadUpdate()

    if (version) {
      log('Update ready')
      setStatus('ready', { availableVersion: version })
    } else {
      log('Download failed')
      setError('Download failed')
    }
  } catch (error) {
    warn('Update check/download failed:', error)
    setError(String(error || 'Unknown error'))
  } finally {
    _checkInProgress = false
    _downloadInProgress = false
  }
}

/**
 * Installs the ready update and restarts the application.
 * Should ONLY be called when the user intentionally exits the application.
 */
export async function installReadyUpdateAndRestart(): Promise<void> {
  if (_installInProgress || _updateShutdownInProgress) {
    log('Install or shutdown already in progress, skipping')
    return
  }

  _installInProgress = true
  _updateShutdownInProgress = true
  setStatus('installing')

  try {
    log('Applying ready update')
    const success = await installReadyUpdate()

    if (success) {
      // The backend calls app.restart(), so this point should not be reached
      // in normal operation. Keep the guard set to prevent loops.
      log('Update installed, application restarting')
    } else {
      log('Update installation failed, exiting normally')
      _updateShutdownInProgress = false
      setError('Installation failed')
      // Exit normally
      await invoke('cmd_exit').catch(() => {})
    }
  } catch (error) {
    warn('Update install failed with exception:', error)
    _updateShutdownInProgress = false
    setError(String(error || 'Unknown error'))
    // Exit normally
    await invoke('cmd_exit').catch(() => {})
  } finally {
    _installInProgress = false
  }
}

/**
 * Checks whether this process owns a ready update.
 * Used by tray exit and close handlers.
 */
export async function isUpdateReady(): Promise<boolean> {
  // First check the reactive store
  const s = get(updateStore)
  if (s.status === 'ready') return true
  // Fall back to a backend check (e.g. after a restart the store is reset
  // but the backend might still have a downloaded update from a previous session)
  const backendReady = await isUpdateReadyOnBackend()
  if (backendReady) {
    setStatus('ready')
    return true
  }
  return false
}

/**
 * Centralized application exit that respects a pending update.
 *
 * Route all “real exit” actions (tray → Quit, keyboard shortcut, etc.)
 * through this function so the update logic runs exactly once.
 */
export async function requestApplicationExit(): Promise<void> {
  log('Application exit requested')

  if (!_updateShutdownInProgress && (await isUpdateReady())) {
    await installReadyUpdateAndRestart()
    return
  }

  log('No ready update, exiting normally')
  await invoke('cmd_exit').catch((e) => warn('cmd_exit failed:', e))
}

/**
 * Sets up the listener for the Rust-side "update-ready-close" event.
 * Called once during app initialization.
 */
export function listenForUpdateCloseEvent(): () => void {
  let unlisten: (() => void) | null = null

  listen<null>('update-ready-close', async () => {
    log('Received update-ready-close event from backend')
    await installReadyUpdateAndRestart()
  })
    .then((fn) => {
      unlisten = fn
    })
    .catch((e) => warn('Failed to listen for update-ready-close:', e))

  return () => {
    if (unlisten) {
      unlisten()
      unlisten = null
    }
  }
}
