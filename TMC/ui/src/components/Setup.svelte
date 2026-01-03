<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { listen } from '@tauri-apps/api/event'
  import Titlebar from './Titlebar.svelte'
  import CustomSelect from './CustomSelect.svelte'
  import { t, setLanguage } from '../i18n/index'

  let runOnStartup = true
  let theme = 'dark'
  let alwaysOnTop = true // Default: sempre in primo piano
  let showNotifications = true
  let language = 'en'
  let isLoading = false

  const languageOptions = [
    { value: 'en', label: 'English' },
    { value: 'it', label: 'Italiano' },
    { value: 'es', label: 'Español' },
    { value: 'fr', label: 'Français' },
    { value: 'pt', label: 'Português' },
    { value: 'de', label: 'Deutsch' },
    { value: 'ar', label: 'العربية' },
    { value: 'ja', label: '日本語' },
    { value: 'zh', label: '中文' },
  ]

  $: themeOptions = [
    { value: 'light', label: $t('Light') },
    { value: 'dark', label: $t('Dark') },
  ]

  let unlistenSetupComplete: (() => void) | null = null

  onMount(async () => {
    // Rileva tema e lingua dal sistema
    try {
      const systemTheme = await invoke<string>('cmd_get_system_theme')
      if (systemTheme) {
        theme = systemTheme
        document.documentElement.setAttribute('data-theme', theme)
      }
    } catch (error) {
      console.error('Failed to get system theme:', error)
    }

    try {
      const systemLang = await invoke<string>('cmd_get_system_language')
      if (systemLang) {
        language = systemLang
        setLanguage(systemLang as any)
      }
    } catch (error) {
      console.error('Failed to get system language:', error)
    }

    // Applica il tema iniziale
    document.documentElement.setAttribute('data-theme', theme)

    // Ascolta evento per chiudere la finestra (backup se il backend non riesce a chiudere)
    try {
      unlistenSetupComplete = await listen('setup-complete', async () => {
        // Aspetta un po' per dare tempo al backend di chiudere la finestra
        await new Promise((resolve) => setTimeout(resolve, 500))
        const window = WebviewWindow.getCurrent()
        if (window) {
          try {
            // Verifica se la finestra è ancora aperta prima di chiuderla
            const isVisible = await window.isVisible()
            if (isVisible) {
              console.log('Setup window still visible, closing from frontend...')
              // Prova a chiudere più volte se necessario
              try {
                await window.close()
              } catch (closeErr) {
                console.warn('First close attempt failed, trying again...', closeErr)
                // Aspetta un po' e riprova
                await new Promise((resolve) => setTimeout(resolve, 200))
                try {
                  await window.close()
                } catch (closeErr2) {
                  console.error('Failed to close window after retry:', closeErr2)
                  // Ultimo tentativo: nascondi invece di chiudere
                  try {
                    await window.hide()
                  } catch (hideErr) {
                    console.error('Failed to hide window:', hideErr)
                  }
                }
              }
            }
          } catch (err) {
            console.error('Failed to check window visibility:', err)
            // Fallback: prova comunque a chiudere
            try {
              await window.close()
            } catch (closeErr) {
              console.error('Failed to close window:', closeErr)
              // Ultimo fallback: nascondi
              try {
                await window.hide()
              } catch (hideErr) {
                console.error('Failed to hide window:', hideErr)
              }
            }
          }
        }
      })
    } catch (error) {
      console.error('Failed to listen to setup-complete event:', error)
    }
  })

  onDestroy(() => {
    if (unlistenSetupComplete) {
      unlistenSetupComplete()
    }
  })

  function handleThemeChange(value: string) {
    theme = value
    document.documentElement.setAttribute('data-theme', theme)
  }

  async function handleLanguageChange(value: string) {
    language = value
    // Applica la lingua immediatamente
    setLanguage(value as any)
    // Aggiorna le opzioni del tema con la nuova lingua
    themeOptions = [
      { value: 'light', label: $t('Light') },
      { value: 'dark', label: $t('Dark') },
    ]
  }

  async function handleComplete() {
    if (isLoading) return // Previeni doppi click
    isLoading = true
    try {
      await invoke('cmd_complete_setup', {
        setupData: {
          run_on_startup: runOnStartup,
          theme: theme,
          always_on_top: alwaysOnTop,
          show_opt_notifications: showNotifications,
          language: language,
        },
      })

      // Il backend ha emesso l'evento setup-complete
      // Aspetta che la finestra principale sia pronta prima di chiudere il setup
      // Verifica che la finestra principale esista e sia visibile
      let attempts = 0
      const maxAttempts = 20 // 2 secondi totali (20 * 100ms)

      const checkAndClose = async () => {
        attempts++
        try {
          // Verifica se la finestra principale esiste e è visibile
          // Usa l'API corretta di Tauri v2
          const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow')
          const mainWindow = WebviewWindow.getByLabel('main')

          if (mainWindow) {
            try {
              const isVisible = await mainWindow.isVisible()
              if (isVisible) {
                console.log('Main window is visible, closing setup...')
                // Aspetta ancora un po' per assicurarsi che la finestra principale sia completamente caricata
                await new Promise((resolve) => setTimeout(resolve, 300))
                const currentWindow = WebviewWindow.getCurrent()
                if (currentWindow) {
                  await currentWindow.close()
                }
                return
              }
            } catch (err) {
              console.warn('Failed to check main window visibility:', err)
            }
          }

          // Se non abbiamo ancora trovato la finestra principale, riprova
          if (attempts < maxAttempts) {
            setTimeout(checkAndClose, 100)
          } else {
            // Timeout: chiudi comunque il setup
            console.warn('Timeout waiting for main window, closing setup anyway...')
            const currentWindow = WebviewWindow.getCurrent()
            if (currentWindow) {
              try {
                await currentWindow.close()
              } catch (err) {
                console.error('Failed to close setup window:', err)
                isLoading = false
              }
            }
          }
        } catch (err) {
          console.error('Error checking windows:', err)
          // Fallback: chiudi dopo un delay
          if (attempts >= maxAttempts) {
            const currentWindow = WebviewWindow.getCurrent()
            if (currentWindow) {
              try {
                await currentWindow.close()
              } catch (closeErr) {
                console.error('Failed to close setup window:', closeErr)
                isLoading = false
              }
            }
          } else {
            setTimeout(checkAndClose, 100)
          }
        }
      }

      // Inizia il check dopo un piccolo delay per dare tempo al backend
      setTimeout(checkAndClose, 200)
    } catch (error) {
      console.error('Failed to complete setup:', error)
      alert('Failed to save settings. Please try again.')
      isLoading = false
    }
  }

  async function handleClose() {
    const window = WebviewWindow.getCurrent()
    await window?.close()
  }

  function handleDragStart(e: MouseEvent) {
    // Solo se clicchi sulla titlebar
    const target = e.target as HTMLElement
    if (target.closest('.titlebar')) {
      const window = WebviewWindow.getCurrent()
      window?.startDragging()
    }
  }
