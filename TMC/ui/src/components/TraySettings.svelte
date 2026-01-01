<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { config, updateConfig } from '../lib/store';
  import type { Config } from '../lib/types';
  import { t } from '../i18n/index';

  let cfg: Config | null = null;
  let unsub: (() => void) | null = null;

  onMount(() => {
    unsub = config.subscribe((v) => (cfg = v));
  });

  onDestroy(() => {
    if (unsub) unsub();
  });

  function updateTray(partial: Partial<Config['tray']>) {
    if (!cfg) return;
    updateConfig({
      tray: { ...cfg.tray, ...partial }
    });
  }

  function resetTrayColors() {
    if (!cfg) return;
    // Colori originali ma leggermente meno accesi
    const defaultText = '#ffffff';
    const defaultBg = '#2d8a3d'; // Verde originale ma leggermente meno acceso
    const defaultWarning = '#d97706'; // Arancione originale ma leggermente meno acceso
    const defaultDanger = '#b91c1c'; // Rosso originale ma leggermente meno acceso
    updateConfig({
      tray: {
        ...cfg.tray,
        text_color_hex: defaultText,
        background_color_hex: defaultBg,
        warning_color_hex: defaultWarning,
        danger_color_hex: defaultDanger
      }
    });
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
    gap: 8px;
    margin: 6px 0;
  }

  .row-label {
    min-width: 65px;
    font-size: 11px;
  }

  input[type='color'] {
    width: 45px;
    height: 30px;
    padding: 0;
    border: 1px solid var(--border);
    background: var(--input-bg);
    cursor: pointer;
    border-radius: 10px;
  }

  input[type='range'] {
    flex: 1;
    height: 8px;
    -webkit-appearance: none;
    appearance: none;
    background: var(--bar-track);
    border-radius: 8px;
    outline: none;
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
    margin-top: calc((18px - 8px) / -2);
    transition: all 0.2s ease;
  }

  input[type='range']::-webkit-slider-thumb:hover {
    transform: scale(1.1);
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

  .value-display {
    min-width: 40px;
    text-align: center;
    font-weight: 600;
    font-size: 11px;
    padding: 4px 6px;
    background: var(--input-bg);
    border-radius: 8px;
  }

  label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: url('/cursors/light/hand.cur'), pointer;
  }
  
  input[type="checkbox"] {
    width: 18px;
    height: 18px;
    cursor: url('/cursors/light/hand.cur'), pointer !important;
  }

  .checkbox-row {
    margin: 8px 0;
    display: flex;
    align-items: center;
  }

  .color-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 8px 0;
  }

  .color-item {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  
  .color-item span {
    font-size: 11px;
  }

  .reset-btn {
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
  
  .reset-btn::after {
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

  .reset-btn:hover {
    opacity: 0.9;
    transform: translateY(-1px);
  }
</style>

<div class="group">
  <div class="title">{$t('Tray Icon Settings')}</div>
  
  <div class="checkbox-row">
    <label for="show-mem-usage">
      <input
        type="checkbox"
        id="show-mem-usage"
        checked={cfg?.tray.show_mem_usage ?? true}
        on:change={(e) => {
          const newValue = e.currentTarget.checked;
          updateTray({ show_mem_usage: newValue });
        }}
      />
      {$t('Show memory percentage in tray icon')}
    </label>
  </div>

  <div class="checkbox-row">
    <label for="transparent-bg">
      <input
        type="checkbox"
        id="transparent-bg"
        checked={cfg?.tray.transparent_bg}
        on:change={() => updateTray({ transparent_bg: !(cfg?.tray.transparent_bg) })}
      />
      {$t('Transparent background (numbers only)')}
    </label>
  </div>

  <div class="color-row">
    <div class="color-item">
      <span class="row-label">{$t('Text')}</span>
      <input
        type="color"
        value={cfg?.tray.text_color_hex}
        on:input={(e) => updateTray({ text_color_hex: e.currentTarget.value })}
      />
    </div>
    <div class="color-item">
      <span class="row-label">{$t('Background')}</span>
      <input
        type="color"
        value={cfg?.tray.background_color_hex}
        on:input={(e) => updateTray({ background_color_hex: e.currentTarget.value })}
        disabled={cfg?.tray.transparent_bg}
      />
    </div>
  </div>
  
  <div class="row">
    <span class="row-label">{$t('Warning')}</span>
    <input
      type="range"
      min="50"
      max="95"
      value={cfg?.tray.warning_level}
      on:input={(e) => updateTray({ warning_level: parseInt(e.currentTarget.value, 10) })}
    />
    <span class="value-display">{cfg?.tray.warning_level}%</span>
    <input
      type="color"
      value={cfg?.tray.warning_color_hex}
      on:input={(e) => updateTray({ warning_color_hex: e.currentTarget.value })}
      disabled={cfg?.tray.transparent_bg}
    />
  </div>
  
  <div class="row">
    <span class="row-label">{$t('Danger')}</span>
    <input
      type="range"
      min="60"
      max="100"
      value={cfg?.tray.danger_level}
      on:input={(e) => updateTray({ danger_level: parseInt(e.currentTarget.value, 10) })}
    />
    <span class="value-display">{cfg?.tray.danger_level}%</span>
    <input
      type="color"
      value={cfg?.tray.danger_color_hex}
      on:input={(e) => updateTray({ danger_color_hex: e.currentTarget.value })}
      disabled={cfg?.tray.transparent_bg}
    />
  </div>
  
  <div style="margin-top: 12px;">
    <button on:click={resetTrayColors} class="reset-btn">{$t('Reset Colors')}</button>
  </div>
</div>