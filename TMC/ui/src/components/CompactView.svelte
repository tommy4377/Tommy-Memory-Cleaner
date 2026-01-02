<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { memory, config, progress } from '../lib/store'
  import { optimizeAsync } from '../lib/api'
  import { Reason, AreasFlag } from '../lib/types'
  import type { MemoryInfo, Config } from '../lib/types'
  import { t } from '../i18n/index'
  import { areasForProfile } from '../lib/profiles'

  let memInfo: MemoryInfo | null = null
  let cfg: Config | null = null
  let prog: any = null
  let memUnsub: (() => void) | null = null
  let cfgUnsub: (() => void) | null = null
  let progUnsub: (() => void) | null = null
  let totalFreedGB = 0
  let lastFreedMB = 0

  onMount(() => {
    memUnsub = memory.subscribe((v) => (memInfo = v))
    cfgUnsub = config.subscribe((v) => (cfg = v))
    progUnsub = progress.subscribe((v) => (prog = v))
    
    // Load memory stats
    loadMemoryStats()
    
    // Listen for optimization events
    if (typeof window !== 'undefined' && window.__TAURI__) {
      import('@tauri-apps/api/event').then(({ listen }) => {
        listen('optimization-completed', (event: any) => {
          const payload = event.payload as { freed_physical_mb: number }
          if (payload.freed_physical_mb > 0) {
            lastFreedMB = payload.freed_physical_mb
            totalFreedGB += payload.freed_physical_mb / 1024
            saveMemoryStats()
            
            // Reset after 5 seconds
            setTimeout(() => {
              lastFreedMB = 0
            }, 5000)
          }
        })
      })
    }
  })

  onDestroy(() => {
    if (memUnsub) memUnsub()
    if (cfgUnsub) cfgUnsub()
    if (progUnsub) progUnsub()
  })

  async function loadMemoryStats() {
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        const { invoke } = await import('@tauri-apps/api/core')
        const stats = await invoke('get_memory_stats') as { total_freed_gb: number } | null
        if (stats) {
          totalFreedGB = stats.total_freed_gb
        }
      }
    } catch (error) {
      console.error('Failed to load memory stats:', error)
    }
  }

  async function saveMemoryStats() {
    try {
      if (typeof window !== 'undefined' && window.__TAURI__) {
        const { invoke } = await import('@tauri-apps/api/core')
        await invoke('save_memory_stats', {
          totalFreedGB,
          lastUpdated: new Date().toISOString()
        })
      }
    } catch (error) {
      console.error('Failed to save memory stats:', error)
    }
  }

  async function optimize() {
    // FIX: Usa prog?.running invece di optimizing locale per mantenere lo stato
    if (prog?.running || !cfg) {
      console.warn('Cannot optimize: running=', prog?.running, 'cfg=', cfg)
      return
    }

    try {
      // FIX: Usa sempre le aree dal profilo selezionato, non quelle salvate
      // Il backend farà comunque il controllo finale, ma questo evita warning inutili
      const areas = areasForProfile(cfg.profile)

      console.log('Starting optimization with profile:', cfg.profile, 'areas:', areas)
      await optimizeAsync(Reason.Manual, areas)
      console.log('Optimization completed')
    } catch (error) {
      console.error('Optimization failed:', error)
      alert('Optimization failed: ' + (error instanceof Error ? error.message : String(error)))
    }
  }

  // Helper per ottenere il testo del bottone tradotto (reattivo)
  $: buttonText = prog?.running ? $t('Optimizing...') : $t('Optimize')

  function formatMB(mb: number) {
    return `${mb.toFixed(1)} MB`
  }

  function formatGB(gb: number) {
    return `${gb.toFixed(1)} GB`
  }
</script>

<div class="compact">
  <div class="bar-container">
    <div class="bar">
      <div
        class="fill"
        class:warning={memInfo &&
          memInfo.physical.used.percentage >= 80 &&
          memInfo.physical.used.percentage < 90}
        class:danger={memInfo && memInfo.physical.used.percentage >= 90}
        style="width: {memInfo?.physical.used.percentage ?? 0}%"
      >
        <span class="percent">{memInfo?.physical.used.percentage ?? 0}%</span>
      </div>
    </div>
    
    <!-- Small memory freed indicator -->
    {#if lastFreedMB > 0}
    <div class="freed-indicator">
      <span class="freed-value">+{formatMB(lastFreedMB)}</span>
    </div>
    {/if}
  </div>
  
  <div class="stats-row">
    <div class="stat-item">
      <span class="stat-label">{$t('Free')}</span>
      <span class="stat-value">
        {memInfo ? `${memInfo.physical.free.value.toFixed(1)} ${memInfo.physical.free.unit}` : '--'}
      </span>
    </div>
    <div class="stat-item">
      <span class="stat-label">{$t('Total')}</span>
      <span class="stat-value">
        {formatGB(totalFreedGB)}
      </span>
    </div>
  </div>
  
  <button on:click={optimize} disabled={prog?.running}>
    {buttonText}
  </button>
</div>

<style>
  .compact {
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: calc(100% - 36px);
  }

  .bar-container {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .bar {
    flex: 1;
    height: 28px;
    background: var(--bar-track);
    border-radius: 14px;
    position: relative;
    overflow: hidden;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .fill {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    background: var(--bar-fill);
    transition: width 0.3s ease;
    border-radius: 14px;
  }

  .percent {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 12px;
    font-weight: 600;
    color: white;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
  }

  .freed-indicator {
    background: linear-gradient(135deg, var(--accent), #4CAF50);
    color: white;
    padding: 4px 8px;
    border-radius: 6px;
    animation: slideIn 0.3s ease-out;
    font-size: 11px;
    font-weight: 600;
  }

  .freed-value {
    color: white;
  }

  .stats-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }

  .stat-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .stat-label {
    font-size: 10px;
    color: var(--text-secondary);
    opacity: 0.8;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .stat-value {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }

  button {
    padding: 8px 16px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 8px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    font-size: 13px;
    min-width: fit-content;
    width: auto;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
    transition: all 0.2s;
    position: relative;
    overflow: hidden;
    white-space: nowrap;
    text-align: center;
  }

  /* Effetto shimmer per il bottone optimize */
  button:not(:disabled)::after {
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
    border-radius: 14px;
  }

  @keyframes shimmer {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(100%);
    }
  }

  button:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 3px 6px rgba(0, 0, 0, 0.15);
  }

  button:active:not(:disabled) {
    transform: translateY(0);
  }

  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    background: linear-gradient(135deg, #6a6a6a, #4a4a4a);
    animation: pulse 1.5s infinite;
  }

  /* Rimuovi shimmer quando disabled */
  button:disabled::after {
    display: none;
  }

  @keyframes pulse {
    0% {
      opacity: 1;
    }
    50% {
      opacity: 0.6;
    }
    100% {
      opacity: 1;
    }
  }
</style>