</script>

<button
  class="setup-container"
  on:mousedown={handleDragStart}
  tabindex="-1"
  type="button"
  style="border: none; padding: 0; width: 100%; height: 100%;"
>
  <Titlebar title="Tommy Memory Cleaner - Setup" onClose={handleClose} />

  <div class="setup-content">
    <div class="setup-header">
      <h1>{$t('Welcome to Tommy Memory Cleaner')}</h1>
      <img src="/icon.png" alt="Tommy Memory Cleaner" class="app-icon" />
    </div>

    <div class="setup-options">
      <div class="option-group">
        <div class="option-row">
          <label>
            <input type="checkbox" bind:checked={runOnStartup} />
            <span>{$t('Run on Windows startup')}</span>
          </label>
        </div>

        <div class="option-row">
          <label>
            <input type="checkbox" bind:checked={alwaysOnTop} />
            <span>{$t('Always on top')}</span>
          </label>
        </div>

        <div class="option-row">
          <label>
            <input type="checkbox" bind:checked={showNotifications} />
            <span>{$t('Show optimization notifications')}</span>
          </label>
        </div>
      </div>

      <div class="option-group">
        <div class="option-row">
          <label for="theme-select">{$t('Theme')}</label>
          <CustomSelect
            id="theme-select"
            options={themeOptions}
            value={theme}
            noShimmer={true}
            on:change={(e) => handleThemeChange(e.detail)}
          />
        </div>

        <div class="option-row">
          <label for="language-select">{$t('Language')}</label>
          <CustomSelect
            id="language-select"
            options={languageOptions}
            value={language}
            noShimmer={true}
            on:change={(e) => handleLanguageChange(e.detail)}
          />
        </div>
      </div>
    </div>

    <div class="setup-footer">
      <button class="complete-btn no-shimmer" on:click={handleComplete} disabled={isLoading}>
        {isLoading ? $t('Saving...') : $t('Complete Setup')}
      </button>
    </div>
  </div>
