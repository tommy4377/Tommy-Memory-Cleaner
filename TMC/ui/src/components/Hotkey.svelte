<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { config, updateConfig } from '../lib/store'
  import { registerHotkey } from '../lib/api'
  import type { Config } from '../lib/types'
  import { t } from '../i18n/index'

  let cfg: Config | null = null
  let unsub: (() => void) | null = null

  let modifiers = {
    ctrl: true,
    alt: true,
    shift: false,
  }
  let mainKey = 'N'
  let errorMsg = ''
  let successMsg = ''

  onMount(() => {
    unsub = config.subscribe((v) => {
      cfg = v
      if (v && v.hotkey) {
        parseHotkey(v.hotkey)
      }
    })
  })

  onDestroy(() => {
    if (unsub) unsub()
  })

  function parseHotkey(hotkey: string) {
    const parts = hotkey.toUpperCase().split('+')
    modifiers.ctrl = parts.includes('CTRL') || parts.includes('CONTROL')
    modifiers.alt = parts.includes('ALT')
    modifiers.shift = parts.includes('SHIFT')
    const nonModifiers = parts.filter((p) => !['CTRL', 'CONTROL', 'ALT', 'SHIFT'].includes(p))
    mainKey = nonModifiers[0] || 'N'
  }

  function buildHotkey(): string {
    const parts: string[] = []
    if (modifiers.ctrl) parts.push('Ctrl')
    if (modifiers.alt) parts.push('Alt')
    if (modifiers.shift) parts.push('Shift')
    if (mainKey) parts.push(mainKey.toUpperCase())
    return parts.join('+')
  }

  // NUOVA FUNZIONE - Aggiungi questa
  function validateMainKey(key: string): string {
    const upperKey = key.toUpperCase().trim()

    // F1-F12
    if (upperKey.match(/^F([1-9]|1[0-2])$/)) {
      return upperKey
    }

    // A-Z, 0-9
    if (upperKey.match(/^[A-Z0-9]$/)) {
      return upperKey
    }

    // Special keys
    const specialKeys = [
      'SPACE',
      'TAB',
      'ENTER',
      'ESCAPE',
      'BACKSPACE',
      'DELETE',
      'INSERT',
      'HOME',
      'END',
      'PAGEUP',
      'PAGEDOWN',
      'UP',
      'DOWN',
      'LEFT',
      'RIGHT',
      'PLUS',
      'MINUS',
    ]

    if (specialKeys.includes(upperKey)) {
      return upperKey
    }

    return ''
  }

  // NUOVA FUNZIONE - Aggiungi questa
  function onKeyInput(e: Event) {
    const target = e.target as HTMLInputElement // Cast esplicito
    let input = target.value

    // Security: Limita lunghezza e rimuovi caratteri pericolosi
    if (input.length > 50) {
      input = input.slice(0, 50)
      target.value = input
    }

    // Rimuovi caratteri potenzialmente pericolosi
    const dangerousPatterns = [
      '<',
      '>',
      '&',
      '"',
      "'",
      '/',
      '\\',
      ';',
      '(',
      ')',
      '{',
      '}',
      '[',
      ']',
    ]
    if (dangerousPatterns.some((pattern) => input.includes(pattern))) {
      errorMsg = 'Invalid characters in hotkey'
      return
    }

    const validated = validateMainKey(input)
    if (validated) {
      mainKey = validated
      errorMsg = ''
    } else if (input === '') {
      mainKey = ''
    }
  }

  async function apply() {
    errorMsg = ''
    successMsg = ''

    if (!mainKey) {
      errorMsg = 'Please select a main key'
      return
    }

    const hotkey = buildHotkey()

    try {
      await registerHotkey(hotkey)
      await updateConfig({ hotkey })
      successMsg = `Hotkey set: ${hotkey}`
      setTimeout(() => (successMsg = ''), 3000)
    } catch (e: any) {
      errorMsg = 'Failed to register. Try different combination.'
    }
  }

  function reset() {
    modifiers = { ctrl: true, alt: true, shift: false }
    mainKey = 'N'
    apply()
  }
</script>

<div class="group">
  <div class="title">{$t('Global Hotkey')}</div>

  <div class="modifiers">
    <button
      type="button"
      class="mod-box"
      class:active={modifiers.ctrl}
      on:click={() => (modifiers.ctrl = !modifiers.ctrl)}
    >
      CTRL
    </button>
    <button
      type="button"
      class="mod-box"
      class:active={modifiers.alt}
      on:click={() => (modifiers.alt = !modifiers.alt)}
    >
      ALT
    </button>
    <button
      type="button"
      class="mod-box"
      class:active={modifiers.shift}
      on:click={() => (modifiers.shift = !modifiers.shift)}
    >
      SHIFT
    </button>
  </div>

  <!-- MODIFICA QUI - Sostituisci l'input esistente con questo -->
  <input
    type="text"
    class="key-input"
    placeholder="Key (A-Z, 0-9, F1-F12)"
    value={mainKey}
    on:input={onKeyInput}
    on:blur={() => (mainKey = validateMainKey(mainKey))}
    maxlength="8"
  />

  <div class="buttons">
    <button on:click={reset}>Default</button>
    <button on:click={apply}>Apply</button>
  </div>

  {#if errorMsg}<div class="msg err">{errorMsg}</div>{/if}
  {#if successMsg}<div class="msg success">{successMsg}</div>{/if}
</div>

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

  .modifiers {
    display: flex;
    gap: 6px;
    margin-bottom: 6px;
  }

  .mod-box {
    flex: 1;
    padding: 8px 12px;
    border: 2px solid var(--border);
    border-radius: 10px;
    text-align: center;
    cursor: url('/cursors/light/hand.cur'), pointer;
    font-size: 11px;
    font-weight: 500;
    background: var(--bg);
    color: var(--fg);
    transition: all 0.2s;
  }

  html[data-theme='dark'] .mod-box {
    cursor: url('/cursors/dark/hand.cur'), pointer;
  }

  .mod-box.active {
    background: var(--btn-bg);
    color: white;
    border-color: var(--btn-bg);
  }

  .key-input {
    width: 100%;
    padding: 8px;
    border: 2px solid var(--border);
    border-radius: 10px;
    text-align: center;
    font-size: 11px;
    text-transform: uppercase;
    background: var(--bg);
    color: var(--fg);
    margin-bottom: 6px;
  }

  .buttons {
    display: flex;
    gap: 6px;
  }

  button {
    flex: 1;
    padding: 7px;
    background: var(--btn-bg);
    color: white;
    border: none;
    border-radius: 10px;
    font-size: 11px;
    cursor: pointer;
    position: relative;
    overflow: hidden;
  }

  /* Shimmer solo per i bottoni Apply/Default, non per i mod-box */
  button:not(.mod-box)::after {
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
  }

  /* Shimmer solo per i mod-box attivi */
  .mod-box.active {
    position: relative;
    overflow: hidden;
  }

  .mod-box.active::after {
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
  }

  @keyframes shimmer {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(100%);
    }
  }

  .msg {
    margin-top: 4px;
    font-size: 9px;
  }
  .err {
    color: #ff5f57;
  }
  .success {
    color: #28c840;
  }
</style>
