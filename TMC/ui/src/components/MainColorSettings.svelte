<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { config, updateConfig } from '../lib/store'
  import { debounce, currentMainColor } from '../lib/themeManager'
  import type { Config } from '../lib/types'
  import ColorPicker from './ColorPicker.svelte'
  import { t } from '../i18n/index'

  let cfg: Config | null = null
  let unsub: (() => void) | null = null
  
  // Local value for the color input
  let localColor = '#2f58c1'

  // Reset state
  let isResetting = false

  // Flag that blocks backend updates while a drag is in progress
  let isDraggingFromPicker = false

  // Drag system state, same approach as the tray settings
  let isDragging = false
  let pendingColor: string | null = null

  // More responsive debounce to avoid rate limiting
  const debouncedColorChange = debounce(async (color: string) => {
    if (!cfg) return

    const theme = cfg.theme === 'light' ? 'light' : 'dark'

    // Use updateConfig directly
    const updates: Partial<Config> = theme === 'light'
      ? { main_color_hex_light: color }
      : { main_color_hex_dark: color }

    await updateConfig(updates)
  }, 100) // Reduced from 300ms to 100ms for smoother feedback

  onMount(() => {
    unsub = config.subscribe((v) => {
      cfg = v
      if (v) {
        updateLocalColor()
      }
    })
  })

  onDestroy(() => {
    if (unsub) unsub()
  })

  function updateLocalColor() {
    if (!cfg) return
    
    // Skip updates while dragging from the picker!
    if (isDraggingFromPicker) {
      return
    }
    
    const theme = cfg.theme === 'light' ? 'light' : 'dark'
    const newColor = theme === 'light'
      ? cfg.main_color_hex_light || cfg.main_color_hex || '#9a8a72'
      : cfg.main_color_hex_dark || (cfg.main_color_hex && cfg.main_color_hex !== '#9a8a72' ? cfg.main_color_hex : undefined) || '#0a84ff'
    
    if (newColor !== localColor) {
      localColor = newColor
      
      // Apply the CSS variables
      const root = document.documentElement
      root.style.setProperty('--btn-bg', newColor)
      root.style.setProperty('--bar-fill', newColor)
      root.style.setProperty('--input-focus', newColor)
      currentMainColor.set(newColor)
    }
  }

  function onColorChange(e: Event | CustomEvent) {
    // Handle both standard DOM events and custom events from ColorPicker
    let color: string
    
    if ('detail' in e && e.detail) {
      // Custom event from ColorPicker dispatch
      color = e.detail.value
    } else if ('target' in e && e.target) {
      // Standard DOM event from native input
      const target = e.target as HTMLInputElement
      color = target.value
    } else {
      console.error('Unknown event type:', e)
      return
    }
    
    // Mark that a drag from the picker is in progress
    isDraggingFromPicker = true

    // Apply the color right away for immediate feedback (CSS variables)
    const root = document.documentElement
    root.style.setProperty('--btn-bg', color)
    root.style.setProperty('--bar-fill', color)
    root.style.setProperty('--input-focus', color)
    currentMainColor.set(color)
    
    // While dragging, accumulate the color (same approach as the tray)
    pendingColor = color

    // If not dragging, save immediately
    if (!isDragging) {
      debouncedColorChange(color)
    }

    // Reset the flag after a short delay
    setTimeout(() => {
      isDraggingFromPicker = false
    }, 150)
  }

  async function resetColor() {
    if (!cfg) return
    
    // If a reset is already in progress, ignore the click
    if (isResetting) {
      return
    }

    isResetting = true

    // Perform the reset immediately
    setTimeout(() => {
      try {
        // Original accent colors
        const theme = cfg?.theme === 'light' ? 'light' : 'dark'
        const defaultMainColor = theme === 'light' ? '#9a8a72' : '#1363b4'

        // Update the local value right away for instant feedback
        localColor = defaultMainColor

        // Apply the CSS variables right away
        const root = document.documentElement
        root.style.setProperty('--btn-bg', defaultMainColor)
        root.style.setProperty('--bar-fill', defaultMainColor)
        root.style.setProperty('--input-focus', defaultMainColor)
        currentMainColor.set(defaultMainColor)
        
        // Persist to the config
        const updates: Partial<Config> = theme === 'light'
          ? { main_color_hex_light: defaultMainColor }
          : { main_color_hex_dark: defaultMainColor }
        
        console.log('🔄 [RESET COLOR] Resetting to:', updates)
        updateConfig(updates)
        
      } catch (error) {
        console.error('Failed to reset color:', error)
      } finally {
        isResetting = false
      }
    }, 0) // Timeout 0 to run after the current render cycle
  }
  
  function handlePointerDown() {
    isDragging = true
  }
  
  function handlePointerUp() {
    if (!isDragging) return
    
    isDragging = false
    
    // Save the pending color if there is one
    if (pendingColor) {
      const theme = cfg?.theme === 'light' ? 'light' : 'dark'
      const updates: Partial<Config> = theme === 'light'
        ? { main_color_hex_light: pendingColor }
        : { main_color_hex_dark: pendingColor }
      
      updateConfig(updates)
      pendingColor = null
    }
    
    // Reset the picker-drag flag
    isDraggingFromPicker = false
  }

  // Register a global pointer-up listener
  onMount(() => {
    window.addEventListener('pointerup', handlePointerUp)
    return () => window.removeEventListener('pointerup', handlePointerUp)
  })
</script>

<div class="group">
  <div class="title">{$t('Main Color')}</div>

  <div class="row">
    <ColorPicker bind:value={localColor} on:input={onColorChange} on:pointerdown={handlePointerDown} />
    <button on:click={resetColor}>{$t('Reset')}</button>
  </div>

  <div class="hint">
    {$t('This color will be used for buttons, progress bars, and accents throughout the app')}
  </div>
</div>

<style>
  .group {
    background: var(--card);
    border-radius: 12px;
    padding: 10px;
  }

  .title {
    font-weight: 500;
    font-size: 12px;
    margin-bottom: 8px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 6px 0;
  }

  button {
    padding: 6px 14px;
    background: var(--btn-bg);
    color: white;
    border: none;
    border-radius: 10px;
    font-size: 11px;
    cursor: pointer;
    transition: all 0.2s;
    position: relative;
    overflow: hidden;
  }

  button::after {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: linear-gradient(
      135deg,
      transparent 30%,
      rgba(255, 255, 255, 0.1) 50%,
      transparent 70%
    );
    animation: shimmer 2s infinite;
    pointer-events: none;
  }

  @keyframes shimmer {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(100%);
    }
  }

  button:hover {
    opacity: 0.9;
    transform: translateY(-1px);
  }

  .hint {
    font-size: 11px;
    opacity: 0.7;
    margin-top: 6px;
    line-height: 1.4;
  }
</style>
