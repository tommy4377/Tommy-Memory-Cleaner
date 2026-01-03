import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

interface AppInfo {
  name: string;
  version: string;
  versionFull: string;
  company: string;
  copyright: string;
  description: string;
}

// Create a writable store with default values
export const appInfo = writable<AppInfo>({
  name: 'Tommy Memory Cleaner',
  version: '2.5.0',
  versionFull: '2.5.0.0',
  company: 'Tommy437',
  copyright: '© 2025 Tommy437. All rights reserved.',
  description: 'Advanced Memory Optimization Tool for Windows'
});

// Function to load app info from backend
export async function loadAppInfo() {
  try {
    const info = await invoke<AppInfo>('get_app_info');
    appInfo.set(info);
    return info;
  } catch (error) {
    console.error('Failed to load app info:', error);
    return null;
  }
}

// Individual getters
export const appName = writable('Tommy Memory Cleaner');
export const appVersion = writable('2.5.0');
export const companyName = writable('Tommy437');

// Load individual values
export async function loadAppName() {
  try {
    const name = await invoke<string>('get_app_name');
    appName.set(name);
    return name;
  } catch (error) {
    console.error('Failed to load app name:', error);
    return null;
  }
}

export async function loadAppVersion() {
  try {
    const version = await invoke<string>('get_app_version');
    appVersion.set(version);
    return version;
  } catch (error) {
    console.error('Failed to load app version:', error);
    return null;
  }
}

export async function loadCompanyName() {
  try {
    const company = await invoke<string>('get_company_name');
    companyName.set(company);
    return company;
  } catch (error) {
    console.error('Failed to load company name:', error);
    return null;
  }
}
