<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { LogicalSize, type PhysicalSize } from '@tauri-apps/api/window';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import Titlebar from './components/Titlebar.svelte';
  import CompactView from './components/CompactView.svelte';
  import FullView from './components/FullView.svelte';
  import { 
    initApp, 
    cleanupApp, 
    config, 
    loadConfig, 
    saveConfig, 
    isAppInitialized,
    setupConfigListener,
    cleanupConfigListener,
    getSystemConfig
  } from './lib/config';
  import { getConfig } from './lib/api';
  import { setLanguage } from './i18n/index';
  import { memoryInfo } from './lib/api';
  import type { Config } from './lib/types';
  
  // ========== STATE ==========
  const appWindow = WebviewWindow.getCurrent();
  
  let cfg: Config | null = null;
  let isCompact = false;
  let isLoading = true;
  let initError: string | null = null;
  
  // Subscriptions
  let configUnsub: (() => void) | null = null;
  let resizeUnlisten: UnlistenFn | null = null;
  let moveUnlisten: UnlistenFn | null = null;
  
  // Window dimensions
  const WINDOW_SIZES = {
    full: { width: 480, height: 680 },
    compact: { width: 380, height: 90 },
    min: { width: 360, height: 90 },
    max: { width: 480, height: 680 }
  } as const;
  
  // ========== LIFECYCLE ==========
  onMount(async () => {
    try {
      // Inizializza app solo se non già inizializzata
      if (!isAppInitialized()) {
        await initApp();
      }
      
      // Setup window
      await setupWindow();
      
      // Subscribe to config changes
      configUnsub = config.subscribe(async (value) => {
        cfg = value;
        if (value) {
          await handleConfigChange(value);
        }
      });
      
      // Listen for setup-complete event to reload config
      const setupCompleteUnlisten = await listen('setup-complete', async () => {
        // Ricarica la configurazione quando il setup è completato
        if (isAppInitialized()) {
          await initApp();
          // La config verrà aggiornata automaticamente tramite il subscribe sopra
        }
      });
      
      // Listen for config-updated event (emesso dal backend dopo setup)
      window.addEventListener('config-updated', async () => {
        // Ricarica la configurazione e applica tutte le impostazioni
        try {
          const cfg = await getConfig();
          if (cfg) {
            // Applica il tema
            const theme = cfg.theme === 'light' ? 'light' : 'dark';
            document.documentElement.setAttribute('data-theme', theme);
            localStorage.setItem('tmc_theme', theme);
            
            // Applica il colore principale corretto per il tema
            let mainColor: string;
            if (theme === 'light') {
              mainColor = cfg.main_color_hex_light || '#9a8a72';
            } else {
              mainColor = cfg.main_color_hex_dark || '#0a84ff';
            }
            
            const root = document.documentElement;
            root.style.setProperty('--btn-bg', mainColor);
            root.style.setProperty('--bar-fill', mainColor);
            root.style.setProperty('--input-focus', mainColor);
            
            // Applica la lingua
            const validLang = getSafeLanguage(cfg.language);
            await setLanguage(validLang);
            
            // Aggiorna lo store (questo aggiornerà anche always_on_top e altre impostazioni)
            config.set(cfg);
          }
        } catch (error) {
          console.error('Failed to reload config after setup:', error);
          // Fallback: ricarica tutto
          if (isAppInitialized()) {
            await initApp();
          }
        }
      });
      
      // Listen for window resize events
      resizeUnlisten = await listen<PhysicalSize>('tauri://resize', async () => {
        // Handle resize if needed
      });
      
      // Listener per monitor change - centra su nuovo monitor quando necessario
      const monitorUnlisten = await listen('tauri://window-scale-factor-changed', async () => {
        await handleMonitorChange();
      });
      
      isLoading = false;
      
    } catch (error) {
      console.error('Failed to initialize app:', error);
      initError = error instanceof Error ? error.message : 'Unknown error occurred';
      isLoading = false;
    }
  });
  
  onDestroy(() => {
    // FIX #10: Cleanup completo di tutte le risorse
    if (configUnsub) {
      configUnsub();
      configUnsub = null;
    }
    
    if (resizeUnlisten) {
      resizeUnlisten();
      resizeUnlisten = null;
    }
    
    if (moveUnlisten) {
      moveUnlisten();
      moveUnlisten = null;
    }
    
    // Cleanup memory refresh e app state
    stopMemoryRefresh();
    cleanupApp().catch(console.error);
  });
  
  // ========== WINDOW MANAGEMENT ==========
  async function setupWindow() {
    try {
      // Mostra la finestra nella taskbar
      await appWindow.setSkipTaskbar(false);
      
      // Set initial size
      await appWindow.setSize(new LogicalSize(
        WINDOW_SIZES.full.width, 
        WINDOW_SIZES.full.height
      ));
      
      // Center window
      await appWindow.center();
      
      // Set min/max size constraints
      await appWindow.setMinSize(new LogicalSize(
        WINDOW_SIZES.min.width,
        WINDOW_SIZES.min.height
      ));
      
      await appWindow.setMaxSize(new LogicalSize(
        WINDOW_SIZES.max.width,
        WINDOW_SIZES.max.height
      ));
      
      // Focus window
      await appWindow.setFocus();
      
    } catch (error) {
      console.error('Failed to setup window:', error);
    }
  }
  
  async function handleConfigChange(newConfig: Config) {
    const shouldBeCompact = newConfig.compact_mode ?? false;
    
    // Handle compact mode change
    if (shouldBeCompact !== isCompact) {
      isCompact = shouldBeCompact;
      await updateWindowSize(isCompact);
    }
    
    // Handle always on top
    if (newConfig.always_on_top !== undefined) {
      try {
        await appWindow.setAlwaysOnTop(newConfig.always_on_top);
      } catch (error) {
        console.error('Failed to set always on top:', error);
      }
    }
    
    // Handle theme
    if (newConfig.theme) {
      const currentTheme = document.documentElement.getAttribute('data-theme');
      if (currentTheme !== newConfig.theme) {
        document.documentElement.setAttribute('data-theme', newConfig.theme);
      }
    }
  }
  
  async function updateWindowSize(compact: boolean) {
    try {
      const size = compact ? WINDOW_SIZES.compact : WINDOW_SIZES.full;
      
      // Disable resizing temporarily for smooth transition
      await appWindow.setResizable(false);
      
      // Update size
      await appWindow.setSize(new LogicalSize(size.width, size.height));
      
      // Re-center window
      await appWindow.center();
      
      // Re-enable resizing for full view
      if (!compact) {
        await appWindow.setResizable(false);
      }
      
    } catch (error) {
      console.error('Failed to update window size:', error);
    }
  }
  
  // FIX: Handle monitor change - re-center and ensure proper size
  async function handleMonitorChange() {
    try {
      // Get current position and size
      const position = await appWindow.innerPosition();
      const size = await appWindow.innerSize();
      
      // Get monitor scale factor (DPI awareness)
      const scaleFactor = await appWindow.scaleFactor();
      
      // Ensure window is within bounds of current monitor
      // Tauri should handle this, but we ensure proper centering
      await appWindow.center();
      
      // Ensure size is correct for current monitor
      const currentSize = await appWindow.innerSize();
      const expectedSize = isCompact ? WINDOW_SIZES.compact : WINDOW_SIZES.full;
      
      // Check if size needs adjustment (accounting for DPI)
      const logicalWidth = currentSize.width / scaleFactor;
      const logicalHeight = currentSize.height / scaleFactor;
      
      if (Math.abs(logicalWidth - expectedSize.width) > 5 || 
          Math.abs(logicalHeight - expectedSize.height) > 5) {
        // Size mismatch, fix it
        await appWindow.setSize(new LogicalSize(
          expectedSize.width,
          expectedSize.height
        ));
        await appWindow.center();
      }
    } catch (error) {
      console.error('Failed to handle monitor change:', error);
    }
  }
  
  // ========== ERROR RECOVERY ==========
  async function retryInit() {
    isLoading = true;
    initError = null;
    
    try {
      await initApp();
      await setupWindow();
      isLoading = false;
    } catch (error) {
      console.error('Retry failed:', error);
      initError = error instanceof Error ? error.message : 'Retry failed';
      isLoading = false;
    }
  }
  
  // ========== KEYBOARD SHORTCUTS ==========
  async function handleKeydown(event: KeyboardEvent) {
    // Ctrl+R or F5: Refresh memory info
    if ((event.ctrlKey && event.key === 'r') || event.key === 'F5') {
      event.preventDefault();
      memoryInfo().then(mem => memory.set(mem)).catch(console.error);
    }
    
    // ESC: Toggle between compact and full mode (BIDIREZIONALE)
    if (event.key === 'Escape') {
      event.preventDefault();
      // Toggle compact mode
      await updateConfig({ compact_mode: !isCompact });
    }
  }
</script>

<style>
  :global(html), 
  :global(body) {
    margin: 0;
    padding: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: transparent;
    border: none !important;
    outline: none !important;
    box-shadow: none !important;
  }
  
  /* Rimuove eventuali bordi visibili su Windows 10 */
  :global(body) {
    border: none !important;
    outline: none !important;
  }
  
  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "Segoe UI Variable", 
                 Roboto, Oxygen, Ubuntu, Cantarell, "Helvetica Neue", sans-serif;
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
    border-radius: 10px;
    overflow: hidden;
    position: relative;
    animation: fadeIn 0.2s ease;
    /* Assicura che il contenuto copra completamente la finestra su Windows 10 */
    margin: 0;
    padding: 0;
    box-shadow: none;
    border: none;
    outline: none;
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
    to { transform: rotate(360deg); }
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
    box-shadow: 0 2px 8px rgba(0,0,0,0.15);
  }
  
  .retry-button:active {
    transform: translateY(0);
  }
</style>

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
      <button class="retry-button" on:click={retryInit}>
        Retry
      </button>
    </div>
  {:else}
    <!-- Main App -->
    <Titlebar />
    {#if isCompact}
      <CompactView />
    {:else}
      <FullView />
    {/if}
  {/if}
</div>