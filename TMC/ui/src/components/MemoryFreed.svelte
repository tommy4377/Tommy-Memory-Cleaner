<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { t } from '../i18n/index'

  let totalFreedGB = 0
  let isLoading = true
  let unlisten: (() => void) | null = null

  onMount(async () => {
    await loadStats()
    isLoading = false
    try {
      unlisten = await listen('optimization-completed', (event: any) => {
        const payload = event.payload as { freed_physical_mb: number }
        if (payload.freed_physical_mb > 0) {
          totalFreedGB += payload.freed_physical_mb / 1024
          saveStats()
        }
      })
    } catch (error) { console.error(error) }
  })

  onDestroy(() => { if (unlisten) unlisten() })

  async function loadStats() {
    try {
      const stats = await invoke('get_memory_stats') as { total_freed_gb: number } | null
      if (stats) totalFreedGB = stats.total_freed_gb
    } catch (error) { console.error(error) }
  }

  async function saveStats() {
    try {
      await invoke('save_memory_stats', {
        totalFreedGb: totalFreedGB,
        lastUpdated: new Date().toISOString()
      })
    } catch (error) { console.error(error) }
  }

  function formatGB(value: number): string {
    return `${value.toFixed(1)} GB`
  }

  // Manteniamo il font leggibile ma compatto
  $: fontSize = totalFreedGB >= 10 ? '14px' : '15px'
</script>

<div class="card compact-container">
  <div class="row-wrapper">
    <div class="label-section">
      {$t('Memory Freed Since Installation')}
    </div>
    <div 
      class="val" 
      style="font-size: {fontSize};"
    >
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

  .row-wrapper {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: rgba(0, 0, 0, 0.04);
    border-radius: 8px;
    padding: 8px 12px;
    transition: background 0.2s ease;
  }

  :global([data-theme="dark"]) .row-wrapper {
    background: #4B4B4D;
  }

  .label-section {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-primary);
    flex: 1;
  }

  .val {
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    background: var(--btn-bg);
    color: var(--text-primary);
    border-radius: 6px;
    text-align: center;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.15);
    transition: all 0.2s ease;
    cursor: url('/cursors/light/arrow.cur'), auto;
    padding: 6px 16px;
    min-width: 85px;
  }

  :global([data-theme="dark"]) .val {
    background: #2170c0;
    cursor: url('/cursors/dark/arrow.cur'), auto;
  }

  :global([data-theme="light"]) .val {
    color: white;
  }

  .val:hover {
    transform: translateY(-1px);
    box-shadow: 0 3px 6px rgba(0, 0, 0, 0.25);
  }

  .row-wrapper:hover {
    /* Rimuovuto l'effetto hover */
  }
</style>