<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { config, applyProfile } from '../lib/store';
  import type { Config, Profile } from '../lib/types';
  import { areasForProfile } from '../lib/profiles';
  import { t } from '../i18n/index';
  
  let cfg: Config | null = null;
  let unsub: (() => void) | null = null;
  let selected: Profile = 'Balanced';
  let isChanging = false;

  onMount(() => {
    unsub = config.subscribe(value => {
      cfg = value;
      if (value && value.profile) {
        selected = value.profile;
      }
    });
  });

  onDestroy(() => {
    if (unsub) {
      unsub();
      unsub = null;
    }
  });

  async function selectProfile(profile: Profile) {
    if (isChanging || selected === profile) return;
    
    isChanging = true;
    selected = profile;
    
    try {
      await applyProfile(profile);
    } catch (error) {
      console.error('Failed to apply profile:', error);
      // Rollback on error
      if (cfg) {
        selected = cfg.profile;
      }
    } finally {
      isChanging = false;
    }
  }
  
  function handleDragStart(e: DragEvent) {
    // Previene il drag dei pulsanti
    e.preventDefault();
  }
  
  function handleDragOver(e: DragEvent) {
    // Previene il comportamento di default del drag
    e.preventDefault();
  }
  
  $: translatedAreaNames = (() => {
    const areas = areasForProfile(selected);
    const areaNames: string[] = [];
    
    // Usa l'ordine e i nomi specifici per ogni area
    if (areas & 128) areaNames.push($t('Working Set'));
    if (areas & 4) areaNames.push($t('Modified Pages'));
    if (areas & 16) areaNames.push($t('Standby List'));
    if (areas & 32) areaNames.push($t('Low Priority Standby'));
    if (areas & 64) areaNames.push($t('System Cache'));
    if (areas & 1) areaNames.push($t('Combined Pages'));
    if (areas & 2) areaNames.push($t('File Cache'));
    if (areas & 8) areaNames.push($t('Registry Cache'));
    
    return areaNames.join(', ');
  })();
  

</script>

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
    background: linear-gradient(135deg, transparent 30%, rgba(255,255,255,0.15) 50%, transparent 70%);
    animation: shimmer 2s infinite;
    pointer-events: none;
    border-radius: 10px;
  }
  
  @keyframes shimmer {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(100%); }
  }
  
  html[data-theme="dark"] .seg button {
    cursor: url('/cursors/dark/hand.cur'), pointer;
  }
  
  .seg button:hover:not(.active):not(:disabled) {
    background: var(--bar-track);
  }
  
  .seg button.active { 
    background: var(--btn-bg); 
    border-color: var(--btn-border); 
    color: var(--btn-fg); 
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  }
  
  .seg button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
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