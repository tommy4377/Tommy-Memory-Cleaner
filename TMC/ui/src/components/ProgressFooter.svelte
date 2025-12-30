<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  import { progress } from '../lib/store';
  import { t } from '../i18n/index';

  const dispatch = createEventDispatcher<{ optimize: void }>();

  let p: any = null;
  let unsub: (() => void) | null = null;
  let percent = 0;
  let statusText = '';

  onMount(() => {
    unsub = progress.subscribe(v => {
      p = v;
      // Calcola percentuale
      percent = v && v.total > 0 ? Math.floor((v.value / v.total) * 100) : 0;
      
      // Genera testo status
      if (v?.running && v?.step) {
        // Traduci i nomi delle aree
        const stepTranslations: Record<string, string> = {
          'Working Set': $t('Working Set'),
          'Modified Page List': $t('Modified Pages'),
          'Standby List': $t('Standby List'),
          'Standby List (Low Priority)': $t('Low Priority Standby'),
          'System File Cache': $t('System Cache'),
          'Combined Page List': $t('Combined Pages'),
          'Modified File Cache': $t('File Cache'),
          'Registry Cache': $t('Registry Cache'),
          'Completed': $t('Done')
        };
        
        const translatedStep = stepTranslations[v.step] || v.step;
        statusText = `${v.value}/${v.total} - ${translatedStep} (${percent}%)`;
      } else if (v?.step === 'Completed' || v?.step === 'Done') {
        statusText = $t('Done');
      } else {
        statusText = $t('Ready');
      }
    });
  });

  onDestroy(() => {
    if (unsub) {
      unsub();
      unsub = null;
    }
  });
  
  function handleOptimize() {
    if (!p?.running) {
      dispatch('optimize');
    }
  }
</script>

<style>
  .footer {
    display: grid;
    grid-template-columns: 120px 1fr 200px;
    align-items: center;
    gap: 16px;
    background: var(--card);
    padding: 14px 16px;
    border-radius: 12px;
  }

  .progress {
    height: 12px;
    background: var(--bar-track);
    border-radius: 12px;
    overflow: hidden;
    position: relative;
    width: 100%;
    min-width: 0;
    flex: 1;
  }

  .fill {
    height: 100%;
    background: var(--bar-fill);
    transition: width 0.4s cubic-bezier(0.4, 0, 0.2, 1);
    border-radius: 12px;
    position: absolute;
    left: 0;
    top: 0;
    will-change: width;
  }
  
  .fill.active {
    background: linear-gradient(90deg, var(--btn-bg), var(--bar-fill));
    animation: pulse 2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.9; }
  }

  button {
    background: var(--btn-bg);
    color: var(--btn-fg);
    border: none;
    padding: 10px 24px;
    border-radius: 12px;
    cursor: url('/cursors/light/hand.cur'), pointer;
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    white-space: nowrap;
    text-align: center;
    min-width: fit-content;
    width: auto;
  }
  
  
  html[data-theme="dark"] button {
    cursor: url('/cursors/dark/hand.cur'), pointer;
    font-weight: 500;
    font-size: 13px;
    min-width: auto;
    width: auto;
    transition: all 0.2s;
    white-space: nowrap;
  }

  button:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 3px 8px rgba(0,0,0,0.2);
  }
  
  button:active:not(:disabled) {
    transform: translateY(0);
  }

  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    background: var(--bar-track);
  }

  .status {
    min-width: 200px;
    text-align: right;
    font-size: 12px;
    font-weight: 450;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  
  .status.active {
    color: var(--btn-bg);
    font-weight: 500;
  }

  @media (max-width: 600px) {
    .footer {
      grid-template-columns: 100px 1fr 150px;
      gap: 12px;
      padding: 12px;
    }
    
    button {
      min-width: 100px;
      padding: 8px 16px;
      font-size: 12px;
    }
    
    .status {
      min-width: 150px;
      font-size: 11px;
    }
  }
</style>

<div class="footer">
  <button on:click={handleOptimize} disabled={p?.running}>
    {p?.running ? $t('Optimizing...') : $t('Optimize')}
  </button>
  
  <div class="progress">
    <div 
      class="fill" 
      class:active={p?.running}
      style="width: {percent}%"
    ></div>
  </div>
  
  <div class="status" class:active={p?.running}>
    {statusText}
  </div>
</div>