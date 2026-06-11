import { writable, type Writable } from 'svelte/store'
import type { Config } from './types'

// Store per il colore corrente (debounced)
export const currentMainColor: Writable<string> = writable('#2f58c1')

// Cache per evitare aggiornamenti non necessari
let lastAppliedTheme: string | null = null
let lastAppliedColors: { light?: string; dark?: string } = {}

// Funzione centralizzata per applicare i colori
export function applyThemeColors(config: Config) {
  if (!config) return

  const theme = config.theme === 'light' ? 'light' : 'dark'
  
  // Evita riapplicazioni non necessarie
  const cacheKey = theme === 'light' ? 'light' : 'dark'
  const colorToApply = theme === 'light' 
    ? config.main_color_hex_light || config.main_color_hex || '#9a8a72'
    : config.main_color_hex_dark || (config.main_color_hex && config.main_color_hex !== '#9a8a72' ? config.main_color_hex : undefined) || '#0a84ff'
  
  if (lastAppliedTheme === theme && lastAppliedColors[cacheKey] === colorToApply) {
    return // Già applicato, salta
  }

  const root = document.documentElement
  
  // Applica il tema
  if (root.getAttribute('data-theme') !== theme) {
    root.setAttribute('data-theme', theme)
  }
  
  // Applica le variabili CSS
  root.style.setProperty('--btn-bg', colorToApply)
  root.style.setProperty('--bar-fill', colorToApply)
  root.style.setProperty('--input-focus', colorToApply)
  
  // Aggiorna la cache
  lastAppliedTheme = theme
  lastAppliedColors[cacheKey] = colorToApply
  
  // Aggiorna lo store reattivo
  currentMainColor.set(colorToApply)
}

// Funzione per resettare i colori
export function resetThemeColors(config: Config) {
  if (!config) return
  
  const theme = config.theme === 'light' ? 'light' : 'dark'
  const defaultColor = theme === 'dark' ? '#2f58c1' : '#9a8a72'
  
  // Resetta entrambi i campi per consistenza
  const updates: Partial<Config> = {
    main_color_hex: defaultColor,
    main_color_hex_light: theme === 'light' ? defaultColor : config.main_color_hex_light,
    main_color_hex_dark: theme === 'dark' ? defaultColor : config.main_color_hex_dark
  }
  
  return updates
}

// Debounce utility con gestione della coda
let updateQueue: Partial<Config>[] = []
let isProcessingQueue = false

export function debounce<T extends (...args: any[]) => any>(
  func: T,
  wait: number
): (...args: Parameters<T>) => void {
  let timeout: NodeJS.Timeout
  
  return (...args: Parameters<T>) => {
    clearTimeout(timeout)
    timeout = setTimeout(() => func(...args), wait)
  }
}

// Sistema di coda per gestire il rate limiting
async function processUpdateQueue() {
  if (isProcessingQueue || updateQueue.length === 0) return
  
  isProcessingQueue = true
  
  try {
    // Wait first to coalesce rapid updates into a single debounce window.
    // This ensures all updates arriving in quick succession are accumulated
    // in the queue before we merge them.
    await new Promise(resolve => setTimeout(resolve, 1000))
    
    // After the debounce wait, drain the queue and MERGE all pending updates.
    // Previously only the last entry was taken, which silently dropped
    // non-overlapping partial updates (e.g., [{theme:'light'}, {language:'it'}]
    // would lose the theme change). Now we merge all entries so no data is lost.
    if (updateQueue.length > 0) {
      const mergedUpdate: Partial<Config> = Object.assign({}, ...updateQueue)
      updateQueue = []
      
      const { updateConfig } = await import('./store')
      try {
        await updateConfig(mergedUpdate)
      } catch (error) {
        console.error('Queue: config update failed, reloading from backend:', error)
        // The save failed — reload the authoritative config from the backend
        // to ensure the UI store is resynchronized with persisted state.
        try {
          const { getConfig } = await import('./api')
          const { config } = await import('./store')
          const freshConfig = await getConfig()
          config.set(freshConfig)
        } catch (reloadError) {
          console.error('Queue: failed to reload config from backend:', reloadError)
        }
      }
    }
  } finally {
    isProcessingQueue = false
    
    // If new updates arrived during the save operation, schedule another
    // processing round to ensure no final value is silently dropped.
    if (updateQueue.length > 0) {
      setTimeout(processUpdateQueue, 0)
    }
  }
}

export function queueConfigUpdate(update: Partial<Config>) {
  updateQueue.push(update)
  processUpdateQueue()
}
