<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { t } from '../i18n/index'

  let totalFreedGB = 0
  let isLoading = true
  let unlisten: (() => void) | null = null

  onMount(async () => {
    // Load initial stats
    await loadStats()
    isLoading = false
    
    // Set up event listener
    try {
      unlisten = await listen('optimization-completed', (event: any) => {
        console.log('Optimization event received:', event)
        const payload = event.payload as { freed_physical_mb: number }
        console.log('Freed MB:', payload.freed_physical_mb)
        if (payload.freed_physical_mb > 0) {
          const freedGB = payload.freed_physical_mb / 1024
          console.log('Converted to GB:', freedGB)
          totalFreedGB += freedGB
          console.log('New totalFreedGB:', totalFreedGB)
          saveStats()
        }
      })
    } catch (error) {
      console.error('Failed to set up event listener:', error)
    }
  })

  onDestroy(() => {
    if (unlisten) {
      unlisten()
    }
  })

  async function loadStats() {
    try {
      console.log('Loading stats from AppData...')
      const stats = await invoke('get_memory_stats') as { total_freed_gb: number } | null
      console.log('Stats loaded from AppData:', stats)
      if (stats) {
        totalFreedGB = stats.total_freed_gb
        console.log('Set totalFreedGB to:', totalFreedGB)
      }
    } catch (error) {
      console.error('Failed to load memory stats:', error)
    }
  }

  async function saveStats() {
    try {
      console.log('Saving stats to AppData:', { totalFreedGB })
      await invoke('save_memory_stats', {
        totalFreedGb: totalFreedGB,
        lastUpdated: new Date().toISOString()
      })
      console.log('Stats saved successfully to AppData')
    } catch (error) {
      console.error('Failed to save memory stats:', error)
    }
  }

  function formatGB(value: number): string {
    // Always show in GB with 1 decimal place
    return `${value.toFixed(1)} GB`
  }

  // Dynamic font size and box width based on number of digits
  $: fontSize = totalFreedGB >= 10 ? '14px' : totalFreedGB >= 1 ? '16px' : '18px'
  $: padding = totalFreedGB >= 10 ? '6px 12px' : '8px 16px'
  $: minWidth = totalFreedGB >= 10 ? '100px' : totalFreedGB >= 1 ? '90px' : '80px'
</script>

<div class="card">
  <div class="row">
    <div class="label">{$t('Memory Freed Since Installation')}</div>
    <div class="val" style="font-size: {fontSize}; padding: {padding}; min-width: {minWidth};">
      {isLoading ? '--' : formatGB(totalFreedGB)}
    </div>
  </div>
</div>

<style>
  .card {
    background: var(--card);
    border-radius: 12px;
    padding: 12px;
  }

  .row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 6px;
    align-items: center;
    margin: 0;
  }

  .label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    text-align: left;
  }

  .val {
    font-weight: 600;
    font-size: 14px;
    font-variant-numeric: tabular-nums;
    background: var(--btn-bg);
    color: var(--text-primary);
    padding: 6px 12px;
    border-radius: 8px;
    min-width: 90px;
    text-align: center;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    transition: all 0.2s ease;
    justify-self: end;
    width: auto;
  }

  /* Light mode: make text white */
  :global([data-theme="light"]) .val {
    color: white;
  }

  .val:hover {
    transform: translateY(-1px);
    box-shadow: 0 3px 6px rgba(0, 0, 0, 0.3);
  }
</style>
