<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { t } from '../i18n/index'

  let totalFreedGB = 0
  let lastFreedMB = 0
  let isLoading = true
  let isCompact = false

  // Props to control display mode
  export let compact = false

  onMount(async () => {
    isCompact = compact
    await loadStats()
    isLoading = false
  })

  async function loadStats() {
    try {
      // Use Tauri invoke to get app data dir and read stats
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
      // Use Tauri invoke to save stats
      await invoke('save_memory_stats', {
        totalFreedGB,
        lastUpdated: new Date().toISOString()
      })
    } catch (error) {
      console.error('Failed to save memory stats:', error)
    }
  }

  // Listen for optimization completion events
  let unlisten: (() => void) | null = null
  
  onMount(async () => {
    isCompact = compact
    await loadStats()
    isLoading = false
    
    // Set up event listener for optimization results
    if (typeof window !== 'undefined' && window.__TAURI__) {
      const { listen } = await import('@tauri-apps/api/event')
      
      unlisten = await listen('optimization-completed', (event: any) => {
        const payload = event.payload as { freed_physical_mb: number }
        if (payload.freed_physical_mb > 0) {
          lastFreedMB = payload.freed_physical_mb
          totalFreedGB += payload.freed_physical_mb / 1024
          saveStats()
          
          // Reset last freed after 5 seconds
          setTimeout(() => {
            lastFreedMB = 0
          }, 5000)
        }
      })
    }
  })

  onDestroy(() => {
    if (unlisten) unlisten()
  })

  function formatGB(value: number): string {
    if (value < 1) {
      return `${(value * 1024).toFixed(1)} MB`
    }
    return `${value.toFixed(2)} GB`
  }

  function formatMB(value: number): string {
    if (value < 1024) {
      return `${value.toFixed(0)} MB`
    }
    return `${(value / 1024).toFixed(2)} GB`
  }
</script>

<div class="memory-stats-card" class:compact={isCompact}>
  {#if !isCompact}
    <div class="stats-header">
      <div class="stats-title">{$t('Memory Freed')}</div>
      <div class="stats-subtitle">{$t('Since installation')}</div>
    </div>
  {/if}
  
  <div class="stats-content" class:compact-content={isCompact}>
    <div class="total-freed" class:compact-total={isCompact}>
      <div class="total-value" class:compact-value={isCompact}>
        {isLoading ? '--' : formatGB(totalFreedGB)}
      </div>
      <div class="total-label" class:compact-label={isCompact}>
        {isCompact ? '' : $t('Total freed')}
      </div>
    </div>
    
    {#if lastFreedMB > 0 && !isCompact}
      <div class="last-freed animated">
        <div class="last-value">+{formatMB(lastFreedMB)}</div>
        <div class="last-label">{$t('Just now')}</div>
      </div>
    {/if}
  </div>
</div>

<style>
  .memory-stats-card {
    background: var(--card);
    border-radius: 12px;
    padding: 16px;
    margin-bottom: 16px;
    border: 1px solid var(--border);
    position: relative;
    overflow: hidden;
  }

  .memory-stats-card.compact {
    padding: 8px 12px;
    margin-bottom: 0;
  }

  .memory-stats-card::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: linear-gradient(90deg, #4CAF50, #8BC34A, #CDDC39);
    border-radius: 12px 12px 0 0;
  }

  .stats-header {
    margin-bottom: 12px;
  }

  .stats-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .stats-subtitle {
    font-size: 12px;
    color: var(--text-secondary);
    opacity: 0.8;
  }

  .stats-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }

  .stats-content.compact-content {
    gap: 8px;
  }

  .total-freed {
    flex: 1;
  }

  .total-freed.compact-total {
    text-align: center;
  }

  .total-value {
    font-size: 24px;
    font-weight: 700;
    color: var(--accent);
    margin-bottom: 2px;
    font-variant-numeric: tabular-nums;
  }

  .total-value.compact-value {
    font-size: 16px;
    margin-bottom: 0;
  }

  .total-label {
    font-size: 11px;
    color: var(--text-secondary);
    opacity: 0.8;
  }

  .total-label.compact-label {
    font-size: 9px;
  }

  .last-freed {
    background: linear-gradient(135deg, var(--accent), #4CAF50);
    color: white;
    padding: 8px 12px;
    border-radius: 8px;
    text-align: center;
    animation: slideIn 0.3s ease-out;
  }

  .last-value {
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 2px;
  }

  .last-label {
    font-size: 10px;
    opacity: 0.9;
  }

  @keyframes slideIn {
    from {
      transform: translateX(20px);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .last-freed {
      animation: none;
    }
  }
</style>
