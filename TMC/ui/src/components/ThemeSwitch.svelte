<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../i18n/index';
  
  let dark = true; // Default dark
  
  onMount(() => {
    const th = localStorage.getItem('tmc_theme');
    if (th) {
      dark = th === 'dark';
    } else {
      // Default dark mode
      dark = true;
      localStorage.setItem('tmc_theme', 'dark');
    }
    document.documentElement.setAttribute('data-theme', dark ? 'dark' : 'light');
  });
  
  function toggle() {
    dark = !dark;
    document.documentElement.setAttribute('data-theme', dark ? 'dark' : 'light');
    localStorage.setItem('tmc_theme', dark ? 'dark' : 'light');
  }
</script>

<style>
  .sw { 
    background: var(--card); 
    border-radius: 8px; 
    padding: 10px; 
    margin-top: 8px; 
    display: flex; 
    align-items: center; 
    gap: 8px; 
  }
  button { 
    background: var(--btn-bg); 
    color: var(--btn-fg); 
    border: 1px solid var(--btn-border); 
    border-radius: 6px; 
    padding: 6px 10px;
    position: relative;
    overflow: hidden;
  }
  
  button::after {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: linear-gradient(135deg, transparent 30%, rgba(255,255,255,0.1) 50%, transparent 70%);
    animation: shimmer 2s infinite;
    pointer-events: none;
  }
  
  @keyframes shimmer {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(100%); }
  }
</style>

<div class="sw">
  <div>{$t('Theme')}</div>
  <button on:click={toggle}>{dark ? $t('Dark') : $t('Light')}</button>
</div>