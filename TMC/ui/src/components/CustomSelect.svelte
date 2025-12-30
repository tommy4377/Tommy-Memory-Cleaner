<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  
  export let value: string = '';
  export let options: Array<{ value: string; label: string }> = [];
  export let placeholder: string = 'Select...';
  export let disabled: boolean = false;
  export let noShimmer: boolean = false;
  
  const dispatch = createEventDispatcher<{ change: string }>();
  
  let isOpen = false;
  let selectedIndex = -1;
  let dropdownEl: HTMLDivElement;
  let buttonEl: HTMLButtonElement;
  
  $: selectedLabel = options.find(opt => opt.value === value)?.label || placeholder;
  $: selectedIndex = options.findIndex(opt => opt.value === value);
  
  function toggle() {
    if (disabled) return;
    isOpen = !isOpen;
    if (isOpen) {
      setTimeout(() => {
        scrollToSelected();
        adjustDropdownPosition();
      }, 10);
    }
  }
  
  function adjustDropdownPosition() {
    if (!dropdownEl || !buttonEl) return;
    
    const rect = buttonEl.getBoundingClientRect();
    const viewportHeight = window.innerHeight;
    const viewportWidth = window.innerWidth;
    const dropdownHeight = 200; // max-height
    const dropdownWidth = rect.width;
    
    // Calcola la posizione
    let top = rect.bottom + 4;
    let left = rect.left;
    
    // Se va fuori in basso, aprilo verso l'alto
    if (rect.bottom + dropdownHeight > viewportHeight && rect.top > dropdownHeight) {
      top = rect.top - dropdownHeight - 4;
      dropdownEl.classList.add('upward');
    } else {
      dropdownEl.classList.remove('upward');
    }
    
    // Assicurati che non vada fuori a destra
    if (left + dropdownWidth > viewportWidth) {
      left = viewportWidth - dropdownWidth - 8;
    }
    
    // Assicurati che non vada fuori a sinistra
    if (left < 8) {
      left = 8;
    }
    
    dropdownEl.style.position = 'fixed';
    dropdownEl.style.top = `${top}px`;
    dropdownEl.style.left = `${left}px`;
    dropdownEl.style.width = `${dropdownWidth}px`;
  }
  
  function select(optionValue: string) {
    if (value !== optionValue) {
      value = optionValue;
      dispatch('change', optionValue);
    }
    isOpen = false;
  }
  
  function handleKeydown(e: KeyboardEvent) {
    if (disabled) return;
    
    switch (e.key) {
      case 'Enter':
      case ' ':
        if (!isOpen) {
          e.preventDefault();
          toggle();
        } else if (selectedIndex >= 0) {
          e.preventDefault();
          select(options[selectedIndex].value);
        }
        break;
      case 'ArrowDown':
        e.preventDefault();
        if (!isOpen) {
          toggle();
        } else {
          selectedIndex = Math.min(selectedIndex + 1, options.length - 1);
          scrollToSelected();
        }
        break;
      case 'ArrowUp':
        e.preventDefault();
        if (isOpen) {
          selectedIndex = Math.max(selectedIndex - 1, 0);
          scrollToSelected();
        }
        break;
      case 'Escape':
        e.preventDefault();
        isOpen = false;
        buttonEl?.focus();
        break;
      case 'Tab':
        isOpen = false;
        break;
    }
  }
  
  function scrollToSelected() {
    if (dropdownEl && selectedIndex >= 0) {
      const items = dropdownEl.querySelectorAll('.option-item');
      if (items[selectedIndex]) {
        items[selectedIndex].scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      }
    }
  }
  
  function handleClickOutside(e: MouseEvent) {
    if (dropdownEl && !dropdownEl.contains(e.target as Node) && 
        buttonEl && !buttonEl.contains(e.target as Node)) {
      isOpen = false;
    }
  }
  
  onMount(() => {
    document.addEventListener('click', handleClickOutside);
    return () => {
      document.removeEventListener('click', handleClickOutside);
    };
  });
  
  onDestroy(() => {
    document.removeEventListener('click', handleClickOutside);
  });
</script>

