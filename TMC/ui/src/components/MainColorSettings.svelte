<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { config, updateConfig } from '../lib/store';
  import type { Config } from '../lib/types';
  import { t } from '../i18n/index';

  let cfg: Config | null = null;
  let unsub: (() => void) | null = null;

  onMount(() => {
    unsub = config.subscribe((v) => (cfg = v));
    applyMainColor();
  });

  onDestroy(() => {
    if (unsub) unsub();
  });

  function applyMainColor() {
    if (!cfg) return;
    const theme = cfg.theme === 'light' ? 'light' : 'dark';
    const mainColor = theme === 'light'
      ? (cfg.main_color_hex_light || cfg.main_color_hex || '#9a8a72')
      : (cfg.main_color_hex_dark || cfg.main_color_hex || '#0a84ff');
    
    const root = document.documentElement;
    root.style.setProperty('--btn-bg', mainColor);
    root.style.setProperty('--bar-fill', mainColor);
    root.style.setProperty('--input-focus', mainColor);
  }

  function onColorChange(e: Event) {
    const target = e.target as HTMLInputElement;
    const color = target.value;
    const theme = cfg?.theme === 'light' ? 'light' : 'dark';
    
    // Salva nel campo corretto in base al tema
    if (theme === 'light') {
      updateConfig({ main_color_hex_light: color });
    } else {
      updateConfig({ main_color_hex_dark: color });
    }
    
    applyMainColor();
  }

  function resetColor() {
    const theme = cfg?.theme === 'light' ? 'light' : 'dark';
    const defaultColor = theme === 'dark' ? '#0a84ff' : '#9a8a72';
    
    if (theme === 'light') {
      updateConfig({ main_color_hex_light: defaultColor });
    } else {
      updateConfig({ main_color_hex_dark: defaultColor });
    }
    
    applyMainColor();
  }

  // Reapplica quando cambia il tema o il colore
  $: if (cfg) {
    setTimeout(applyMainColor, 100);
  }
</script>

<style>
  .group {
    background: var(--card);
    border-radius: 12px;
    padding: 10px;
  }

  .title {
    font-weight: 500;
    font-size: 12px;
    margin-bottom: 8px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 6px 0;
  }

  button {
    padding: 6px 14px;
    background: var(--btn-bg);
    color: white;
    border: none;
    border-radius: 10px;
    font-size: 11px;
    cursor: pointer;
    transition: all 0.2s;
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

  button:hover {
    opacity: 0.9;
    transform: translateY(-1px);
  }

  .color-input {
    width: 45px;
    height: 30px;
    padding: 0;
    border: 1px solid var(--border);
    background: var(--input-bg);
    cursor: pointer;
    border-radius: 10px;
  }

  .hint {
    font-size: 11px;
    opacity: 0.7;
    margin-top: 6px;
    line-height: 1.4;
  }
</style>

<div class="group">
  <div class="title">{$t('Main Color')}</div>
  
  <div class="row">
    <input
      type="color"
      value={(() => {
        if (!cfg) return document.documentElement.getAttribute('data-theme') === 'dark' ? '#0a84ff' : '#9a8a72';
        const theme = cfg.theme === 'light' ? 'light' : 'dark';
        return theme === 'light'
          ? (cfg.main_color_hex_light || cfg.main_color_hex || '#9a8a72')
          : (cfg.main_color_hex_dark || cfg.main_color_hex || '#0a84ff');
      })()}
      on:input={onColorChange}
      class="color-input"
    />
    <button on:click={resetColor}>{$t('Reset')}</button>
  </div>
  
  <div class="hint">
    {$t('This color will be used for buttons, progress bars, and accents throughout the app')}
  </div>
</div>

