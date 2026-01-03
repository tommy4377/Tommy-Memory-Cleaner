<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import Profiles from './Profiles.svelte'
  import MemoryBars from './MemoryBars.svelte'
  import MemoryFreed from './MemoryFreed.svelte'
  import AutoOptimization from './AutoOptimization.svelte'
  import BasicSettings from './BasicSettings.svelte'
  import ProcessExclusions from './ProcessExclusions.svelte'
  import Hotkey from './Hotkey.svelte'
  import MainColorSettings from './MainColorSettings.svelte'
  import TraySettings from './TraySettings.svelte'
  import ProgressFooter from './ProgressFooter.svelte'
  import { t } from '../i18n/index'
  import { Reason, AreasFlag } from '../lib/types'
  import type { Config } from '../lib/types'
  import { areasForProfile } from '../lib/profiles'
  import { config } from '../lib/store'
  import { optimizeAsync } from '../lib/api'

  let activeTab: 'main' | 'settings' | 'customization' = 'main'
  let hideTabs = false // Mostra i tabs
  let cfg: Config | null = null
  let unsub: (() => void) | null = null

  onMount(() => {
    unsub = config.subscribe((v) => (cfg = v))
  })

  onDestroy(() => {
    if (unsub) unsub()
  })

  async function onOptimize() {
    if (!cfg) {
      console.warn('Cannot optimize: no config')
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
</script>

<div class="container">
  {#if !hideTabs}
  <div class="tabs">
    <button class="tab" class:active={activeTab === 'main'} on:click={() => (activeTab = 'main')}>
      {$t('Main')}
    </button>
    <button
      class="tab"
      class:active={activeTab === 'settings'}
      on:click={() => (activeTab = 'settings')}
    >
      {$t('Settings')}
    </button>
    <button
      class="tab"
      class:active={activeTab === 'customization'}
      on:click={() => (activeTab = 'customization')}
    >
      {$t('Customization')}
    </button>
  </div>
  {/if}

  <div class="content">
    <div class="tab-content" class:active={activeTab === 'main' || hideTabs}>
      <Profiles />
      <MemoryBars />
      <MemoryFreed />
      <AutoOptimization />
    </div>

    {#if !hideTabs}
    <div class="tab-content" class:active={activeTab === 'settings'}>
      <BasicSettings />
      <ProcessExclusions />
    </div>

    <div class="tab-content" class:active={activeTab === 'customization'}>
      <Hotkey />
      <MainColorSettings />
      <TraySettings />
    </div>
    {/if}
  </div>

  <div class="footer">
    <ProgressFooter on:optimize={onOptimize} />
  </div>
</div>

<style>
  .container {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .tabs {
    display: flex;
    gap: 2px;
    padding: 4px 10px 0;
    background: var(--bg);
    height: 32px;
    flex-shrink: 0;
  }

  .tab {
    padding: 6px 14px;
    background: var(--card);
    color: var(--fg);
    border: 1px solid var(--border);
    border-bottom: none;
    border-radius: 10px 10px 0 0;
    cursor: url('/cursors/light/hand.cur'), pointer;
    font-size: 12px;
    opacity: 0.7;
    transition: all 0.2s;
  }

  .tab.active {
    opacity: 1;
    background: var(--bg);
  }

  /* Dark theme cursor for tabs */
  
  .content {
    flex: 1;
    padding: 10px;
    background: var(--bg);
    overflow-y: auto;
    overflow-x: hidden;
    min-height: 0;
  }

  .content::-webkit-scrollbar {
    width: 5px;
  }

  .content::-webkit-scrollbar-track {
    background: var(--bar-track);
  }

  .content::-webkit-scrollbar-thumb {
    background: var(--bar-fill);
    border-radius: 3px;
  }

  .tab-content {
    display: none;
    flex-direction: column;
    gap: 8px;
  }

  .tab-content.active {
    display: flex;
  }

  .footer {
    padding: 8px;
    background: var(--bg);
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }
</style>
