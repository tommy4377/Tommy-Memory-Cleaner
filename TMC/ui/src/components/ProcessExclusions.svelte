<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { config, updateConfig } from '../lib/store'
  import { listProcessNames, getCriticalProcesses } from '../lib/api'
  import type { Config } from '../lib/types'
  import { t } from '../i18n/index'

  let cfg: Config | null = null
  let unsub: (() => void) | null = null
  let candidates: string[] = []
  let selected = ''
  let showDropdown = false
  let filtered: string[] = []
  let inputEl: HTMLInputElement
  let selectedIndex = -1
  let dropdownEl: HTMLDivElement
  let criticalProcesses: Set<string> = new Set()

  onMount(() => {
    unsub = config.subscribe((v) => (cfg = v))

    // Carica prima i processi critici dal backend
    Promise.all([listProcessNames(), getCriticalProcesses()])
      .then(([processes, critical]) => {
        // Memorizza i processi critici in un Set per lookup veloce
        criticalProcesses = new Set(critical.map((p) => p.toLowerCase().replace('.exe', '')))

        const uniqueProcesses = new Set<string>()

        for (const process of processes) {
          const cleanName = process.toLowerCase().replace('.exe', '')
          const displayName = process.endsWith('.exe') ? process : `${process}.exe`

          // Salta se è un processo critico
          if (
            criticalProcesses.has(cleanName) ||
            criticalProcesses.has(displayName.toLowerCase())
          ) {
            continue
          }

          // Salta processi di sistema Windows comuni
          if (
            cleanName.startsWith('windows') ||
            cleanName.startsWith('microsoft') ||
            cleanName.includes('system32') ||
            cleanName.includes('syswow64')
          ) {
            continue
          }

          uniqueProcesses.add(displayName)
        }

        candidates = Array.from(uniqueProcesses).sort((a, b) =>
          a.toLowerCase().localeCompare(b.toLowerCase()),
        )

        updateFilteredList()
      })
      .catch((err) => {
        console.error('Failed to load processes:', err)
        candidates = []
        filtered = []
      })

    // Gestione click fuori dal dropdown
    const handleClick = (e: MouseEvent) => {
      if (!e.target || !(e.target as Element).closest('.dropdown-container')) {
        showDropdown = false
        selectedIndex = -1
      }
    }

    document.addEventListener('click', handleClick)

    return () => {
      document.removeEventListener('click', handleClick)
    }
  })

  onDestroy(() => {
    if (unsub) unsub()
  })

  function updateFilteredList() {
    const searchTerm = selected.toLowerCase().trim()
    const excluded = cfg?.process_exclusion_list || []

    // Combina processi esclusi (in alto) e candidati disponibili
    let combinedList: string[] = []

    // Prima aggiungi i processi esclusi che matchano la ricerca
    if (searchTerm) {
      combinedList = excluded.filter((p) => p.toLowerCase().includes(searchTerm))
    } else {
      combinedList = [...excluded]
    }

    // Poi aggiungi i candidati non ancora esclusi
    const availableProcesses = candidates.filter((p) => {
      const processName = p.toLowerCase()

      // Non mostrare se già escluso
      if (excluded.some((ex) => ex.toLowerCase() === processName)) {
        return false
      }

      // Filtra per ricerca
      if (searchTerm && !processName.includes(searchTerm)) {
        return false
      }

      return true
    })

    // Unisci le due liste
    filtered = [...combinedList, ...availableProcesses.slice(0, 50)]
  }

  function filterProcesses(value: string) {
    selected = value
    selectedIndex = -1
    updateFilteredList()
  }

  function isExcluded(processName: string): boolean {
    return (
      cfg?.process_exclusion_list.some((p) => p.toLowerCase() === processName.toLowerCase()) ||
      false
    )
  }

  function selectProcess(name: string) {
    if (!isExcluded(name)) {
      selected = name
      showDropdown = false
      selectedIndex = -1
      inputEl?.focus()
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!showDropdown && e.key !== 'ArrowDown') return

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault()
        if (!showDropdown) {
          showDropdown = true
          updateFilteredList()
        } else {
          selectedIndex = Math.min(selectedIndex + 1, filtered.length - 1)
          scrollToSelected()
        }
        break
      case 'ArrowUp':
        e.preventDefault()
        selectedIndex = Math.max(selectedIndex - 1, -1)
        scrollToSelected()
        break
      case 'Enter':
        e.preventDefault()
        if (selectedIndex >= 0 && selectedIndex < filtered.length) {
          const item = filtered[selectedIndex]
          if (!isExcluded(item)) {
            selectProcess(item)
          }
        } else if (selected.trim()) {
          add()
        }
        break
      case 'Escape':
        e.preventDefault()
        showDropdown = false
        selectedIndex = -1
        break
    }
  }

  function scrollToSelected() {
    if (selectedIndex >= 0 && dropdownEl) {
      const items = dropdownEl.querySelectorAll('.dropdown-item')
      if (items[selectedIndex]) {
        items[selectedIndex].scrollIntoView({ block: 'nearest' })
      }
    }
  }

  async function add() {
    if (!cfg || !selected.trim()) return

    let processName = selected.trim()

    // Security: Validazione lunghezza massima
    if (processName.length > 100) {
      processName = processName.slice(0, 100)
      selected = processName
    }

    // Security: Rimuovi caratteri pericolosi
    const dangerousPatterns = [
      '<',
      '>',
      '&',
      '"',
      "'",
      '/',
      '\\',
      ';',
      '|',
      '(',
      ')',
      '{',
      '}',
      '[',
      ']',
      '`',
      '$',
    ]
    if (dangerousPatterns.some((pattern) => processName.includes(pattern))) {
      selected = ''
      showDropdown = false
      selectedIndex = -1
      return
    }

    // Security: Verifica pattern di injection
    const injectionPatterns = [
      'javascript:',
      'data:',
      'vbscript:',
      'file:',
      'ftp:',
      'http',
      'https',
    ]
    if (injectionPatterns.some((pattern) => processName.toLowerCase().includes(pattern))) {
      selected = ''
      showDropdown = false
      selectedIndex = -1
      return
    }

    const cleanName = processName.toLowerCase()

    // Verifica che non sia un processo critico
    if (criticalProcesses.has(cleanName.replace('.exe', '')) || criticalProcesses.has(cleanName)) {
      selected = ''
      return
    }

    // Verifica che non sia già nella lista
    const existing = cfg.process_exclusion_list.map((x) => x.toLowerCase())
    if (!existing.includes(cleanName)) {
      const next = [...cfg.process_exclusion_list, processName]
      await updateConfig({ process_exclusion_list: next })
      selected = ''
      showDropdown = false
      selectedIndex = -1
      updateFilteredList()
    }
  }

  async function remove(name: string, e?: MouseEvent) {
    if (e) {
      e.stopPropagation()
      e.preventDefault()
    }
    if (!cfg) return
    const next = cfg.process_exclusion_list.filter((x) => x.toLowerCase() !== name.toLowerCase())
    await updateConfig({ process_exclusion_list: next })
    updateFilteredList()
  }
