<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte'
  import { progress } from '../lib/store'
  import { t } from '../i18n/index'

  const dispatch = createEventDispatcher<{ optimize: void }>()

  let p: any = null
  let unsub: (() => void) | null = null
  let percent = 0
  let statusText = ''

  onMount(() => {
    unsub = progress.subscribe((v) => {
      p = v
    })
  })

  // Calcola percentuale e testo status in modo reattivo
  $: percent = p && p.total > 0 ? Math.floor((p.value / p.total) * 100) : 0

  // Rendi statusText reattivo sia al progress che alla lingua
  $: statusText = (() => {
    if (p?.running && p?.step) {
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
        Completed: $t('Done'),
      }

      const translatedStep = stepTranslations[p.step] || p.step
      return `${p.value}/${p.total} - ${translatedStep} (${percent}%)`
    } else if (p?.step === 'Completed' || p?.step === 'Done') {
      return $t('Done')
    } else {
      return $t('Ready')
    }
  })()

  onDestroy(() => {
    if (unsub) {
      unsub()
      unsub = null
    }
  })

  function handleOptimize() {
    if (!p?.running) {
      dispatch('optimize')
    }
  }
</script>

<div class="footer">
  <button on:click={handleOptimize} disabled={p?.running}>
    {p?.running ? $t('Optimizing...') : $t('Optimize')}
  </button>

  <div class="progress">
    <div class="fill" class:active={p?.running} style="width: {percent}%"></div>
  </div>

  <div class="status" class:active={p?.running}>
    {statusText}
  </div>
</div>

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
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.9;
    }
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

  /* Effetto shimmer per il bottone optimize */
  button:not(:disabled)::after {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: linear-gradient(
      135deg,
      transparent 30%,
      rgba(255, 255, 255, 0.1) 50%,
      transparent 70%
    );
    animation: shimmer 2s infinite;
    pointer-events: none;
    border-radius: 12px;
  }

  @keyframes shimmer {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(100%);
    }
  }

  html[data-theme='dark'] button {
    cursor: url('/cursors/dark/hand.cur'), pointer;
  }

  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    background: var(--bar-track);
  }

  /* Rimuovi shimmer quando disabled */
  button:disabled::after {
    display: none;
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
