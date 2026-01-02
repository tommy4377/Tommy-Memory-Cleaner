<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { t } from '../i18n/index'

  let totalFreedGB = 0
  let isLoading = true

  onMount(async () => {
    await loadStats()
    isLoading = false
    
    // Listen for optimization events
    if (typeof window !== 'undefined' && window.__TAURI__) {
      const { listen } = await import('@tauri-apps/api/event')
      
      const unlisten = await listen('optimization-completed', (event: any) => {
        const payload = event.payload as { freed_physical_mb: number }
        if (payload.freed_physical_mb > 0) {
          totalFreedGB += payload.freed_physical_mb / 1024
          saveStats()
        }
      })
      
      onDestroy(() => {
        unlisten()
      })
    }
  })

  async function loadStats() {
    try {
      const stats = await invoke('get_memory_stats') as { total_freed_gb: number } | null
      if (stats) {
        totalFreedGB = stats.total_freed_gb
      }
    } catch (error) {
      console.error('Failed to load memory stats:', error)
    }
  }

  async function saveStats() {
    try {
      await invoke('save_memory_stats', {
        totalFreedGB,
        lastUpdated: new Date().toISOString()
      })
    } catch (error) {
      console.error('Failed to save memory stats:', error)
    }
  }

  function formatGB(value: number): string {
    // Always show in GB with 1 decimal place
    return `${value.toFixed(1)} GB`
  }

  // Dynamic font size based on number of digits
  $: fontSize = totalFreedGB >= 10 ? '14px' : totalFreedGB >= 1 ? '16px' : '18px'
  $: padding = totalFreedGB >= 10 ? '6px 12px' : '8px 16px'
  $: minWidth = totalFreedGB >= 10 ? '90px' : '100px'
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
    gap: 10px;
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
    background: var(--accent);
    color: var(--text-primary);
    padding: 6px 12px;
    border-radius: 8px;
    min-width: 90px;
    text-align: center;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    transition: all 0.2s ease;
    justify-self: end;
  }

  .val:hover {
    transform: translateY(-1px);
    box-shadow: 0 3px 6px rgba(0, 0, 0, 0.3);
  }
</style>