<div class="custom-select" class:open={isOpen} class:disabled={disabled} class:no-shimmer={noShimmer}>
  <button
    bind:this={buttonEl}
    type="button"
    class="select-button"
    on:click={toggle}
    on:keydown={handleKeydown}
    disabled={disabled}
    aria-haspopup="listbox"
    aria-expanded={isOpen}
    aria-label={selectedLabel}
  >
    <span class="select-value">{selectedLabel}</span>
    <svg class="select-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <polyline points="6 9 12 15 18 9"></polyline>
    </svg>
  </button>
  
  {#if isOpen}
    <div bind:this={dropdownEl} class="dropdown-menu" class:no-shimmer={noShimmer} class:setup-no-shimmer={noShimmer} role="listbox">
      {#each options as option, index}
        <button
          type="button"
          class="option-item"
          class:selected={option.value === value}
          class:hovered={index === selectedIndex && option.value !== value}
          class:no-shimmer={noShimmer}
          class:setup-no-shimmer={noShimmer}
          on:click={() => select(option.value)}
          role="option"
          aria-selected={option.value === value}
        >
          {option.label}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .custom-select {
    position: relative;
    min-width: 110px;
  }
  
  .select-button {
    width: 100%;
    padding: 8px 12px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--input-bg);
    color: var(--fg);
    font-size: 12px;
    font-weight: 450;
    cursor: url('/cursors/light/hand.cur'), pointer;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    transition: all 0.2s;
    text-align: left;
  }
  
  html[data-theme="dark"] .select-button {
    cursor: url('/cursors/dark/hand.cur'), pointer;
  }
  
  .select-button:hover:not(:disabled) {
    border-color: var(--input-focus);
  }
  
  .select-button:focus {
    outline: none;
    border-color: var(--input-focus);
  }
  
  [data-theme="light"] .select-button:focus {
    box-shadow: 0 0 0 3px rgba(154, 138, 114, 0.15);
  }
  
  html[data-theme="dark"] .select-button:focus {
    box-shadow: 0 0 0 3px rgba(10, 132, 255, 0.15);
  }
  
  .select-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  .select-value {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  
  .select-icon {
    flex-shrink: 0;
    transition: transform 0.2s;
    opacity: 0.7;
  }
  
  .custom-select.open .select-icon {
    transform: rotate(180deg);
  }
  
  .dropdown-menu {
    position: fixed;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    z-index: 10000;
    max-height: 200px;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 4px;
  }
  
  .dropdown-menu::-webkit-scrollbar {
    width: 8px;
  }
  
  .dropdown-menu::-webkit-scrollbar-track {
    background: var(--bar-track);
    border-radius: 4px;
  }
  
  .dropdown-menu::-webkit-scrollbar-thumb {
    background: var(--bar-fill);
    border-radius: 4px;
  }
  
  .dropdown-menu::-webkit-scrollbar-thumb:hover {
    background: var(--btn-bg);
  }
  
  .option-item {
    width: 100%;
    padding: 10px 12px;
    border: none;
    background: transparent;
    color: var(--fg);
    font-size: 12px;
    font-weight: 450;
    text-align: left;
    cursor: url('/cursors/light/hand.cur'), pointer;
    border-radius: 8px;
    transition: all 0.15s;
    position: relative;
    overflow: hidden;
  }
  
  html[data-theme="dark"] .option-item {
    cursor: url('/cursors/dark/hand.cur'), pointer;
  }
  
  .option-item:hover,
  .option-item.hovered {
    background: var(--bar-track);
  }
  
  /* Light theme: colore di selezione attinente al tema */
  [data-theme="light"] .option-item.selected {
    background: #9a8a72;
    color: white;
    font-weight: 500;
  }

  /* SHIMMER COMPLETAMENTE DISABILITATO - NESSUN ::after */
  [data-theme="light"] .option-item.selected::after {
    display: none !important;
    content: none !important;
    animation: none !important;
    background: none !important;
    pointer-events: none !important;
    opacity: 0 !important;
    visibility: hidden !important;
  }
  
  /* Dark theme: colore blu */
  html[data-theme="dark"] .option-item.selected {
    background: #0a84ff;
    color: white;
    font-weight: 500;
  }

  /* SHIMMER COMPLETAMENTE DISABILITATO - NESSUN ::after */
  html[data-theme="dark"] .option-item.selected::after {
    display: none !important;
    content: none !important;
    animation: none !important;
    background: none !important;
    pointer-events: none !important;
    opacity: 0 !important;
    visibility: hidden !important;
  }
  
  .option-item:focus {
    outline: none;
  }
  
  .option-item:focus-visible {
    outline: 2px solid var(--input-focus);
    outline-offset: -2px;
  }
</style>

