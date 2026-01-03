<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { config, updateConfig } from '../lib/store'
  import { queueConfigUpdate, debounce } from '../lib/themeManager'
  import { invoke } from '@tauri-apps/api/core'
  import { TrayIcon } from '@tauri-apps/api/tray'
  import ColorPicker from './ColorPicker.svelte'
  import type { Config } from '../lib/types'
  import { t } from '../i18n/index'

  let cfg: Config | null = null
  let unsub: (() => void) | null = null
  
  // Stati locali per il drag
  let isDragging = false
  let pendingUpdates: Partial<Config['tray']> = {}
  let rafId: number | null = null
  let trayIcon: TrayIcon | null = null
  
  // Flag per impedire aggiornamenti backend durante il drag
  let isDraggingFromPicker = false
  
  // Cache per le icone generate
  let iconCache = new Map<string, Uint8Array>()
  
  // Stato per il reset colors
  let isResetting = false
  let resetTimeout: number | null = null
  
  // Funzione throttled con requestAnimationFrame per update visivo immediato
  function throttledUpdate() {
    if (rafId) return
    
    rafId = requestAnimationFrame(async () => {
      rafId = null
      // Aggiorna subito la tray icon per preview
      await updateTrayIcon()
    })
  }
  
  // Genera l'icona della tray con i colori correnti
  async function generateTrayIcon(textColor: string, bgColor: string): Promise<Uint8Array> {
    const cacheKey = `${textColor}-${bgColor}`
    
    if (iconCache.has(cacheKey)) {
      return iconCache.get(cacheKey)!
    }
    
    try {
      const iconData = await invoke('generate_tray_icon', {
        text_color: textColor,
        background_color: bgColor
      }) as Uint8Array
      
      iconCache.set(cacheKey, iconData)
      return iconData
    } catch (error) {
      console.error('Failed to generate tray icon:', error)
      throw error
    }
  }
  
  // Aggiorna la tray icon istantaneamente
  async function updateTrayIcon() {
    if (!cfg || !cfg.tray?.show_mem_usage) return
    
    try {
      // Ottieni la tray icon esistente
      if (!trayIcon) {
        trayIcon = await TrayIcon.getById('main')
      }
      
      if (trayIcon) {
        // Genera e aggiorna l'icona con i colori correnti
        const iconData = await generateTrayIcon(
          cfg.tray.text_color_hex,
          cfg.tray.background_color_hex
        )
        
        await trayIcon.setIcon(iconData)
      }
    } catch (error) {
      console.error('Failed to update tray icon:', error)
    }
  }

  onMount(() => {
    unsub = config.subscribe((v) => (cfg = v))
  })

  onDestroy(() => {
    if (unsub) unsub()
    if (rafId) cancelAnimationFrame(rafId)
    if (resetTimeout) clearTimeout(resetTimeout)
  })

  // Debounce per salvare i colori della tray
  const debouncedTrayColorUpdate = debounce(async (color: string, field: keyof Config['tray']) => {
    if (!cfg) return
    
    const updates: Partial<Config> = {
      tray: {
        ...cfg.tray,
        [field]: color
      }
    }
    
    await updateConfig(updates)
  }, 100)
  
  // Funzione ottimizzata per aggiornare i colori della tray
  function onTrayColorChange(color: string, field: keyof Config['tray']) {
    // Imposta flag per indicare che stiamo draggando
    isDraggingFromPicker = true
    
    // Aggiorna subito visivamente
    pendingUpdates = { ...pendingUpdates, [field]: color }
    throttledUpdate()
    
    // Salva con debounce
    debouncedTrayColorUpdate(color, field)
    
    // Resetta il flag
    setTimeout(() => {
      isDraggingFromPicker = false
    }, 150)
  }

  function updateTray(partial: Partial<Config['tray']>) {
    if (!cfg) return
    
    // Aggiorna subito visivamente
    throttledUpdate()
    
    // Durante il drag, accumula gli aggiornamenti
    pendingUpdates = { ...pendingUpdates, ...partial }
    
    // Se non stiamo draggando, salva subito
    if (!isDragging) {
      queueConfigUpdate({
        tray: { ...cfg.tray, ...partial },
      })
      pendingUpdates = {}
    }
  }
  
  function handlePointerDown() {
    isDragging = true
  }
  
  function handlePointerUp() {
    if (!isDragging) return
    
    isDragging = false
    
    // Salva tutti gli aggiornamenti pendenti
    if (Object.keys(pendingUpdates).length > 0) {
      queueConfigUpdate({
        tray: {
          show_mem_usage: cfg?.tray?.show_mem_usage ?? true,
          text_color_hex: cfg?.tray?.text_color_hex ?? '#ffffff',
          background_color_hex: cfg?.tray?.background_color_hex ?? '#2d8a3d',
          transparent_bg: cfg?.tray?.transparent_bg ?? false,
          warning_level: cfg?.tray?.warning_level ?? 80,
          warning_color_hex: cfg?.tray?.warning_color_hex ?? '#d97706',
          danger_level: cfg?.tray?.danger_level ?? 95,
          danger_color_hex: cfg?.tray?.danger_color_hex ?? '#b91c1c',
          ...pendingUpdates,
        },
      })
      pendingUpdates = {}
    }
    
    // Update finale per assicurarsi che l'icona sia aggiornata
    throttledUpdate()
  }
  
  // Aggiungi listener globali per pointer up
  onMount(() => {
    window.addEventListener('pointerup', handlePointerUp)
    return () => window.removeEventListener('pointerup', handlePointerUp)
  })

  function resetTrayColors() {
    if (!cfg) return
    
    // Se è già in stato di reset, fai un fake click (non fare nulla)
    if (isResetting) {
      return
    }
    
    isResetting = true
    
    // Esegui il reset immediatamente
    setTimeout(() => {
      try {
        // Colori originali 
        const defaultText = '#ffffff'
        const defaultBg = '#2d8a3d' // Verde originale
        const defaultWarning = '#d97706' // Arancione originale
        const defaultDanger = '#b91c1c' // Rosso originale
        
        queueConfigUpdate({
          tray: {
            show_mem_usage: cfg?.tray?.show_mem_usage ?? true,
            text_color_hex: defaultText,
            background_color_hex: defaultBg,
            transparent_bg: cfg?.tray?.transparent_bg ?? false,
            warning_level: cfg?.tray?.warning_level ?? 80,
            warning_color_hex: defaultWarning,
            danger_level: cfg?.tray?.danger_level ?? 95,
            danger_color_hex: defaultDanger,
          },
        })
        
        // Pulisci la cache delle icone forzando la rigenerazione
        iconCache.clear()
        
        // Aggiorna l'icona subito senza delay
        throttledUpdate()
        
      } catch (error) {
        console.error('Error resetting tray colors:', error)
      } finally {
        isResetting = false
      }
    }, 0)
  }
