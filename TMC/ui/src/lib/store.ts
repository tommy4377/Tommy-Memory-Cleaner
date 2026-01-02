import { writable, get } from 'svelte/store'
import type { Config, MemoryInfo, Profile } from './types'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { setLanguage } from '../i18n/index'
import { cacheTranslationsInBackend } from '../lib/translations'
import { areasForProfile } from '../lib/profiles'
import type { Language } from '../i18n/index'

// ========== TYPES ==========
interface ProgressState {
  value: number
  total: number
  step: string
  running: boolean
}

interface AppState {
  initialized: boolean
  listeners: {
    progress: UnlistenFn | null
    done: UnlistenFn | null
    optimizeNow: UnlistenFn | null
  }
  refreshInterval: number | null
}

// ========== STORES ==========
export const config = writable<Config | null>(null)
export const memory = writable<MemoryInfo | null>(null)
export const progress = writable<ProgressState>({
  value: 0,
  total: 1,
  step: '',
  running: false,
})

// ========== STATE ==========
const appState: AppState = {
  initialized: false,
  listeners: {
    progress: null,
    done: null,
    optimizeNow: null,
  },
  refreshInterval: null,
}

// ========== CONSTANTS ==========
const SUPPORTED_LANGUAGES: Language[] = ['en', 'it', 'es', 'fr', 'pt', 'de', 'ar', 'ja', 'zh']
const MEMORY_REFRESH_INTERVAL = 2000 // 2 seconds

// ========== HELPERS ==========
function isValidLanguage(lang: string): lang is Language {
  return SUPPORTED_LANGUAGES.includes(lang as Language)
}

export function getSafeLanguage(lang: string | undefined): Language {
  if (!lang) return 'en'
  const cleanLang = lang.trim().toLowerCase()

  // Map common variations
  const langMap: Record<string, Language> = {
    en: 'en',
    'en-us': 'en',
    'en-gb': 'en',
    it: 'it',
    'it-it': 'it',
    es: 'es',
    'es-es': 'es',
    'es-mx': 'es',
    fr: 'fr',
    'fr-fr': 'fr',
    pt: 'pt',
    'pt-br': 'pt',
    'pt-pt': 'pt',
    de: 'de',
    'de-de': 'de',
    ar: 'ar',
    'ar-sa': 'ar',
    ja: 'ja',
    'ja-jp': 'ja',
    zh: 'zh',
    'zh-cn': 'zh',
    'zh-tw': 'zh',
  }

  return langMap[cleanLang] || 'en'
}

function parseMemoryAreas(areas: any): number {
  if (typeof areas === 'number') {
    return areas
  }
  if (typeof areas === 'string') {
    const parsed = parseInt(areas, 10)
    return isNaN(parsed) ? 0 : parsed
  }
  return 0
}

