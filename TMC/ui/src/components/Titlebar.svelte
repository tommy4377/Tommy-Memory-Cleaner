<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte'
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { LogicalSize } from '@tauri-apps/api/window'
  import { config, updateConfig } from '../lib/store'
  import type { Config } from '../lib/types'

  const appWindow = WebviewWindow.getCurrent()
  const dispatch = createEventDispatcher()

  export let title: string = 'Tommy Memory Cleaner'
  export let onClose: (() => void) | null = null

  let cfg: Config | null = null
  let unsub: (() => void) | null = null

  onMount(() => {
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
    padding: 4px 8px;
    background: var(--card);
    border-bottom: 1px solid var(--border);
    user-select: none;
    height: 32px;
    flex-shrink: 0;
    /* CRITICAL FIX: Negative margin per coprire il bordino superiore */
    margin: -1px 0 0 0 !important;
    padding-top: 5px !important; /* Compensa il negative margin */
    padding-bottom: 0 !important;
    /* Assicura che tocchi il bordo superiore */
    position: relative;
    top: 0;
    border: none;
    box-shadow: none;
  }

  /* Fix for dark mode border artifacts */
  :global(html[data-theme='dark']) .titlebar {
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
    width: 16px;
    height: 16px;
    pointer-events: none;
  }

  .title {
    font-weight: 500;
    font-size: 12px;
    pointer-events: none;
  }

  .controls {
    display: flex;
    gap: 5px;
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
