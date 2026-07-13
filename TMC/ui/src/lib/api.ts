/**
 * API layer for Tauri backend communication
 */

import { invoke } from '@tauri-apps/api/core'
import type { Areas, MemoryInfo, Reason, Config } from './types'
import { areasToString } from './profiles'

/** Get current memory usage information */
export async function memoryInfo(): Promise<MemoryInfo> {
  return await invoke<MemoryInfo>('cmd_memory_info')
}

/** Get current application configuration */
export async function getConfig(): Promise<Config> {
  return await invoke<Config>('cmd_get_config')
}

/** Save configuration changes */
export async function saveConfig(cfg: Partial<Config>): Promise<void> {
  await invoke('cmd_save_config', { cfgJson: cfg })
}

/** Show native system notification */
export async function showNotification(title: string, message: string): Promise<void> {
  await invoke('cmd_show_notification', { title, message })
}

/** Register global hotkey */
export async function registerHotkey(hotkey: string): Promise<void> {
  await invoke('cmd_register_hotkey', { hotkey })
}

/** Execute memory optimization */
export async function optimizeAsync(reason: Reason, areas: Areas): Promise<void> {
  const areasString = areasToString(areas)
  await invoke('cmd_optimize_async', { reason, areas: areasString })
}

/** Get list of running process names */
export async function listProcessNames(): Promise<string[]> {
  return await invoke<string[]>('cmd_list_process_names')
}

/** Get list of critical system processes */
export async function getCriticalProcesses(): Promise<string[]> {
  return await invoke<string[]>('cmd_get_critical_processes')
}

/** Configure application to run on startup */
export async function runOnStartup(enable: boolean): Promise<void> {
  await invoke('cmd_run_on_startup', { enable })
}

/** Set window always on top state */
export async function setAlwaysOnTop(on: boolean): Promise<void> {
  await invoke('cmd_set_always_on_top', { on })
}

/** Set process priority */
export async function setPriority(priority: 'Low' | 'Normal' | 'High'): Promise<void> {
  await invoke('cmd_set_priority', { priority })
}

// ========== UPDATER ==========

export interface UpdateCheckResult {
  available: boolean
  current_version: string
  available_version: string | null
}

/** Check for an available update */
export async function checkForUpdate(): Promise<UpdateCheckResult> {
  return await invoke<UpdateCheckResult>('check_for_update')
}

/** Download an available update without installing */
export async function downloadUpdate(): Promise<string> {
  return await invoke<string>('download_update')
}

/** Install the ready update and restart the app */
export async function installReadyUpdate(): Promise<void> {
  await invoke('install_ready_update')
}

/** Check if a downloaded update is ready to install */
export async function isUpdateReadyBackend(): Promise<boolean> {
  return await invoke<boolean>('cmd_is_update_ready')
}

/** Get the version of the ready update */
export async function getReadyVersion(): Promise<string | null> {
  return await invoke<string | null>('cmd_get_ready_version')
}