// ========== INITIALIZATION ==========
export async function initApp(): Promise<void> {
  // FIX #5: Cleanup before re-initializing to prevent memory leak
  if (appState.initialized) {
    console.warn('App already initialized, cleaning up first...')
    await cleanupApp()
  }

  // FIX #8: Remove hardcoded delay - not necessary
  // If a delay is needed, it should be based on real conditions

  try {
    // Clean up any existing state (double check for safety)
    await cleanupApp()

    // Load configuration
    const { getConfig } = await import('./api')
    const cfg = await getConfig()

    // Validate and fix configuration
    if (cfg) {
      // Fix memory_areas if needed
      cfg.memory_areas = parseMemoryAreas(cfg.memory_areas)

      // Validate language
      const validLang = getSafeLanguage(cfg.language)
      if (validLang !== cfg.language) {
        cfg.language = validLang
        // Save corrected language
        const { saveConfig } = await import('./api')
        await saveConfig({ language: validLang }).catch(() => {})
      }

      // Set language in UI
      await setLanguage(validLang)

      // Cache translations in backend for notifications
      await cacheTranslationsInBackend()

      // Set theme
      const theme = cfg.theme === 'light' ? 'light' : 'dark'
      document.documentElement.setAttribute('data-theme', theme)
      localStorage.setItem('tmc_theme', theme)

      // Apply main color based on theme
      // Priority: main_color_hex_light/dark > custom main_color_hex > default
      let mainColor: string
      if (theme === 'light') {
        if (cfg.main_color_hex_light) {
          mainColor = cfg.main_color_hex_light
        } else if (
          cfg.main_color_hex &&
          cfg.main_color_hex !== '#0a84ff' &&
          cfg.main_color_hex !== '#007aff'
        ) {
          // If main_color_hex is custom (not default dark), use it
          mainColor = cfg.main_color_hex
        } else {
          mainColor = '#9a8a72' // Default light
        }
      } else {
        if (cfg.main_color_hex_dark) {
          mainColor = cfg.main_color_hex_dark
        } else if (
          cfg.main_color_hex &&
          cfg.main_color_hex !== '#9a8a72' &&
          cfg.main_color_hex !== '#007aff'
        ) {
          // If main_color_hex is custom (not default light), use it
          mainColor = cfg.main_color_hex
        } else {
          mainColor = '#0a84ff' // Default dark
        }
      }

      const root = document.documentElement
      root.style.setProperty('--btn-bg', mainColor)
      root.style.setProperty('--bar-fill', mainColor)
      root.style.setProperty('--input-focus', mainColor)

      // Update store
      config.set(cfg)
    }

    // Load initial memory info
    try {
      const { memoryInfo } = await import('./api')
      const mem = await memoryInfo()
      memory.set(mem)
    } catch (error) {
      console.error('Failed to load initial memory info:', error)
    }

    // Setup event listeners
    await setupEventListeners()

    // Start memory refresh
    startMemoryRefresh()

    appState.initialized = true
    console.info('App initialized successfully')
  } catch (error) {
    console.error('Failed to initialize app:', error)
    await cleanupApp()
    throw error
  }
}

async function setupEventListeners(): Promise<void> {
  try {
    // Progress listener
    appState.listeners.progress = await listen('tmc://opt_progress', (event: any) => {
      const payload = event.payload as { value: number; total: number; step: string }
      progress.set({
        value: payload.value,
        total: payload.total,
        step: payload.step,
        running: true,
      })
    })

    // Done listener
    appState.listeners.done = await listen('tmc://opt_done', () => {
      progress.update((p) => ({
        ...p,
        step: 'Done',
        running: false,
        value: p.total,
      }))

      // Reset after delay
      setTimeout(() => {
        progress.set({
          value: 0,
          total: 1,
          step: '',
          running: false,
        })
      }, 2000)

      // Refresh memory after optimization
      refreshMemoryOnce()
    })

    // Optimize now listener
    appState.listeners.optimizeNow = await listen('tmc://optimize_now', async () => {
      try {
        const { getConfig } = await import('./api')
        const currentCfg = await getConfig()
        if (currentCfg) {
          const { optimizeAsync } = await import('./api')
          const { Reason } = await import('./types')
          await optimizeAsync(Reason.Manual, currentCfg.memory_areas)
        }
      } catch (error) {
        console.error('Failed to handle optimize_now event:', error)
      }
    })
  } catch (error) {
    console.error('Failed to setup event listeners:', error)
    throw error
  }
}

// ========== CLEANUP ==========
export async function cleanupApp(): Promise<void> {
  console.info('Cleaning up app...')

  // Remove event listeners
  if (appState.listeners.progress) {
    appState.listeners.progress()
    appState.listeners.progress = null
  }

  if (appState.listeners.done) {
    appState.listeners.done()
    appState.listeners.done = null
  }

  if (appState.listeners.optimizeNow) {
    appState.listeners.optimizeNow()
    appState.listeners.optimizeNow = null
  }

  // Stop memory refresh
  stopMemoryRefresh()

  // Reset state
  appState.initialized = false
}

