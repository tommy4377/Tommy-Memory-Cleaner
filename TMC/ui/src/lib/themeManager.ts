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
    : config.main_color_hex_dark || config.main_color_hex || '#2f58c1'
  
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
    // Processa solo l'ultimo aggiornamento nella coda
    const latestUpdate = updateQueue[updateQueue.length - 1]
    updateQueue = []
    
    // Aspetta un po' prima di processare per evitare il rate limit
    await new Promise(resolve => setTimeout(resolve, 1000))
    
    // Usa updateConfig senza await per non bloccare
    const { updateConfig } = await import('./store')
    updateConfig(latestUpdate).catch(console.error)
  } finally {
    isProcessingQueue = false
    
    // Se ci sono altri aggiornamenti in coda, processali
    if (updateQueue.length > 0) {
      setTimeout(processUpdateQueue, 1000)
    }
  }
}

export function queueConfigUpdate(update: Partial<Config>) {
  updateQueue.push(update)
  processUpdateQueue()
}
