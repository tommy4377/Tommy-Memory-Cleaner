<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { LogicalSize, type PhysicalSize } from '@tauri-apps/api/window'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import Titlebar from './components/Titlebar.svelte'

  // Lazy load components for better performance
  let CompactView: any = null
  let FullView: any = null

  // Load components when needed
  async function loadComponents() {
    if (!CompactView) {
      const module = await import('./components/CompactView.svelte')
      CompactView = module.default
    }
    if (!FullView) {
      const module = await import('./components/FullView.svelte')
      FullView = module.default
    }
  }

  import {
    initApp,
    cleanupApp,
    config,
    memory,
    isAppInitialized,
    updateConfig,
    getSafeLanguage,
    stopMemoryRefresh,
  } from './lib/store'
  import { applyThemeColors } from './lib/themeManager'
  import { getConfig, saveConfig } from './lib/api'
  import { setLanguage } from './i18n/index'
  import { memoryInfo } from './lib/api'
  import type { Config } from './lib/types'
  import { invoke } from '@tauri-apps/api/core'

  // ========== STATE ==========
  const appWindow = WebviewWindow.getCurrent()

  let cfg: Config | null = null
  let isCompact = false
  let isLoading = true
  let initError: string | null = null

  // Subscriptions
  let configUnsub: (() => void) | null = null
  let resizeUnlisten: UnlistenFn | null = null
  let moveUnlisten: UnlistenFn | null = null
  
  // Listener per resize
  let handleResize: () => void

  // Window dimensions
  const WINDOW_SIZES = {
    full: { width: 500, height: 700 },
    compact: { width: 420, height: 100 },
    min: { width: 360, height: 90 },
    max: { width: 500, height: 700 },
  } as const

  // ========== LIFECYCLE ==========
  onMount(async () => {
    // Log della dimensione della finestra
    console.log(`Window size: ${window.innerWidth}x${window.innerHeight}px`)
    
    // 1. Leggi la configurazione iniziale
    let currentConfig = await getConfig()
    
    // 2. ALWAYS detect platform on every startup for correct border styling
    // This ensures existing configs get updated if the Windows version changes
    // or if the previous detection was incorrect
    try {
      const platform = await invoke('cmd_get_platform') as string
      const isWindows10 = platform === 'windows-10'
      
      // Only save if changed to avoid unnecessary writes
      if (currentConfig.is_windows_10 !== isWindows10 || !currentConfig.platform_detected) {
        await saveConfig({ 
          platform_detected: true,
          is_windows_10: isWindows10 
        })
        currentConfig = { ...currentConfig, platform_detected: true, is_windows_10: isWindows10 }
        console.log(`Platform updated: ${platform}`)
      } else {
        console.log(`Platform unchanged: ${platform}`)
      }
    } catch (error) {
      console.error('Failed to detect platform:', error)
    }
    
    // Apply theme immediately to avoid flash
    if (currentConfig) {
      applyThemeColors(currentConfig)
    }

    // 3. Setup window CON la configurazione aggiornata
    await setupWindow(currentConfig)
    
    // 4. Initialize app
    await initApp()
    
    // Force correct size on startup to prevent scrollbars
    // This fixes the issue where scrollbar appears after setup
    try {
      const window = WebviewWindow.getCurrent()
      // Only set if not already in compact mode (though usually it starts full)
      if (!currentConfig?.compact_mode) {
        // FIX: Wait for window animations/init to settle
        setTimeout(async () => {
          try {
            await window.setSize(new LogicalSize(500, 700))
          } catch (e) { console.warn('Resize failed:', e) }
        }, 250)
      }
    } catch (e) {
      console.warn('Failed to force window size:', e)
    }

    isLoading = false
    initError = null

    // 5. Subscribe to config changes
    configUnsub = config.subscribe((v) => {
      cfg = v
      if (v) handleConfigChange(v)
    })

    // Apply initial theme
    if (cfg?.theme) {
      applyThemeColors(cfg)
      // Update tray icon with correct theme
      invoke('cmd_update_tray_theme', { theme: cfg.theme })
    }

    // Apply initial language
    if (cfg?.language) {
      setLanguage(cfg.language as 'en' | 'it' | 'es' | 'fr' | 'pt' | 'de' | 'ar' | 'ja')
    }

    // Listener per resize
    handleResize = () => {
      console.log(`Window resized to: ${window.innerWidth}x${window.innerHeight}px`)
    }
    window.addEventListener('resize', handleResize)

    // Listen for setup-complete event to reload config
    const setupCompleteUnlisten = await listen('setup-complete', async () => {
      // Ricarica la configurazione quando il setup è completato
      if (isAppInitialized()) {
        await initApp()
        // La config verrà aggiornata automaticamente tramite il subscribe sopra
      }
    })

    // Listen for window resize events
    resizeUnlisten = await listen<PhysicalSize>('tauri://resize', async () => {
      // Handle resize if needed
    })

    // Listener per monitor change - centra su nuovo monitor quando necessario
    const monitorUnlisten = await listen('tauri://window-scale-factor-changed', async () => {
      await handleMonitorChange()
    })

    isLoading = false
  })

  onDestroy(() => {
    // FIX #10: Cleanup completo di tutte le risorse
    if (configUnsub) {
      configUnsub()
      configUnsub = null
    }

    if (resizeUnlisten) {
      resizeUnlisten()
      resizeUnlisten = null
    }

    if (moveUnlisten) {
      moveUnlisten()
      moveUnlisten = null
    }
    
    // Rimuovi il listener per resize
    window.removeEventListener('resize', handleResize)

    // Cleanup memory refresh e app state
    stopMemoryRefresh()
    cleanupApp().catch(console.error)
  })

  // ========== WINDOW MANAGEMENT ==========
  async function setupWindow(config: Config) {
    try {
      // Mostra la finestra nella taskbar
      await appWindow.setSkipTaskbar(false)

      // Set initial size based on config
      const startCompact = config?.compact_mode ?? false
      const size = startCompact ? WINDOW_SIZES.compact : WINDOW_SIZES.full
      await appWindow.setSize(new LogicalSize(size.width, size.height))

      // Center window
      await appWindow.center()

      // Set min/max size constraints
      await appWindow.setMinSize(new LogicalSize(WINDOW_SIZES.min.width, WINDOW_SIZES.min.height))
      await appWindow.setMaxSize(new LogicalSize(WINDOW_SIZES.max.width, WINDOW_SIZES.max.height))

      // Focus window
      await appWindow.setFocus()
    } catch (error) {
      console.error('Failed to setup window:', error)
    }
  }

  async function handleConfigChange(newConfig: Config) {
    const shouldBeCompact = newConfig.compact_mode ?? false

    // Handle compact mode change
    if (shouldBeCompact !== isCompact) {
      isCompact = shouldBeCompact
      await updateWindowSize(isCompact)
    }

    // Handle always on top
    if (newConfig.always_on_top !== undefined) {
      try {
        await appWindow.setAlwaysOnTop(newConfig.always_on_top)
      } catch (error) {
        console.error('Failed to set always on top:', error)
      }
    }
  }

  async function updateWindowSize(compact: boolean) {
    try {
      const size = compact ? WINDOW_SIZES.compact : WINDOW_SIZES.full

      // Get current position BEFORE resizing
      const currentPos = await appWindow.innerPosition()

      // Disable resizing temporarily for smooth transition
      await appWindow.setResizable(false)

      // Update size - use current position, don't recenter
      await appWindow.setSize(new LogicalSize(size.width, size.height))

      // Keep window at same top-left position (don't center on every toggle)
      // This prevents the window from jumping around the screen
      // Only adjust if going to compact mode (shrink at top)
      // or expanding (keep same top position)

      // Re-enable resizing for full view
      if (!compact) {
        await appWindow.setResizable(false)
      }

      // FIX: Re-apply rounded corners on Windows 10 to ensure border is correct
      // This helps with "glitchy" transitions
      try {
        await invoke('cmd_apply_rounded_corners')
      } catch (e) {
        console.error('Failed to re-apply rounded corners:', e)
      }
    } catch (error) {
      console.error('Failed to update window size:', error)
    }
  }

  // FIX: Handle monitor change - re-center and ensure proper size
  async function handleMonitorChange() {
    try {
      // Get current position and size
      const position = await appWindow.innerPosition()
      const size = await appWindow.innerSize()

      // Get monitor scale factor (DPI awareness)
      const scaleFactor = await appWindow.scaleFactor()

      // Ensure window is within bounds of current monitor
      // Tauri should handle this, but we ensure proper centering
      await appWindow.center()

      // Ensure size is correct for current monitor
      const currentSize = await appWindow.innerSize()
      const expectedSize = isCompact ? WINDOW_SIZES.compact : WINDOW_SIZES.full

      // Check if size needs adjustment (accounting for DPI)
      const logicalWidth = currentSize.width / scaleFactor
      const logicalHeight = currentSize.height / scaleFactor

      if (
        Math.abs(logicalWidth - expectedSize.width) > 5 ||
        Math.abs(logicalHeight - expectedSize.height) > 5
      ) {
        // Size mismatch, fix it
        await appWindow.setSize(new LogicalSize(expectedSize.width, expectedSize.height))
        await appWindow.center()
      }
    } catch (error) {
      console.error('Failed to handle monitor change:', error)
    }
  }

  // ========== ERROR RECOVERY ==========
  async function retryInit() {
    onMount(async () => {
    // Log della dimensione della finestra
    console.log(`Window size: ${window.innerWidth}x${window.innerHeight}px`)
    
    // Inizializzazione app
    try {
      await initApp()
      isLoading = false
      await setupWindow(await getConfig())
      isLoading = false
    } catch (error) {
      console.error('Retry failed:', error)
      initError = error instanceof Error ? error.message : 'Retry failed'
      isLoading = false
    }
  })
  }

  // ========== KEYBOARD SHORTCUTS ==========
  async function handleKeydown(event: KeyboardEvent) {
    // Ctrl+R or F5: Refresh memory info
    if ((event.ctrlKey && event.key === 'r') || event.key === 'F5') {
      event.preventDefault()
      memoryInfo()
        .then((mem) => memory.set(mem))
        .catch(console.error)
    }

    // ESC: Toggle between compact and full mode (BIDIREZIONALE)
    if (event.key === 'Escape') {
      event.preventDefault()
      // Toggle compact mode
      await updateConfig({ compact_mode: !isCompact })
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="app">
  {#if isLoading}
    <!-- Loading State -->
    <div class="loading">
      <div class="loading-spinner"></div>
      <div class="loading-text">Initializing Tommy Memory Cleaner...</div>
    </div>
  {:else if initError}
    <!-- Error State -->
    <div class="error">
      <div class="error-icon">⚠️</div>
      <div class="error-title">Failed to Initialize</div>
      <div class="error-message">{initError}</div>
      <button class="retry-button" on:click={retryInit}> Retry </button>
    </div>
  {:else}
    <!-- Main App -->
    <Titlebar />
    {#if isCompact}
      {#await loadComponents() then}
        <svelte:component this={CompactView} />
      {:catch error}
        <div class="error">
          <div class="error-message">Failed to load CompactView: {error}</div>
        </div>
      {/await}
    {:else}
      {#await loadComponents() then}
        <svelte:component this={FullView} />
      {:catch error}
        <div class="error">
          <div class="error-message">Failed to load FullView: {error}</div>
        </div>
      {/await}
    {/if}
  {/if}
</div>

<style>
  :global(html),
  :global(body),
  :global(#app) {
    margin: 0;
    padding: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--bg);
    border: none !important;
    outline: none !important;
    box-shadow: none !important;
    /* DPI-aware anti-aliasing */
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    image-rendering: crisp-edges;
    border-radius: var(--window-border-radius, 16px);  /* Synced with backend */
    /* Ensure no positioning issues */
    position: relative;
    top: 0;
    left: 0;
  }

  /* Rimuove eventuali bordi visibili su Windows 10 */
  :global(body) {
    border: none !important;
    outline: none !important;
  }

  :global(body) {
    font-family:
      -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Segoe UI Variable', Roboto, Oxygen, Ubuntu,
      Cantarell, 'Helvetica Neue', sans-serif;
    font-size: 12px;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }

  :global(*) {
    box-sizing: border-box;
  }

  .app {
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--fg);
    overflow: hidden;
    position: relative;
    animation: fadeIn 0.2s ease;
    /* Match border-radius with Rust window.rs for seamless rounded corners */
    border-radius: var(--window-border-radius, 16px);
    /* Ensure content stays within rounded bounds */
    border: 1px solid transparent;
    /* Remove any margins to ensure full window coverage */
    margin: 0;
    padding: 0;
    /* Add padding-top to account for fixed titlebar using CSS variable */
    padding-top: var(--titlebar-height, 32px);
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: scale(0.98);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .loading,
  .error {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 32px;
    text-align: center;
  }

  .loading-spinner {
    width: 48px;
    height: 48px;
    border: 3px solid var(--bar-track);
    border-top-color: var(--btn-bg);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 16px;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .loading-text {
    font-size: 14px;
    opacity: 0.8;
  }

  .error-icon {
    font-size: 48px;
    margin-bottom: 16px;
    color: #ff5f57;
  }

  .error-title {
    font-size: 16px;
    font-weight: 600;
    margin-bottom: 8px;
  }

  .error-message {
    font-size: 13px;
    opacity: 0.8;
    margin-bottom: 16px;
    max-width: 300px;
  }

  .retry-button {
    padding: 8px 24px;
    background: var(--btn-bg);
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    transition: all 0.2s;
  }

  .retry-button:hover {
    transform: translateY(-1px);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
  }

  .retry-button:active {
    transform: translateY(0);
  }
</style>