// ========== CONFIG MANAGEMENT ==========
export async function updateConfig(
  partial: Partial<Config>,
  reRegisterHotkey = false,
): Promise<void> {
  const currentConfig = get(config)

  if (!currentConfig) {
    throw new Error('No config available to update')
  }

  // FIX #6: Don't update store until save is confirmed
  // This avoids race conditions and issues if save fails
  // Create updated config for local calculations
  const updatedConfig = { ...currentConfig, ...partial }

  try {
    // Save to backend BEFORE updating store
    const { saveConfig } = await import('./api')
    await saveConfig(partial)

    // Only after successful save, update store
    config.set(updatedConfig)

    // Apply side effects

    // Language change
    if (partial.language !== undefined) {
      const validLang = getSafeLanguage(partial.language)
      await setLanguage(validLang)

      // If language was corrected, save it
      if (validLang !== partial.language) {
        await saveConfig({ language: validLang })
        config.update((c) => (c ? { ...c, language: validLang } : c))
      }
    }

    // Theme change - apply correct color when theme changes
    // IMPORTANT: maintain custom colors separated by theme
    if (partial.theme !== undefined) {
      const newTheme = partial.theme === 'light' ? 'light' : 'dark'
      document.documentElement.setAttribute('data-theme', newTheme)
      localStorage.setItem('tmc_theme', newTheme)

      // Apply correct color for new theme
      // Priority: main_color_hex_light/dark > custom main_color_hex > default
      let mainColor: string
      if (newTheme === 'light') {
        if (currentConfig.main_color_hex_light) {
          mainColor = currentConfig.main_color_hex_light
        } else if (
          currentConfig.main_color_hex &&
          currentConfig.main_color_hex !== '#0a84ff' &&
          currentConfig.main_color_hex !== '#007aff'
        ) {
          mainColor = currentConfig.main_color_hex
        } else {
          mainColor = '#9a8a72'
        }
      } else {
        if (currentConfig.main_color_hex_dark) {
          mainColor = currentConfig.main_color_hex_dark
        } else if (
          currentConfig.main_color_hex &&
          currentConfig.main_color_hex !== '#9a8a72' &&
          currentConfig.main_color_hex !== '#007aff'
        ) {
          mainColor = currentConfig.main_color_hex
        } else {
          mainColor = '#0a84ff'
        }
      }

      const root = document.documentElement
      root.style.setProperty('--btn-bg', mainColor)
      root.style.setProperty('--bar-fill', mainColor)
      root.style.setProperty('--input-focus', mainColor)
    }

    // Main color change - applica in base al tema corrente
    const currentTheme = document.documentElement.getAttribute('data-theme') || 'dark'

    if (partial.main_color_hex_light !== undefined || partial.main_color_hex_dark !== undefined) {
      const mainColor =
        currentTheme === 'light'
          ? partial.main_color_hex_light ||
            currentConfig.main_color_hex_light ||
            currentConfig.main_color_hex ||
            '#9a8a72'
          : partial.main_color_hex_dark ||
            currentConfig.main_color_hex_dark ||
            currentConfig.main_color_hex ||
            '#0a84ff'

      const root = document.documentElement
      root.style.setProperty('--btn-bg', mainColor)
      root.style.setProperty('--bar-fill', mainColor)
      root.style.setProperty('--input-focus', mainColor)
    }

    // Backward compatibility con main_color_hex
    if (partial.main_color_hex !== undefined) {
      const root = document.documentElement
      root.style.setProperty('--btn-bg', partial.main_color_hex)
      root.style.setProperty('--bar-fill', partial.main_color_hex)
      root.style.setProperty('--input-focus', partial.main_color_hex)
    }

    // Hotkey re-registration
    if (reRegisterHotkey && partial.hotkey !== undefined) {
      try {
        const { registerHotkey } = await import('./api')
        await registerHotkey(partial.hotkey)
      } catch (error) {
        console.error('Failed to register hotkey:', error)
        // Non-critical, don't rollback
      }
    }
  } catch (error) {
    console.error('Failed to save config:', error)

    // Rollback on error
    config.set(currentConfig)

    // Try to reload from backend
    try {
      const { getConfig } = await import('./api')
      const freshConfig = await getConfig()
      config.set(freshConfig)
    } catch (reloadError) {
      console.error('Failed to reload config:', reloadError)
    }

    throw error
  }
}

