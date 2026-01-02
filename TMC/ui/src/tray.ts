/**
 * Tray menu implementation
 * Handles tray icon menu interactions and translations
 */

import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { listen } from '@tauri-apps/api/event'
import { areasForProfile, areasToString } from './lib/profiles'
import { dict, setLanguage, lang } from './i18n'
import { get } from 'svelte/store'

const win = getCurrentWebviewWindow()

// Expose win globally for Rust inline code
;(window as any).win = win

/** Update translations in the DOM */
function updateTrayTranslations() {
  const translations = get(dict)

  // Translate all elements with data-i18n attribute
  document.querySelectorAll('[data-i18n]').forEach((el) => {
    const key = el.getAttribute('data-i18n')
    if (key && translations[key]) {
      el.textContent = translations[key]
    } else if (key && !translations[key]) {
      // Fallback: show key if translation is missing
      console.warn(`Missing translation for "${key}" in language ${get(lang)}`)
      el.textContent = key
    }
  })
}

/** Setup event listeners for tray menu */
async function setupEventListeners() {
  // Listen for language change events from backend
  await listen('language-changed', async (event: any) => {
    const newLanguage = event.payload
    console.log('Language changed in tray:', newLanguage)
    await setLanguage(newLanguage)
    // Wait for translations to load
    await new Promise((resolve) => setTimeout(resolve, 50))
    updateTrayTranslations()
  })

  // Listen for tray menu open events
  await listen('tray-menu-open', async () => {
    console.log('Tray menu opened, reloading config...')
    await reloadTrayConfig()
  })

  // Listen for configuration change events
  await listen('config-changed', async () => {
    console.log('Config changed, reloading tray config...')
    await reloadTrayConfig()
  })
}

/** Reload tray configuration */
async function reloadTrayConfig() {
  try {
    const config = (await invoke('cmd_get_config')) as any
    document.body.setAttribute('data-theme', config.theme || 'dark')

    // Apply main color to menu items
    const mainColor =
      config.theme === 'light'
        ? config.main_color_hex_light || '#9a8a72'
        : config.main_color_hex_dark || '#0a84ff'
    document.documentElement.style.setProperty('--main-color', mainColor)

    // Set language using i18n system
    await setLanguage(config.language || 'en')

    // Wait for translations to load before updating
    await new Promise((resolve) => setTimeout(resolve, 50))

    // Update translations immediately
    updateTrayTranslations()
  } catch (err: any) {
    console.error('Config reload failed:', err)
  }
}

/** Load initial configuration and setup listeners */
async function loadConfig() {
  try {
    await reloadTrayConfig()

    // Listen for future dictionary changes
    const unsubscribe = dict.subscribe(() => {
      // Wait a tick to ensure DOM is updated
      requestAnimationFrame(() => {
        updateTrayTranslations()
      })
    })
  } catch (err: any) {
    console.error('Config load failed:', err)
  }
}

/** Handle tray menu actions */
async function handleAction(action: string) {
  if (!action) return

  try {
    // Close menu before executing action
    await win.hide()

    // Small delay to ensure menu is closed
    await new Promise((resolve) => setTimeout(resolve, 50))

    // Execute action
    if (action === 'open') {
      await invoke('cmd_show_or_create_window')
    } else if (action === 'optimize') {
      // Read current profile from config and use correct areas
      try {
        const config = (await invoke('cmd_get_config')) as any
        const profile = config.profile || 'Balanced'

        // Use areasForProfile to get correct areas
        const areas = areasForProfile(profile)
        const areasString = areasToString(areas)

        await invoke('cmd_optimize_async', {
          reason: 'Manual',
          areas: areasString,
        })
      } catch (err: any) {
        console.error('Failed to get config for optimization, using default balanced profile:', err)
        // Fallback to balanced if config read fails
        const defaultAreas = areasForProfile('Balanced')
        const defaultAreasString = areasToString(defaultAreas)
        await invoke('cmd_optimize_async', {
          reason: 'Manual',
          areas: defaultAreasString,
        })
      }
    } else if (action === 'exit') {
      await invoke('cmd_exit')
    }
  } catch (err: any) {
    console.error('Action failed:', err)
  } finally {
    // Always ensure menu is closed after action
    win.hide().catch(() => {})
  }
}

/** Setup menu item click handlers */
function setupMenuItems() {
  const items = document.querySelectorAll('.menu-item')
  items.forEach((item) => {
    const action = item.getAttribute('data-action')
    if (action) {
      ;(item as HTMLElement).onclick = (e) => {
        e.preventDefault()
        e.stopPropagation()
        handleAction(action)
      }
    }
  })
}

// Initial setup
setupMenuItems()

// Flag to prevent immediate close during setup
let isInitializing = true
setTimeout(() => {
  isInitializing = false
}, 500)

/** Close the tray menu */
function closeMenu() {
  if (isInitializing) return

  document.body.classList.remove('menu-open')

  // Hide window with retry
  win.hide().catch((err) => {
    console.warn('Failed to hide tray menu window:', err)
    // Retry after short delay
    setTimeout(() => {
      win.hide().catch(() => {})
    }, 100)
  })
}

// Expose closeMenu globally
;(window as any).closeMenu = closeMenu

/** Show the menu */
function showMenu() {
  document.body.classList.add('menu-open')
}

// Expose showMenu globally for backend calls
;(window as any).showMenu = showMenu

// Show menu when window becomes visible
if (!document.hidden) {
  // Small delay to ensure DOM is ready
  setTimeout(() => {
    showMenu()
  }, 50)
}

document.addEventListener('visibilitychange', () => {
  if (!document.hidden) {
    showMenu()
  } else {
    // Close only if window becomes hidden (e.g., alt-tab)
    closeMenu()
  }
})

// Auto-close when window loses focus (click outside)
// Use Tauri API instead of window.addEventListener for better reliability
win.onFocusChanged((event: any) => {
  const isFocused = event.payload
  if (!isFocused && document.body.classList.contains('menu-open')) {
    // Small delay to allow menu item clicks to work
    setTimeout(() => {
      closeMenu()
    }, 100)
  }
})

// Fallback for click on overlay (if present)
document.querySelector('.click-overlay')?.addEventListener('click', () => {
  if (document.body.classList.contains('menu-open')) {
    win.hide()
  }
})

// Handle clicks outside menu container - only way to close menu
document.addEventListener('click', (e) => {
  const menuContainer = document.querySelector('.menu-container')
  const clickOverlay = document.getElementById('click-overlay')

  // If click is on overlay (outside menu), close menu
  if (clickOverlay && e.target === clickOverlay) {
    if (document.body.classList.contains('menu-open')) {
      closeMenu()
    }
    return
  }

  // If click is outside menu container, close it
  if (menuContainer && !menuContainer.contains(e.target as Node)) {
    if (document.body.classList.contains('menu-open')) {
      closeMenu()
    }
  }
})

// Close on ESC key
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') {
    closeMenu()
  }
})

// Expose loadConfig globally for external calls
;(window as any).loadConfig = loadConfig

/** Initialize tray */
async function initializeTray() {
  // First register event listeners
  await setupEventListeners()
  // Then load configuration
  await loadConfig()
}

// Initialize on startup
initializeTray()

// Position menu container based on window position
// Window is fullscreen, so we need to position the container
window.addEventListener('load', () => {
  // Get window position (already positioned above tray icon)
  // Menu container should be positioned top-left of window
  const menuContainer = document.querySelector('.menu-container') as HTMLElement
  if (menuContainer) {
    menuContainer.style.position = 'absolute'
    menuContainer.style.top = '0'
    menuContainer.style.left = '0'
  }
})
