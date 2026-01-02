<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { config, updateConfig } from '../lib/store'
  import type { Config } from '../lib/types'
  import { t } from '../i18n/index'

  let cfg: Config | null = null
  let unsub: (() => void) | null = null
  let hours = 0
  let freePct = 0
  let updateTimer: ReturnType<typeof setTimeout> | null = null

  onMount(() => {
    unsub = config.subscribe((v) => {
      cfg = v
      if (v) {
        hours = v.auto_opt_interval_hours || 0
        freePct = v.auto_opt_free_threshold || 0
      }
    })
  })

  onDestroy(() => {
    if (unsub) unsub()
    if (updateTimer) clearTimeout(updateTimer)
  })

  function scheduleUpdate(key: 'hours' | 'free', value: number) {
    if (updateTimer) clearTimeout(updateTimer)

    updateTimer = setTimeout(async () => {
      if (key === 'hours') {
        await updateConfig({ auto_opt_interval_hours: value })
      } else {
        await updateConfig({ auto_opt_free_threshold: value })
      }
    }, 300) // Debounce di 300ms
  }

  function onHoursInput(e: Event) {
    hours = parseInt(e.currentTarget.value)
    scheduleUpdate('hours', hours)
  }

  function onFreeInput(e: Event) {
    freePct = parseInt(e.currentTarget.value)
    scheduleUpdate('free', freePct)
  }
</script>

<div class="group">
  <div class="setting">
    <div class="label">
      <span>{$t('Auto optimize every')}</span>
      <span class="value">{hours} {$t('hours')}</span>
    </div>
    <input type="range" min="0" max="24" value={hours} on:input={onHoursInput} />
    <div class="hint">
      {#if hours === 0}
        {$t('Disabled')}
      {:else}
        {$t('Will optimize every X hours').replace('%hours%', hours.toString())}
      {/if}
    </div>
  </div>

  <div class="setting">
    <div class="label">
      <span>{$t('When free RAM below')}</span>
      <span class="value">{freePct}%</span>
    </div>
    <input type="range" min="0" max="50" value={freePct} on:input={onFreeInput} />
    <div class="hint">
      {#if freePct === 0}
        {$t('Disabled')}
      {:else}
        {$t('5 minute cooldown between optimizations')}
      {/if}
    </div>
  </div>
</div>

<style>
  .group {
    background: var(--card);
    border-radius: 12px;
    padding: 12px;
  }

  .setting {
    margin-bottom: 14px;
  }

  .label {
    display: flex;
    justify-content: space-between;
    margin-bottom: 6px;
    font-size: 13px;
  }

  .value {
    font-weight: 600;
    color: var(--btn-bg);
    font-size: 14px;
  }

  input[type='range'] {
    width: 100%;
    height: 8px;
    -webkit-appearance: none;
    appearance: none;
    background: var(--bar-track);
    border-radius: 8px;
    outline: none;
    cursor: pointer;
    margin: 0;
    padding: 0;
  }

  input[type='range']::-webkit-slider-track {
    background: var(--bar-track);
    height: 8px;
    border-radius: 8px;
    width: 100%;
  }

  input[type='range']::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 18px;
    height: 18px;
    background: var(--btn-bg);
    border: 2px solid var(--card);
    border-radius: 50%;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    cursor: pointer;
    transform: translateY(1px);
    transition: all 0.2s ease;
  }

  input[type='range']::-webkit-slider-thumb:hover {
    transform: translateY(1px) scale(1.1);
    box-shadow: 0 3px 6px rgba(0, 0, 0, 0.3);
  }

  input[type='range']::-moz-range-track {
    background: var(--bar-track);
    height: 8px;
    border-radius: 8px;
    border: none;
    width: 100%;
  }

  input[type='range']::-moz-range-thumb {
    width: 18px;
    height: 18px;
    background: var(--btn-bg);
    border: 2px solid var(--card);
    border-radius: 50%;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  input[type='range']::-moz-range-thumb:hover {
    transform: scale(1.1);
    box-shadow: 0 3px 6px rgba(0, 0, 0, 0.3);
  }

  .hint {
    font-size: 11px;
    opacity: 0.7;
    margin-top: 4px;
  }
</style>
