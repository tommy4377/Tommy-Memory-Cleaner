<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { config, updateConfig } from '../lib/store';
  import { setAlwaysOnTop, runOnStartup, setPriority } from '../lib/api';
  import type { Config, Priority } from '../lib/types';
  import { t, setLanguage } from '../i18n/index';
  import CustomSelect from './CustomSelect.svelte';
  
  let cfg: Config | null = null;
  let unsub: (() => void) | null = null;

  onMount(() => {
    unsub = config.subscribe((v) => (cfg = v));
  });

  onDestroy(() => {
    if (unsub) unsub();
  });

  async function toggle(key: keyof Config) {
    if (!cfg) return;
    const value = !(cfg as any)[key];
    
    if (key === 'always_on_top') {
      await setAlwaysOnTop(value);
      await updateConfig({ [key]: value } as any);
    } else if (key === 'run_on_startup') {
      // runOnStartup already saves the config with the actual state from the system
      // Just call it - it will update both system and config
      await runOnStartup(value);
      // Reload config from backend to sync the UI with actual state
      const { getConfig } = await import('../lib/api');
      const updatedConfig = await getConfig();
      if (updatedConfig) {
        config.set(updatedConfig);
      }
    } else {
      await updateConfig({ [key]: value } as any);
    }
  }

  async function onPriorityChange(val: string) {
    await setPriority(val as Priority);
    await updateConfig({ run_priority: val as Priority });
  }

  async function onLangChange(code: string) {
    setLanguage(code as 'en' | 'it');
    await updateConfig({ language: code });
  }

  async function onThemeChange(theme: string) {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('tmc_theme', theme);
    await updateConfig({ theme });
  }
  
  $: priorityOptions = [
    { value: 'Low', label: $t('Low') },
    { value: 'Normal', label: $t('Normal') },
    { value: 'High', label: $t('High') }
  ];
  
  const languageOptions = [
    { value: 'en', label: 'English' },
    { value: 'it', label: 'Italiano' },
    { value: 'es', label: 'Español' },
    { value: 'fr', label: 'Français' },
    { value: 'pt', label: 'Português' },
    { value: 'de', label: 'Deutsch' },
    { value: 'ar', label: 'العربية' },
    { value: 'ja', label: '日本語' },
    { value: 'zh', label: '中文' }
  ];
  
  $: themeOptions = [
    { value: 'light', label: $t('Light') },
    { value: 'dark', label: $t('Dark') }
  ];
</script>

<style>
  .group { 
    background: var(--card); 
    border-radius: 12px; 
    padding: 12px; 
  }
  
  .row { 
    display: flex; 
    align-items: center; 
    justify-content: space-between; 
    gap: 10px; 
    margin: 8px 0;
    min-height: 28px;
  }
  
  label { 
    display: flex; 
    align-items: center; 
    gap: 8px; 
    cursor: url('/cursors/light/hand.cur'), pointer; 
    font-size: 13px;
    font-weight: 450;
  }
  
  html[data-theme="dark"] label {
    cursor: url('/cursors/dark/hand.cur'), pointer;
  }
  
  input[type="checkbox"] {
    width: 18px;
    height: 18px;
    cursor: url('/cursors/light/hand.cur'), pointer !important;
  }
  
  html[data-theme="dark"] input[type="checkbox"] {
    cursor: url('/cursors/dark/hand.cur'), pointer !important;
  }
  
  .select-row {
    font-size: 13px;
    font-weight: 450;
  }
  
  .select-wrapper {
    min-width: 110px;
  }
</style>

<div class="group">
  <!-- Checkbox rows -->
  <div class="row">
    <label>
      <input type="checkbox" checked={cfg?.always_on_top} on:change={() => toggle('always_on_top')} /> 
      {$t('Always on top')}
    </label>
  </div>
  
  <div class="row">
    <label>
      <input type="checkbox" checked={cfg?.auto_update} on:change={() => toggle('auto_update')} /> 
      {$t('Auto update')}
    </label>
  </div>
  
  <div class="row">
    <label>
      <input type="checkbox" checked={cfg?.close_after_opt} on:change={() => toggle('close_after_opt')} /> 
      {$t('Close after optimization')}
    </label>
  </div>
  
  <div class="row">
    <label>
      <input type="checkbox" checked={cfg?.minimize_to_tray} on:change={() => toggle('minimize_to_tray')} /> 
      {$t('Close to notification area')}
    </label>
  </div>
  
  <div class="row">
    <label>
      <input type="checkbox" checked={cfg?.show_opt_notifications} on:change={() => toggle('show_opt_notifications')} /> 
      {$t('Show optimization notifications')}
    </label>
  </div>
  
  <div class="row">
    <label>
      <input type="checkbox" checked={cfg?.run_on_startup} on:change={() => toggle('run_on_startup')} /> 
      {$t('Run on startup')}
    </label>
  </div>
  
  <!-- Select rows -->
  <div class="row select-row">
    <div>{$t('Priority')}</div>
    <div class="select-wrapper">
      <CustomSelect
        value={cfg?.run_priority || 'Normal'}
        options={priorityOptions}
        on:change={(e) => onPriorityChange(e.detail)}
      />
    </div>
  </div>
  
  <div class="row select-row">
    <div>{$t('Language')}</div>
    <div class="select-wrapper">
      <CustomSelect
        value={cfg?.language || 'en'}
        options={languageOptions}
        on:change={(e) => onLangChange(e.detail)}
      />
    </div>
  </div>

  <div class="row select-row">
    <div>{$t('Theme')}</div>
    <div class="select-wrapper">
      <CustomSelect
        value={cfg?.theme || 'dark'}
        options={themeOptions}
        on:change={(e) => onThemeChange(e.detail)}
      />
    </div>
  </div>
</div>