</script>

<div class="group">
  <div class="header">
    <div class="title">{$t('Process Exclusions')}</div>
    <div class="subtitle">{$t('These processes will not have their memory optimized')}</div>
    <div class="row">
      <div class="dropdown-container">
        <input
          bind:this={inputEl}
          placeholder={$t('Type process name...')}
          bind:value={selected}
          on:input={(e) => filterProcesses(e.currentTarget.value)}
          on:focus={() => {
            showDropdown = true
            updateFilteredList()
          }}
          on:keydown={handleKeydown}
          role="combobox"
          aria-expanded={showDropdown}
          aria-controls="process-dropdown"
          autocomplete="off"
        />
        {#if showDropdown && filtered.length > 0}
          <div bind:this={dropdownEl} id="process-dropdown" class="dropdown" role="listbox">
            {#each filtered as item, index}
              {#if isExcluded(item)}
                <div class="dropdown-item excluded" role="option" aria-selected={false}>
                  <span class="process-name">
                    {item}
                    <span class="excluded-badge">Excluded</span>
                  </span>
                  <button
                    class="remove-x"
                    on:click={(e) => remove(item, e)}
                    title="Remove exclusion"
                  >
                    ×
                  </button>
                </div>
              {:else}
                <button
                  type="button"
                  class="dropdown-item"
                  class:selected={index === selectedIndex}
                  role="option"
                  tabindex="-1"
                  aria-selected={index === selectedIndex}
                  on:click={() => selectProcess(item)}
                >
                  <span class="process-name">{item}</span>
                </button>
              {/if}

              {#if isExcluded(item) && index < filtered.length - 1 && !isExcluded(filtered[index + 1])}
                <div class="divider"></div>
              {/if}
            {/each}
          </div>
        {/if}
      </div>
      <button class="add-btn" on:click={add} disabled={!selected.trim()}>
        {$t('Add')}
      </button>
    </div>

    {#if cfg && cfg.process_exclusion_list.length > 0}
      <div class="exclusion-list">
        {#each cfg.process_exclusion_list as p}
          <div class="exclusion-item">
            <span>{p}</span>
            <button class="remove-btn" on:click={() => remove(p)} title="Remove {p}"> × </button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .group {
    background: var(--card);
    border-radius: 12px;
    padding: 12px;
    display: flex;
    flex-direction: column;
  }

  .header {
    flex-shrink: 0;
  }

  .title {
    font-weight: 500;
    margin-bottom: 4px;
    font-size: 13px;
  }

  .subtitle {
    font-size: 11px;
    opacity: 0.7;
    margin-bottom: 10px;
  }

  .dropdown-container {
    position: relative;
    flex: 1; /* Takes all available space */
    min-width: 200px; /* Minimum width for shorter text */
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  input {
    flex: 1;
    width: 100%; /* Ensure full width */
    min-width: 0; /* Allows input to shrink */
    padding: 7px 10px;
    font-size: 13px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: var(--input-bg);
    color: var(--fg);
    text-overflow: ellipsis; /* Add ellipsis for overflow */
  }

  /* Responsive placeholder using CSS */
  input::placeholder {
    text-overflow: ellipsis;
    overflow: hidden;
    white-space: nowrap;
  }

  /* For smaller screens, adjust font size */
  @media (max-width: 480px) {
    input {
      font-size: 12px;
      padding: 6px 8px;
    }
  }

  input:focus {
    outline: none;
    border-color: var(--btn-bg);
  }

  .dropdown {
    position: absolute;
    bottom: 100%;
    left: 0;
    right: 0; /* Uses full width of container */
    min-width: 300px; /* Minimum width */
    max-height: 350px;
    overflow-y: auto;
    overflow-x: hidden;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 10px;
    margin-bottom: 4px;
    z-index: 1000;
    box-shadow: 0 -6px 20px rgba(0, 0, 0, 0.2);
  }

  .dropdown::-webkit-scrollbar {
    width: 8px;
  }

  .dropdown::-webkit-scrollbar-track {
    background: var(--bar-track);
    border-radius: 3px;
  }

  .dropdown::-webkit-scrollbar-thumb {
    background: var(--bar-fill);
    border-radius: 3px;
  }

  .dropdown::-webkit-scrollbar-thumb:hover {
    background: var(--btn-bg);
  }

  .dropdown-item {
    padding: 10px 14px; /* Aumentato da 6px 10px */
    font-size: 13px; /* Aumentato da 11px */
    cursor: pointer;
    transition: background-color 0.1s;
    background: transparent;
    text-align: left;
    width: 100%;
    border: none;
    color: var(--fg);
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 36px; /* Aggiunto - altezza minima per ogni item */
  }

  .dropdown-item.excluded {
    background: var(--bar-track);
    cursor: default;
  }

  .dropdown-item:not(.excluded):hover,
  .dropdown-item.selected:not(.excluded) {
    background: var(--btn-bg);
    color: white;
  }

  .dropdown-item.excluded:hover {
    background: var(--bar-track);
  }

  .process-name {
    display: flex;
    align-items: center;
    gap: 8px; /* Aumentato da 6px */
    flex: 1;
    font-weight: 450; /* Aggiunto - testo più leggibile */
  }

  .excluded-badge {
    font-size: 10px; /* Aumentato da 9px */
    padding: 2px 6px; /* Aumentato padding */
    background: var(--btn-bg);
    color: white;
    border-radius: 8px; /* Aumentato da 4px */
    font-weight: 600; /* Aumentato peso font */
  }

  .remove-x {
    width: 20px; /* Aumentato da 16px */
    height: 20px; /* Aumentato da 16px */
    border-radius: 50%;
    background: #ff5f57;
    color: white;
    border: none;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s;
    font-size: 14px; /* Aumentato da 12px */
    line-height: 1;
    padding: 0;
    flex-shrink: 0;
    margin-left: 8px; /* Aggiunto margine */
  }

  .remove-x:hover {
    background: #ff3030;
    transform: scale(1.1);
  }

  .divider {
    height: 1px;
    background: var(--border);
    margin: 2px 0;
  }

  button.add-btn {
    background: var(--btn-bg);
    color: var(--btn-fg);
    border: none;
    border-radius: 8px;
    padding: 5px 12px;
    font-size: 11px;
    position: relative;
    overflow: hidden;
    cursor: pointer;
    transition: opacity 0.2s;
  }

  button.add-btn::after {
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

  button.add-btn:hover {
    opacity: 0.9;
  }

  button.add-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .exclusion-list {
    margin-top: 8px;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .exclusion-item {
    display: flex;
    align-items: center;
    gap: 4px;
    background: var(--bar-track);
    padding: 3px 6px 3px 10px;
    border-radius: 12px;
    font-size: 11px;
  }

  .exclusion-item .remove-btn {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #ff5f57;
    color: white;
    border: none;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    padding: 0;
    transition: all 0.2s;
  }

  .exclusion-item .remove-btn:hover {
    background: #ff3030;
    transform: scale(1.15);
  }
</style>
