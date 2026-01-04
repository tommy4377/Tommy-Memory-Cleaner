<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte'
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { LogicalSize } from '@tauri-apps/api/window'
  import { config, updateConfig } from '../lib/store'
  import type { Config } from '../lib/types'
  import { invoke } from '@tauri-apps/api/core'

  const appWindow = WebviewWindow.getCurrent()
  const dispatch = createEventDispatcher()

  export let title: string = 'Tommy Memory Cleaner'
  export let onClose: (() => void) | null = null

  let cfg: Config | null = null
  let unsub: (() => void) | null = null
  let titlebarHeight = 32
  let borderRadius = 16
  
  onMount(async () => {
    // Get window configuration from backend
    try {
      const windowConfig = await invoke('cmd_get_window_config') as { border_radius: number, titlebar_height: number }
      titlebarHeight = windowConfig.titlebar_height
      borderRadius = windowConfig.border_radius
      
      // Set CSS variables
      document.documentElement.style.setProperty('--titlebar-height', `${titlebarHeight}px`)
      document.documentElement.style.setProperty('--window-border-radius', `${borderRadius}px`)
    } catch (error) {
      console.error('Failed to get window config:', error)
      // Fallback to hardcoded values
      document.documentElement.style.setProperty('--titlebar-height', '32px')
      document.documentElement.style.setProperty('--window-border-radius', '16px')
    }
    
    unsub = config.subscribe((v) => (cfg = v))

    // Applica cursore move alla titlebar con !important per sovrascrivere qualsiasi altro stile
    const applyCursor = () => {
      const theme = document.documentElement.getAttribute('data-theme')
      const cursorUrl =
        theme === 'dark'
          ? 'url(/cursors/dark/sizeall.cur), move'
          : 'url(/cursors/light/sizeall.cur), move'

      const titlebar = document.querySelector('.titlebar') as HTMLElement
      const draggable = document.querySelector('.draggable') as HTMLElement

      if (titlebar) {
        titlebar.style.setProperty('cursor', cursorUrl, 'important')
      }
      if (draggable) {
        draggable.style.setProperty('cursor', cursorUrl, 'important')
      }
    }

    // Applica subito e ripetutamente per assicurarsi che venga applicato
    setTimeout(() => applyCursor(), 50)
    setTimeout(() => applyCursor(), 100)
    setTimeout(() => applyCursor(), 200)

    // Osserva cambiamenti del tema
    const observer = new MutationObserver(() => {
      setTimeout(() => applyCursor(), 50)
    })
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme'],
    })

    // Applica anche su mouseenter per essere sicuri
    const titlebarEl = document.querySelector('.titlebar')
    const draggableEl = document.querySelector('.draggable')
    if (titlebarEl) {
      titlebarEl.addEventListener('mouseenter', applyCursor)
    }
    if (draggableEl) {
      draggableEl.addEventListener('mouseenter', applyCursor)
    }
  })

  onDestroy(() => {
    if (unsub) unsub()
  })

  async function handleClose() {
    if (onClose) {
      onClose()
    } else if (cfg?.minimize_to_tray) {
      // Nascondi la finestra (rimane nascosta dalla taskbar)
      await appWindow.hide()
    } else {
      // Chiudi completamente - questo la rimuoverà anche dalla taskbar
      await appWindow.close()
    }
  }

  async function onMinimize() {
    await appWindow.minimize()
  }

  async function handleDragStart(e: MouseEvent) {
    // Solo se è click sinistro e non su un elemento interattivo (button, input, select)
    const target = e.target as HTMLElement
    if (e.button === 0 && !target.closest('button, input, select, .traffic')) {
      e.preventDefault()
      // Imposta cursore appropriato durante il drag
      document.body.style.cursor = 'move'
      try {
        await appWindow.startDragging()
      } catch (err) {
        console.warn('Failed to start dragging:', err)
      }
    }
  }

  function handleDragEnd() {
    // Ripristina cursore quando finisce il drag
    document.body.style.cursor = ''
  }

  async function toggleCompact() {
    // Nel setup, il pulsante compact non fa nulla
    if (onClose) return

    if (!cfg) return
    const next = !cfg.compact_mode

    await updateConfig({ compact_mode: next })

    if (next) {
      await appWindow.setSize(new LogicalSize(420, 100))
    } else {
      await appWindow.setSize(new LogicalSize(480, 680))
    }

    await appWindow.center()
  }
</script>

