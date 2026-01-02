<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { config, applyProfile } from '../lib/store'
  import type { Config, Profile } from '../lib/types'
  import { areasForProfile } from '../lib/profiles'
  import { t } from '../i18n/index'

  let cfg: Config | null = null
  let unsub: (() => void) | null = null
  let selected: Profile = 'Balanced'
  let isChanging = false
  let changeTimeout: number | null = null

  onMount(() => {
    unsub = config.subscribe((value) => {
      cfg = value
      if (value && value.profile) {
        selected = value.profile
      }
    })
  })

  onDestroy(() => {
    if (unsub) {
      unsub()
      unsub = null
    }
    if (changeTimeout) {
      clearTimeout(changeTimeout)
      changeTimeout = null
    }
  })

  async function selectProfile(profile: Profile) {
    if (isChanging || selected === profile) return

    // Clear any existing timeout
    if (changeTimeout) {
      clearTimeout(changeTimeout)
    }

    isChanging = true
    // Don't update selected immediately to avoid visual flicker
    const previousSelected = selected

    try {
      await applyProfile(profile)
      // Only update after successful apply
      selected = profile
    } catch (error) {
      console.error('Failed to apply profile:', error)
      // Rollback on error
      selected = previousSelected
    } finally {
      // Add a small delay to prevent rapid clicking
      changeTimeout = setTimeout(() => {
        isChanging = false
        changeTimeout = null
      }, 200) as unknown as number
    }
  }

  function handleDragStart(e: DragEvent) {
    // Previene il drag dei pulsanti
    e.preventDefault()
  }

  function handleDragOver(e: DragEvent) {
    // Previene il comportamento di default del drag
    e.preventDefault()
  }

  $: t_func = $t
  $: translatedAreaNames = (() => {
    const areas = areasForProfile(selected)
    const areaNames: string[] = []

    // Usa l'ordine e i nomi specifici per ogni area
    if (areas & 128) areaNames.push(t_func('Working Set'))
    if (areas & 4) areaNames.push(t_func('Modified Pages'))
    if (areas & 16) areaNames.push(t_func('Standby List'))
    if (areas & 32) areaNames.push(t_func('Low Priority Standby'))
    if (areas & 64) areaNames.push(t_func('System Cache'))
    if (areas & 1) areaNames.push(t_func('Combined Pages'))
    if (areas & 2) areaNames.push(t_func('File Cache'))
    if (areas & 8) areaNames.push(t_func('Registry Cache'))

    return areaNames.join(', ')
  })()
</script>

<div class="group">
  <div class="seg">
    <button
      class:active={selected === 'Normal'}
      on:click={() => selectProfile('Normal')}
      on:dragstart={handleDragStart}
      on:selectstart={handleDragStart}
      disabled={isChanging}
    >
      {$t('Normal')}
    </button>
    <button
      class:active={selected === 'Balanced'}
      on:click={() => selectProfile('Balanced')}
      on:dragstart={handleDragStart}
      on:selectstart={handleDragStart}
      disabled={isChanging}
    >
      {$t('Balanced')}
    </button>
    <button
      class:active={selected === 'Gaming'}
      on:click={() => selectProfile('Gaming')}
      on:dragstart={handleDragStart}
      on:selectstart={handleDragStart}
      disabled={isChanging}
    >
      {$t('Gaming')}
    </button>
  </div>

  <div class="info">
    <div class="info-title">{$t('Active areas')}:</div>
    <div class="areas-list">
      {translatedAreaNames}
    </div>
  </div>
</div>

<style>
  .group {
    background: var(--card);
    border-radius: 12px;
    padding: 12px;
  }

  .seg {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
  }

  .seg button {
    flex: 1;
    padding: 10px 14px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--fg);
    cursor: url('/cursors/light/hand.cur'), pointer;
    font-size: 13px;
    font-weight: 500;
    transition: all 0.2s;
    position: relative;
    overflow: hidden;
    user-select: none;
    -webkit-user-select: none;
    -moz-user-select: none;
    -ms-user-select: none;
  }

  /* Effetto shimmer per i bottoni dei profili quando sono attivi */
  .seg button.active::after {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: linear-gradient(
      135deg,
      transparent 30%,
      rgba(255, 255, 255, 0.15) 50%,
      transparent 70%
    );
    animation: shimmer 2s infinite;
    pointer-events: none;
    border-radius: 10px;
  }

  @keyframes shimmer {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(100%);
    }
  }

  html[data-theme='dark'] .seg button {
    cursor: url('/cursors/dark/hand.cur'), pointer;
  }

  .seg button:hover:not(.active):not(:disabled) {
    background: var(--bar-track);
  }

  .seg button.active {
    background: var(--btn-bg);
    border-color: var(--btn-border);
    color: var(--btn-fg);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .seg button:disabled {
    opacity: 0.7;
    cursor: wait;
    position: relative;
  }

  /* Add loading spinner for disabled state */
  .seg button:disabled::before {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    width: 12px;
    height: 12px;
    margin: -6px 0 0 -6px;
    border: 2px solid transparent;
    border-top-color: var(--btn-bg);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .info {
    margin-top: 10px;
    padding: 10px;
    background: var(--bar-track);
    border-radius: 6px;
    font-size: 11px;
    line-height: 1.6;
  }

  .info-title {
    font-weight: 600;
    color: var(--btn-bg);
    margin-bottom: 4px;
    font-size: 12px;
  }

  .areas-list {
    opacity: 0.9;
  }
</style>