</script>

<div class="group">
  <div class="title">{$t('Tray Icon Settings')}</div>

  <div class="checkbox-row">
    <label for="show-mem-usage">
      <input
        type="checkbox"
        id="show-mem-usage"
        checked={cfg?.tray.show_mem_usage ?? true}
        on:change={(e) => {
          const newValue = e.currentTarget.checked
          updateTray({ show_mem_usage: newValue })
        }}
      />
      {$t('Show memory percentage in tray icon')}
    </label>
  </div>

  <div class="checkbox-row">
    <label for="transparent-bg">
      <input
        type="checkbox"
        id="transparent-bg"
        checked={cfg?.tray.transparent_bg}
        on:change={() => updateTray({ transparent_bg: !cfg?.tray.transparent_bg })}
      />
      {$t('Transparent background (numbers only)')}
    </label>
  </div>

  <div class="color-row">
    <div class="color-item">
      <span class="row-label">{$t('Text')}</span>
      <ColorPicker 
        value={cfg?.tray.text_color_hex}
        on:input={(e) => onTrayColorChange(e.detail.value, 'text_color_hex')}
        on:pointerdown={handlePointerDown}
      />
    </div>
    <div class="color-item">
      <span class="row-label">{$t('Background')}</span>
      <ColorPicker 
        value={cfg?.tray.background_color_hex}
        on:input={(e) => onTrayColorChange(e.detail.value, 'background_color_hex')}
        on:pointerdown={handlePointerDown}
        disabled={cfg?.tray.transparent_bg}
      />
    </div>
  </div>

  <div class="row">
    <span class="row-label">{$t('Warning')}</span>
    <input
      type="range"
      min="50"
      max="95"
      value={cfg?.tray.warning_level}
      on:input={(e) => {
        const newValue = parseInt(e.currentTarget.value, 10)
        pendingUpdates = { ...pendingUpdates, warning_level: newValue }
      }}
      on:pointerdown={handlePointerDown}
    />
    <span class="value-display">
      {isDragging && pendingUpdates.warning_level !== undefined 
        ? pendingUpdates.warning_level 
        : cfg?.tray.warning_level}%
    </span>
    <ColorPicker 
      value={cfg?.tray.warning_color_hex}
      on:input={(e) => onTrayColorChange(e.detail.value, 'warning_color_hex')}
      on:pointerdown={handlePointerDown}
      disabled={cfg?.tray.transparent_bg}
    />
  </div>

  <div class="row">
    <span class="row-label">{$t('Danger')}</span>
    <input
      type="range"
      min="60"
      max="100"
      value={cfg?.tray.danger_level}
      on:input={(e) => {
        const newValue = parseInt(e.currentTarget.value, 10)
        pendingUpdates = { ...pendingUpdates, danger_level: newValue }
      }}
      on:pointerdown={handlePointerDown}
    />
    <span class="value-display">
      {isDragging && pendingUpdates.danger_level !== undefined 
        ? pendingUpdates.danger_level 
        : cfg?.tray.danger_level}%
    </span>
    <ColorPicker 
      value={cfg?.tray.danger_color_hex}
      on:input={(e) => onTrayColorChange(e.detail.value, 'danger_color_hex')}
      on:pointerdown={handlePointerDown}
      disabled={cfg?.tray.transparent_bg}
    />
  </div>

  <div style="margin-top: 12px;">
    <button on:click={resetTrayColors} class="reset-btn">
      {$t('Reset Colors')}
    </button>
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
    gap: 8px;
    margin: 6px 0;
  }

  .row-label {
    min-width: 65px;
    font-size: 11px;
  }

  
  input[type='range'] {
    flex: 1;
    height: 8px;
    -webkit-appearance: none;
    appearance: none;
    background: var(--bar-track);
    border-radius: 8px;
    margin: 0;
    padding: 0;
  }

  input[type='range']::-webkit-slider-track {
    background: var(--bar-track);
    height: 8px;
    border-radius: 8px;
  }

  input[type='range']::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 18px;
    height: 18px;
    background: var(--btn-bg);
    border: 2px solid var(--card);
    border-radius: 50%;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    cursor: pointer;
    transform: translateY(1px);
    transition: all 0.2s ease;
  }

  input[type='range']::-webkit-slider-thumb:hover {
    transform: translateY(1px) scale(1.1);
    box-shadow: 0 3px 6px rgba(0, 0, 0, 0.3);
  }

  input[type='range']::-moz-range-track {
    background: var(--bar-track);
    height: 8px;
    border-radius: 8px;
    border: none;
    width: 100%;
  }

  input[type='range']::-moz-range-thumb {
    width: 18px;
    height: 18px;
    background: var(--btn-bg);
    border: 2px solid var(--card);
    border-radius: 50%;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  input[type='range']::-moz-range-thumb:hover {
    transform: scale(1.1);
    box-shadow: 0 3px 6px rgba(0, 0, 0, 0.3);
  }

  .value-display {
    min-width: 40px;
    text-align: center;
    font-weight: 600;
    font-size: 11px;
    padding: 4px 6px;
    background: var(--input-bg);
    border-radius: 8px;
  }

  label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: url('/cursors/light/hand.cur'), pointer;
    font-size: 12px;
  }

  input[type='checkbox'] {
    width: 18px;
    height: 18px;
    cursor: url('/cursors/light/hand.cur'), pointer !important;
  }

  .checkbox-row {
    margin: 8px 0;
    display: flex;
    align-items: center;
  }

  .color-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 8px 0;
  }

  .color-item {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .color-item span {
    font-size: 11px;
  }

  .reset-btn {
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

  .reset-btn::after {
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

  .reset-btn:hover {
    opacity: 0.9;
    transform: translateY(-1px);
  }
</style>