export async function applyProfile(profile: Profile): Promise<void> {
  const areas = areasForProfile(profile)
  const priority = getProfilePriority(profile)

  await updateConfig({
    profile,
    memory_areas: areas,
    run_priority: priority,
  })
}

function getProfilePriority(profile: Profile): 'Low' | 'Normal' | 'High' {
  switch (profile) {
    case 'Normal':
      return 'Low'
    case 'Balanced':
      return 'Normal'
    case 'Gaming':
      return 'High'
    default:
      return 'Normal'
  }
}

// ========== MEMORY REFRESH ==========

const MEMORY_REFRESH_CRITICAL = 500 // 0.5 seconds when memory is critical
const MEMORY_REFRESH_NORMAL = 2000 // 2 seconds when memory is normal
const MEMORY_REFRESH_LOW = 1000 // 1 second when memory is low
const LOW_MEMORY_THRESHOLD = 80 // 80%
const CRITICAL_MEMORY_THRESHOLD = 90 // 90%

export function startMemoryRefresh(intervalMs: number = MEMORY_REFRESH_INTERVAL): void {
  stopMemoryRefresh()

  const refresh = async () => {
    try {
      const { memoryInfo } = await import('./api')
      const mem = await memoryInfo()
      memory.set(mem)

      // Adaptive refresh: adjust interval based on memory usage
      // DISABILITATO: causa troppi riavvii dell'interval
      /*
      const usagePercent = mem.physical.used.percentage
      let newInterval = intervalMs

      if (usagePercent >= CRITICAL_MEMORY_THRESHOLD) {
        newInterval = MEMORY_REFRESH_CRITICAL
      } else if (usagePercent >= LOW_MEMORY_THRESHOLD) {
        newInterval = MEMORY_REFRESH_LOW
      }

      // Restart with new interval if changed
      if (newInterval !== intervalMs) {
        stopMemoryRefresh()
        appState.refreshInterval = window.setInterval(refresh, newInterval)
        console.debug(`Adaptive refresh: ${newInterval}ms (memory: ${usagePercent.toFixed(1)}%)`)
      }
      */
    } catch (error) {
      if (import.meta.env.DEV) {
        console.error('Failed to refresh memory info:', error)
      }
    }
  }

  // Initial refresh
  refresh()

  // Setup interval
  appState.refreshInterval = window.setInterval(refresh, intervalMs)
}

export function stopMemoryRefresh(): void {
  if (appState.refreshInterval !== null) {
    clearInterval(appState.refreshInterval)
    appState.refreshInterval = null
  }
}

export async function refreshMemoryOnce(): Promise<void> {
  try {
    const { memoryInfo } = await import('./api')
    const mem = await memoryInfo()
    memory.set(mem)
  } catch (error) {
    console.error('Failed to refresh memory:', error)
    throw error
  }
}

// ========== UTILITIES ==========
export function isAppInitialized(): boolean {
  return appState.initialized
}

export function getAppState(): Readonly<AppState> {
  return { ...appState }
}

// ========== AUTO CLEANUP ==========
if (typeof window !== 'undefined') {
  // Cleanup on page unload
  window.addEventListener('beforeunload', () => {
    cleanupApp()
  })

  // Handle visibility changes
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) {
      // App in background: reduce refresh rate
      stopMemoryRefresh()
    } else if (appState.initialized) {
      // App in foreground: restore refresh
      startMemoryRefresh()
    }
  })

  // Debug helpers in development
  if (import.meta.env.DEV) {
    ;(window as any).__TMC_DEBUG = {
      getState: () => appState,
      getConfig: () => get(config),
      getMemory: () => get(memory),
      reinit: async () => {
        await cleanupApp()
        await initApp()
      },
    }
  }
}
