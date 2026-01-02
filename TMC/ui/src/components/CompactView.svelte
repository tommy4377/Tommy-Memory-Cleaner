<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { memory, config, progress } from '../lib/store'
  import { optimizeAsync } from '../lib/api'
  import { Reason, AreasFlag } from '../lib/types'
  import type { MemoryInfo, Config } from '../lib/types'
  import { t } from '../i18n/index'
  import { areasForProfile } from '../lib/profiles'
  import MemoryStats from './MemoryStats.svelte'

  let memInfo: MemoryInfo | null = null
  let cfg: Config | null = null
  let prog: any = null
  let memUnsub: (() => void) | null = null
  let cfgUnsub: (() => void) | null = null
  let progUnsub: (() => void) | null = null

  // Compact mode prop
  export let compact = true

  onMount(() => {
    memUnsub = memory.subscribe((v) => (memInfo = v))
    cfgUnsub = config.subscribe((v) => (cfg = v))
    // FIX: Usa lo store progress invece di una variabile locale per mantenere lo stato durante il cambio di vista
    progUnsub = progress.subscribe((v) => (prog = v))
  })

  onDestroy(() => {
    if (memUnsub) memUnsub()
    if (cfgUnsub) cfgUnsub()
    if (progUnsub) progUnsub()
  })

  async function optimize() {
    // FIX: Usa prog?.running invece di optimizing locale per mantenere lo stato
    if (prog?.running || !cfg) {
      console.warn('Cannot optimize: running=', prog?.running, 'cfg=', cfg)
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

  // Helper per ottenere il testo del bottone tradotto (reattivo)
  $: buttonText = prog?.running ? $t('Optimizing...') : $t('Optimize')
</script>

<div class="compact">
  <div class="left-section">
    <MemoryStats compact={compact} />
  </div>
  <div class="right-section" class:compact-right={compact}>
    <div class="bar">
      <div
        class="fill"
        class:warning={memInfo &&
          memInfo.physical.used.percentage >= 80 &&
          memInfo.physical.used.percentage < 90}
        class:danger={memInfo && memInfo.physical.used.percentage >= 90}
        style="width: {memInfo?.physical.used.percentage ?? 0}%"
      >
        <span class="percent">{memInfo?.physical.used.percentage ?? 0}%</span>
      </div>
    </div>
    <button on:click={optimize} disabled={prog?.running}>
      {buttonText}
    </button>
  </div>
</div>

<style>
  .compact {
    padding: 16px 20px;
    display: flex;
    align-items: center;
    gap: 16px;
    height: calc(100% - 36px);
  }

  .left-section {
    flex: 1;
    min-width: 0;
  }

  .right-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: stretch;
    min-width: 200px;
  }

  .right-section.compact-right {
    min-width: 150px;
  }

  .bar {
    height: 28px;
    background: var(--bar-track);
    border-radius: 14px;
    position: relative;
    overflow: hidden;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .fill {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    background: var(--bar-fill);
    transition:
      width 0.3s ease,
      background 0.3s ease;
    border-radius: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }

  .fill.warning {
    background: linear-gradient(90deg, #ff9900, #ff6600);
  }

  .fill.danger {
    background: linear-gradient(90deg, #ff3030, #cc0000);
  }

  .percent {
    color: white;
    font-weight: 600;
    font-size: 13px;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
  }

  button {
    background: var(--btn-bg);
    color: white;
    border: none;
    padding: 8px 20px;
    border-radius: 14px;
    cursor: pointer;
    font-weight: 600;
    font-size: 13px;
    min-width: fit-content;
    width: auto;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
    transition: all 0.2s;
    position: relative;
    overflow: hidden;
    white-space: nowrap;
    text-align: center;
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
    border-radius: 14px;
  }

  @keyframes shimmer {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(100%);
    }
  }

  button:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 3px 6px rgba(0, 0, 0, 0.15);
  }

  button:active:not(:disabled) {
    transform: translateY(0);
  }

  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    background: linear-gradient(135deg, #6a6a6a, #4a4a4a);
    animation: pulse 1.5s infinite;
  }

  /* Rimuovi shimmer quando disabled */
  button:disabled::after {
    display: none;
  }

  @keyframes pulse {
    0% {
      opacity: 1;
    }
    50% {
      opacity: 0.6;
    }
    100% {
      opacity: 1;
    }
  }
</style>
