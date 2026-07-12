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
      // Fallback to hardcoded values. 12px matches the Windows 10 clip region
      // (CORNER_RADIUS_PX in system/window.rs); on Windows 11 this merely
      // rounds the content inside the DWM frame, which is cosmetic and safe.
      document.documentElement.style.setProperty('--titlebar-height', '32px')
      document.documentElement.style.setProperty('--window-border-radius', '12px')
    }
    
    unsub = config.subscribe((v) => (cfg = v))

    // Apply the move cursor to the titlebar with !important to override any other style
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

    // Apply immediately and repeatedly to make sure it sticks
    setTimeout(() => applyCursor(), 50)
    setTimeout(() => applyCursor(), 100)
    setTimeout(() => applyCursor(), 200)

    // Watch for theme changes
    const observer = new MutationObserver(() => {
      setTimeout(() => applyCursor(), 50)
    })
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme'],
    })

    // Also apply on mouseenter, to be safe
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
      // Hide the window (it stays hidden from the taskbar)
      await appWindow.hide()
    } else {
      // Close completely - this also removes it from the taskbar
      await appWindow.close()
    }
  }

  async function onMinimize() {
    await appWindow.minimize()
  }

  // ── Manual window dragging ──────────────────────────────────────────────
  // The titlebar deliberately does NOT use `data-tauri-drag-region`: the
  // attribute makes wry's injected script call the native drag on every
  // mousedown, which would race the handlers below (double drag initiation)
  // and reintroduce the Windows "window stuck to cursor" bug: if the OS drag
  // loop starts on a quick click, the release can be lost and the window
  // keeps following the mouse (tauri-apps/tauri#10767).
  //
  // Instead, mousedown only *arms* a drag; `startDragging()` is called once
  // the pointer moves a few pixels with the left button still held. A click
  // that never moves (including each press of a double-click) therefore never
  // enters the OS drag loop and cannot get stuck.
  let dragPending = false
  let dragStartX = 0
  let dragStartY = 0
  const DRAG_THRESHOLD_PX = 4

  function isInteractiveTarget(target: EventTarget | null): boolean {
    return !!(target as HTMLElement | null)?.closest('button, input, select, .traffic')
  }

  function handleDragStart(e: MouseEvent) {
    // Only a single left-click on a non-interactive area arms the drag
    if (e.button !== 0 || isInteractiveTarget(e.target)) return
    e.preventDefault()
    if (e.detail >= 2) {
      // Second press of a double-click: handled by dblclick (toggleMaximize),
      // must never start a drag
      dragPending = false
      return
    }
    dragPending = true
    dragStartX = e.clientX
    dragStartY = e.clientY
  }

  async function handleDragMove(e: MouseEvent) {
    if (!dragPending) return
    if (e.buttons !== 1) {
      // Left button no longer held (release was lost) — disarm
      dragPending = false
      return
    }
    const dx = Math.abs(e.clientX - dragStartX)
    const dy = Math.abs(e.clientY - dragStartY)
    if (dx < DRAG_THRESHOLD_PX && dy < DRAG_THRESHOLD_PX) return

    dragPending = false
    try {
      // A maximized window must not be dragged
      if (await appWindow.isMaximized()) return
      document.body.style.cursor = 'move'
      await appWindow.startDragging()
    } catch (err) {
      console.warn('Failed to start dragging:', err)
    } finally {
      document.body.style.cursor = ''
    }
  }

  function handleDragEnd() {
    // Disarm any pending drag and restore the cursor
    dragPending = false
    document.body.style.cursor = ''
  }

  function handleTitlebarDblClick(e: MouseEvent) {
    if (isInteractiveTarget(e.target)) return
    // This is a fixed-size window (500x700 / compact): double-click must NOT
    // maximize it — `resizable: false` does not block programmatic maximize,
    // and toggleMaximize() here visibly blew the window up. The handler only
    // makes sure no armed drag can fire from the double-click.
    dragPending = false
  }

  let isTransitioning = false

  async function toggleCompact() {
    // During setup, the compact button does nothing
    if (onClose) return

    if (!cfg) return

    // Prevent spamming while the transition is in progress
    if (isTransitioning) return
    
    isTransitioning = true
    const next = !cfg.compact_mode

    try {
      // Resize IMMEDIATELY, before updating the config
      if (next) {
        await appWindow.setSize(new LogicalSize(420, 100))
      } else {
        await appWindow.setSize(new LogicalSize(500, 700))
      }

      // Update the config after changing the size
      await updateConfig({ compact_mode: next })

      // No corner reapplication after resize:
      // Win11 rounding is a persistent DWM attribute; Win10 rounding is CSS
      // and follows the new window size on the very same frame.

      // Do NOT center the window, to avoid issues
      // await appWindow.center()
    } catch (error) {
      console.error('Error during toggle:', error)
    } finally {
      // Reset the flag after a short delay to prevent spamming
      setTimeout(() => {
        isTransitioning = false
      }, 100)
    }
  }
</script>

<div
  class="titlebar"
  on:mousedown={handleDragStart}
  on:mousemove={handleDragMove}
  on:mouseup={handleDragEnd}
  on:mouseleave={handleDragEnd}
  on:dblclick={handleTitlebarDblClick}
  role="toolbar"
  tabindex="0"
>
  <!-- Mouse events bubble to the .titlebar handlers above; attaching them
       here too would double-fire (a double toggleMaximize cancels itself) -->
  <div class="draggable" role="none">
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

  /* Exception for the traffic buttons - not draggable */
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
    opacity: 0.9; /* Slightly raised from 0.85 for better visibility */
  }

  .controls {
    display: flex;
    gap: 5px;
    position: absolute; /* CHANGED: absolute positioning */
    right: 16px; /* Perfect symmetry with left side (16px) */
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
