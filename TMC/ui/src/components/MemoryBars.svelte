<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { memory } from '../lib/store';
  import type { MemoryInfo } from '../lib/types';
  import { t } from '../i18n/index';
  
  let mem: MemoryInfo | null = null;
  let unsub: (() => void) | null = null;

  onMount(() => {
    unsub = memory.subscribe((v) => (mem = v));
  });

  onDestroy(() => {
    if (unsub) unsub();
  });
</script>

<style>
  .card { 
    background: var(--card); 
    border-radius: 12px; 
    padding: 12px; 
  }
  
  .bar { 
    height: 10px; 
    border-radius: 8px; 
    background: var(--bar-track); 
    position: relative; 
    overflow: hidden; 
  }
  
  .fill { 
    position: absolute; 
    left: 0; 
    top: 0; 
    bottom: 0; 
    background: var(--bar-fill); 
    border-radius: 8px;
    transition: width 0.3s ease;
  }
  
  .row { 
    display: grid; 
    grid-template-columns: 65px 1fr 140px; 
    gap: 10px; 
    align-items: center; 
    margin: 12px 0; 
  }
  
  .label { 
    font-size: 13px;
    font-weight: 500;
  }
  
  .val { 
    font-size: 12px;
    text-align: right;
    opacity: 0.9;
  }
</style>

<div class="card">
  <div class="row">
    <div class="label">{$t('Physical')}</div>
    <div class="bar">
      <div class="fill" style="width: {mem ? mem.physical.used.percentage : 0}%"></div>
    </div>
    <div class="val">
      {mem ? `${mem.physical.used.value.toFixed(1)} ${mem.physical.used.unit} (${mem.physical.used.percentage}%)` : '--'}
    </div>
  </div>
  <div class="row">
    <div class="label">{$t('Free')}</div>
    <div class="bar">
      <div class="fill" style="width: {mem ? mem.physical.free.percentage : 0}%"></div>
    </div>
    <div class="val">
      {mem ? `${mem.physical.free.value.toFixed(1)} ${mem.physical.free.unit} (${mem.physical.free.percentage}%)` : '--'}
    </div>
  </div>
</div>