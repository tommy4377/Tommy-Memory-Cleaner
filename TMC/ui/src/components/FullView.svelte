<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import Profiles from './Profiles.svelte';
  import MemoryBars from './MemoryBars.svelte';
  import BasicSettings from './BasicSettings.svelte';
  import ProcessExclusions from './ProcessExclusions.svelte';
  import AutoOptimization from './AutoOptimization.svelte';
  import ProgressFooter from './ProgressFooter.svelte';
  import Hotkey from './Hotkey.svelte';
  import TraySettings from './TraySettings.svelte';
  import MainColorSettings from './MainColorSettings.svelte';
  import { config } from '../lib/store';
  import { optimizeAsync } from '../lib/api';
  import { Reason, AreasFlag } from '../lib/types';
  import type { Config } from '../lib/types';
  import { t } from '../i18n/index';
  import { areasForProfile } from '../lib/profiles';
  
  let cfg: Config | null = null;
  let unsub: (() => void) | null = null;
  let activeTab: 'main' | 'settings' | 'customization' = 'main';

  onMount(() => {
    unsub = config.subscribe((v) => (cfg = v));
  });

  onDestroy(() => {
    if (unsub) unsub();
  });

  async function onOptimize() {
    if (!cfg) {
      console.warn('Cannot optimize: no config');
      return;
    }
    
    try {
      // FIX: Usa sempre le aree dal profilo selezionato, non quelle salvate
      // Il backend farà comunque il controllo finale, ma questo evita warning inutili
      const areas = areasForProfile(cfg.profile);
      
      console.log('Starting optimization with profile:', cfg.profile, 'areas:', areas);
      await optimizeAsync(Reason.Manual, areas);
      console.log('Optimization completed');
    } catch (error) {
      console.error('Optimization failed:', error);
      alert('Optimization failed: ' + (error instanceof Error ? error.message : String(error)));
    }
  }
</script>

<style>
  .container {
    height: calc(100vh - 32px);
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
  
  html[data-theme="dark"] .tab {
    cursor: url('/cursors/dark/hand.cur'), pointer;
  }

  .tab.active {
    opacity: 1;
    background: var(--bg);
  }

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

<div class="container">
  <div class="tabs">
    <button class="tab" class:active={activeTab === 'main'} on:click={() => activeTab = 'main'}>
      {$t('Main')}
    </button>
    <button class="tab" class:active={activeTab === 'settings'} on:click={() => activeTab = 'settings'}>
      {$t('Settings')}
    </button>
    <button class="tab" class:active={activeTab === 'customization'} on:click={() => activeTab = 'customization'}>
      {$t('Customization')}
    </button>
  </div>

  <div class="content">
    <div class="tab-content" class:active={activeTab === 'main'}>
      <Profiles />
      <MemoryBars />
      <AutoOptimization />
    </div>

    <div class="tab-content" class:active={activeTab === 'settings'}>
      <BasicSettings />
      <ProcessExclusions />
    </div>

    <div class="tab-content" class:active={activeTab === 'customization'}>
      <Hotkey />
      <MainColorSettings />
      <TraySettings />
    </div>
  </div>

  <div class="footer">
    <ProgressFooter on:optimize={onOptimize} />
  </div>
</div>