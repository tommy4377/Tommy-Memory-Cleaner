import { invoke } from '@tauri-apps/api/core';
import type { Areas, MemoryInfo, Reason, Config } from './types';
import { areasToString } from './profiles';

export async function memoryInfo(): Promise<MemoryInfo> {
  return await invoke<MemoryInfo>('cmd_memory_info');
}

export async function getConfig(): Promise<Config> {
  return await invoke<Config>('cmd_get_config');
}

export async function saveConfig(cfg: Partial<Config>): Promise<void> {
  // Invia direttamente l'oggetto parziale
  await invoke('cmd_save_config', { cfgJson: cfg });
}

export async function showNotification(title: string, message: string): Promise<void> {
  await invoke('cmd_show_notification', { title, message });
}

export async function registerHotkey(hotkey: string): Promise<void> {
  await invoke('cmd_register_hotkey', { hotkey });
}

export async function optimizeAsync(reason: Reason, areas: Areas): Promise<void> {
  const areasString = areasToString(areas);
  await invoke('cmd_optimize_async', { reason, areas: areasString });
}

export async function listProcessNames(): Promise<string[]> {
  return await invoke<string[]>('cmd_list_process_names');
}

export async function getCriticalProcesses(): Promise<string[]> {
  return await invoke<string[]>('cmd_get_critical_processes');
}

export async function runOnStartup(enable: boolean): Promise<void> {
  await invoke('cmd_run_on_startup', { enable });
}

export async function setAlwaysOnTop(on: boolean): Promise<void> {
  await invoke('cmd_set_always_on_top', { on });
}

export async function setPriority(priority: 'Low' | 'Normal' | 'High'): Promise<void> {
  await invoke('cmd_set_priority', { priority });
}