</button>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    padding: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--bg);
    border: none !important;
    outline: none !important;
    box-shadow: none !important;
    border-radius: 10px;
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

  .setup-container {
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--fg);
    overflow: hidden;
    position: relative;
    animation: fadeIn 0.2s ease;
    /* Assicura che il contenuto copra completamente la finestra su Windows 10 */
    margin: 0;
    padding: 0;
    box-shadow: none;
    border: none;
    outline: none;
    /* Assicura opacità completa su Windows */
    -webkit-backdrop-filter: none;
    backdrop-filter: none;
  }
  
  /* Aggiungiamo uno stile per il contenuto principale simile alla full view */
  .setup-content {
    flex: 1;
    padding: 10px;
    background: var(--bg);
    overflow-y: auto;
    overflow-x: hidden;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  
  /* Scrollbar styling come nella full view */
  .setup-content::-webkit-scrollbar {
    width: 5px;
  }
  
  .setup-content::-webkit-scrollbar-track {
    background: var(--bar-track);
  }
  
  .setup-content::-webkit-scrollbar-thumb {
    background: var(--bar-fill);
    border-radius: 3px;
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

  .setup-header {
    text-align: center;
    padding: 8px 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .setup-header h1 {
    font-size: 20px;
    font-weight: 600;
    margin: 0;
    color: var(--fg);
  }

  .app-icon {
    width: 56px;
    height: 56px;
    object-fit: contain;
    flex-shrink: 0;
  }

  .setup-options {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .option-group {
    background: var(--card);
    border-radius: 12px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex-shrink: 0;
  }

  .option-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 28px;
  }

  .option-row > label:first-child {
    flex: 0 0 auto;
    cursor: url('/cursors/light/hand.cur'), pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    user-select: none;
  }
  
  /* Dark theme cursor for label */
  html[data-theme='dark'] .option-row > label:first-child {
    cursor: url('/cursors/dark/hand.cur'), pointer;
  }

  .option-row label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: url('/cursors/light/hand.cur'), pointer;
    font-size: 13px;
    font-weight: 450;
    flex: 1;
    min-width: 0;
  }
  
  /* Dark theme cursor for option row label */
  html[data-theme='dark'] .option-row label {
    cursor: url('/cursors/dark/hand.cur'), pointer;
  }
  
  .option-row label > span {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .option-row input[type='checkbox'] {
    width: 18px;
    height: 18px;
    cursor: url('/cursors/light/hand.cur'), pointer;
    accent-color: var(--btn-bg);
    flex-shrink: 0;
  }
  
  /* Dark theme cursor for checkbox */
  html[data-theme='dark'] .option-row input[type='checkbox'] {
    cursor: url('/cursors/dark/hand.cur'), pointer;
  }

  .option-row > label:first-child {
    flex: 0 0 auto;
    min-width: 80px;
  }

  .setup-footer {
    padding: 8px;
    background: var(--bg);
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: center;
    flex-shrink: 0;
  }

  .complete-btn {
    background: var(--btn-bg);
    color: var(--text-on-primary);
    border: none;
    border-radius: 8px;
    padding: 10px 28px;
    font-size: 14px;
    font-weight: 500;
    cursor: url('/cursors/light/hand.cur'), pointer;
    transition: opacity 0.2s;
  }
  
  /* Dark theme cursor for button */
  html[data-theme='dark'] .complete-btn {
    cursor: url('/cursors/dark/hand.cur'), pointer;
  }
  
  .complete-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .complete-btn:disabled {
    opacity: 0.6;
    cursor: url('/cursors/light/no.cur'), not-allowed;
  }
  
  html[data-theme='dark'] .complete-btn:disabled {
    cursor: url('/cursors/dark/no.cur'), not-allowed;
  }

  .complete-btn.no-shimmer::after {
    display: none !important;
    content: none !important;
    animation: none !important;
    background: none !important;
    pointer-events: none !important;
    opacity: 0 !important;
    visibility: hidden !important;
    transform: none !important;
  }

  /* DISABILITA COMPLETAMENTE LO SHIMMER IN TUTTO IL SETUP */
  .setup-container :global(.option-item.selected::after),
  .setup-container :global([data-theme='light'] .option-item.selected::after),
  .setup-container :global(html[data-theme='dark'] .option-item.selected::after),
  .setup-container :global(button::after),
  .setup-container :global(.shimmer-btn::after),
  .setup-container :global(.complete-btn::after) {
    display: none !important;
    content: none !important;
    animation: none !important;
    background: none !important;
    pointer-events: none !important;
    opacity: 0 !important;
    visibility: hidden !important;
    transform: none !important;
  }
</style>