<div
  class="titlebar"
  on:mousedown={handleDragStart}
  on:mouseup={handleDragEnd}
  on:mouseleave={handleDragEnd}
  role="toolbar"
  tabindex="0"
>
  <div
    class="draggable"
    on:mousedown={handleDragStart}
    on:mouseup={handleDragEnd}
    on:mouseleave={handleDragEnd}
    role="none"
  >
    <img class="logo" src="/icon.png" alt="Tommy Memory Cleaner" />
    <div class="title">{title}</div>
  </div>
  <div class="controls">
    <button
      class="traffic compact"
      aria-label="Toggle view"
      title={cfg?.compact_mode ? 'Full view' : 'Compact view'}
      on:click={toggleCompact}
    ></button>
    <button class="traffic min" aria-label="Minimize" title="Minimize" on:click={onMinimize}
    ></button>
    <button
      class="traffic close"
      aria-label="Close"
      title={onClose ? 'Close' : cfg?.minimize_to_tray ? 'Minimize to tray' : 'Close'}
      on:click={handleClose}
    ></button>
  </div>
</div>

<style>
  .titlebar {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    user-select: none;
    height: var(--titlebar-height, 32px);
    flex-shrink: 0;
    /* Fixed positioning to fill entire window width */
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    /* Remove margins and padding */
    margin: 0;
    padding: 0;
    border: none;
    box-shadow: none;
    width: 100%;
    overflow: hidden;
    /* Ensure it's on top */
    z-index: 1000;
    /* Create background with pseudo-element for full coverage */
    background: transparent;
  }
  
  .titlebar::before {
    content: '';
    position: absolute;
    top: -10px;
    left: -10px;
    right: -10px;
    bottom: -10px;
    background: var(--card);
    /* No border needed since we're extending beyond edges */
    border: none;
    /* Match border-radius with window for seamless rounded corners */
    border-radius: var(--window-border-radius, 16px) var(--window-border-radius, 16px) 0 0;
    z-index: -1;
    /* Use multiple shadows to cover any possible gaps */
    box-shadow: 
      0 0 0 10px var(--card),
      0 0 0 20px var(--card),
      0 0 0 30px var(--card);
  }

  /* Fix for dark mode border artifacts */
  :global(html[data-theme='dark']) .titlebar::before {
    border-bottom-color: transparent;
  }

  .draggable {
    flex: 1;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 5px;
    cursor: url('/cursors/light/sizeall.cur'), move !important;
    -webkit-app-region: no-drag;
    /* Add padding to account for rounded corners */
    padding: 0 8px 0 16px;
  }

  :global(html[data-theme='dark']) .draggable {
    cursor: url('/cursors/dark/sizeall.cur'), move !important;
  }

  .titlebar {
    cursor: url('/cursors/light/sizeall.cur'), move !important;
    -webkit-app-region: no-drag;
  }

  :global(html[data-theme='dark']) .titlebar {
    cursor: url('/cursors/dark/sizeall.cur'), move !important;
  }

  /* Eccezione per i bottoni traffic - non draggable */
  .titlebar .traffic {
    cursor: url('/cursors/light/hand.cur'), pointer !important;
    -webkit-app-region: no-drag;
  }

  :global(html[data-theme='dark']) .titlebar .traffic {
    cursor: url('/cursors/dark/hand.cur'), pointer !important;
  }

  .logo {
    width: 16px; /* Aumentato da 14px a 18px */
    height: 16px;
    pointer-events: none;
  }

  .title {
    font-weight: 500;
    font-size: 12px; /* Aumentato da 11px a 13px */
    pointer-events: none;
    opacity: 0.9; /* Leggermente aumentato da 0.85 per migliore visibilità */
  }

  .controls {
    display: flex;
    gap: 5px;
    position: absolute; /* CAMBIATO: posizionamento assoluto */
    right: 8px; /* Stessa distanza dal bordo destro come icona/titolo da sinistra */
    top: 0;
    height: 100%;
    align-items: center; /* Centra verticalmente i bottoni */
  }

  .traffic {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: none;
    cursor: url('/cursors/light/hand.cur'), pointer !important;
    padding: 0;
    transition: all 0.2s ease;
  }

  :global(html[data-theme='dark']) .traffic {
    cursor: url('/cursors/dark/hand.cur'), pointer !important;
  }

  .traffic:hover {
    transform: scale(1.15);
  }

  .close {
    background: #ff5f57;
  }
  .min {
    background: #ffbd2e;
  }
  .compact {
    background: #28c840;
  }
</style>
