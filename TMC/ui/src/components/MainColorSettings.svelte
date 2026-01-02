<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { config, updateConfig } from '../lib/store'
  import { debounce, currentMainColor } from '../lib/themeManager'
  import type { Config } from '../lib/types'
  import ColorPicker from './ColorPicker.svelte'
  import { t } from '../i18n/index'

  let cfg: Config | null = null
  let unsub: (() => void) | null = null
  
  // Valore locale per l'input color
  let localColor = '#2f58c1'
  
  // Stato per il reset
  let isResetting = false
  
  // Flag per impedire aggiornamenti backend durante il drag
  let isDraggingFromPicker = false
  
  // Stati per il drag system come nella tray
  let isDragging = false
  let pendingColor: string | null = null
  
  // Debounce più reattivo per evitare rate limiting
  const debouncedColorChange = debounce(async (color: string) => {
    if (!cfg) return
    
    const theme = cfg.theme === 'light' ? 'light' : 'dark'
    
    // Usa updateConfig diretto
    const updates: Partial<Config> = theme === 'light'
      ? { main_color_hex_light: color }
      : { main_color_hex_dark: color }
    
    await updateConfig(updates)
  }, 100) // Ridotto da 300ms a 100ms per più fluidità

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
    
    // Se stiamo draggando dal picker, non aggiornare!
    if (isDraggingFromPicker) {
      return
    }
    
    const theme = cfg.theme === 'light' ? 'light' : 'dark'
    const newColor = theme === 'light'
      ? cfg.main_color_hex_light || cfg.main_color_hex || '#9a8a72'
      : cfg.main_color_hex_dark || cfg.main_color_hex || '#1363b4'
    
    if (newColor !== localColor) {
      localColor = newColor
      
      // Applica le CSS variables
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
    
    // Imposta flag per indicare che stiamo draggendo dal picker
    isDraggingFromPicker = true
    
    // Applica subito il colore per feedback immediato (CSS variables)
    const root = document.documentElement
    root.style.setProperty('--btn-bg', color)
    root.style.setProperty('--bar-fill', color)
    root.style.setProperty('--input-focus', color)
    currentMainColor.set(color)
    
    // Durante il drag, accumula il colore come nella tray
    pendingColor = color
    
    // Se non stiamo draggando, salva subito
    if (!isDragging) {
      debouncedColorChange(color)
    }
    
    // Resetta il flag dopo un breve ritardo
    setTimeout(() => {
      isDraggingFromPicker = false
    }, 150)
  }

  async function resetColor() {
    if (!cfg) return
    
    // Se è già in stato di reset, fai un fake click (non fare nulla)
    if (isResetting) {
      return
    }
    
    isResetting = true
    
    // Esegui il reset immediatamente
    setTimeout(() => {
      try {
        // Colori originali del main
        const theme = cfg.theme === 'light' ? 'light' : 'dark'
        const defaultMainColor = theme === 'light' ? '#9a8a72' : '#1363b4'
        
        // Aggiorna subito il locale per feedback istantaneo
        localColor = defaultMainColor
        
        // Applica subito le CSS variables
        const root = document.documentElement
        root.style.setProperty('--btn-bg', defaultMainColor)
        root.style.setProperty('--bar-fill', defaultMainColor)
        root.style.setProperty('--input-focus', defaultMainColor)
        currentMainColor.set(defaultMainColor)
        
        // Salva nel config
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
    }, 0) // Timeout 0 per eseguire dopo il ciclo di render corrente
  }
  
  function handlePointerDown() {
    isDragging = true
  }
  
  function handlePointerUp() {
    if (!isDragging) return
    
    isDragging = false
    
    // Salva il colore pendente se esiste
    if (pendingColor) {
      const theme = cfg.theme === 'light' ? 'light' : 'dark'
      const updates: Partial<Config> = theme === 'light'
        ? { main_color_hex_light: pendingColor }
        : { main_color_hex_dark: pendingColor }
      
      updateConfig(updates)
      pendingColor = null
    }
    
    // Resetta il flag di dragging dal picker
    isDraggingFromPicker = false
  }
  
  // Aggiungi listener globali per pointer up
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
