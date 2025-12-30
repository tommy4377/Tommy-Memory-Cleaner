<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  
  export let value: string = '#007aff';
  export let label: string = '';
  
  const dispatch = createEventDispatcher<{ change: string }>();
  
  let showPicker = false;
  let hue = 0;
  let saturation = 100;
  let lightness = 50;
  let alpha = 1;
  
  function hexToHsl(hex: string): [number, number, number] {
    const r = parseInt(hex.slice(1, 3), 16) / 255;
    const g = parseInt(hex.slice(3, 5), 16) / 255;
    const b = parseInt(hex.slice(5, 7), 16) / 255;
    
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    let h = 0, s = 0, l = (max + min) / 2;
    
    if (max !== min) {
      const d = max - min;
      s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
      switch (max) {
        case r: h = ((g - b) / d + (g < b ? 6 : 0)) / 6; break;
        case g: h = ((b - r) / d + 2) / 6; break;
        case b: h = ((r - g) / d + 4) / 6; break;
      }
    }
    
    return [Math.round(h * 360), Math.round(s * 100), Math.round(l * 100)];
  }
  
  function hslToHex(h: number, s: number, l: number): string {
    l /= 100;
    const a = s * Math.min(l, 1 - l) / 100;
    const f = (n: number) => {
      const k = (n + h / 30) % 12;
      const color = l - a * Math.max(Math.min(k - 3, 9 - k, 1), -1);
      return Math.round(255 * color).toString(16).padStart(2, '0');
    };
    return `#${f(0)}${f(8)}${f(4)}`;
  }
  
  function updateFromHex(hex: string) {
    const [h, s, l] = hexToHsl(hex);
    hue = h;
    saturation = s;
    lightness = l;
  }
  
  function updateColor() {
    const newHex = hslToHex(hue, saturation, lightness);
    value = newHex;
    dispatch('change', newHex);
  }
  
  $: if (value && !showPicker) {
    updateFromHex(value);
  }
  
  $: if (showPicker) {
    updateColor();
  }
  
  function togglePicker() {
    showPicker = !showPicker;
    if (showPicker) {
      updateFromHex(value);
    }
  }
  
  function closePicker(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target) return;
    // Chiudi solo se il click è fuori dal color-picker-container
    if (!target.closest('.color-picker-container')) {
      showPicker = false;
    }
  }
</script>

<svelte:window on:click={closePicker} />

<div class="color-picker-wrapper">
  {#if label}
    <label class="color-label" for="color-picker-button">{label}</label>
  {/if}
  <div class="color-picker-container">
    <button 
      id="color-picker-button"
      type="button"
      class="color-button" 
      style="background-color: {value};"
      on:click|stopPropagation={togglePicker}
      aria-label="Select color"
    >
      <span class="color-preview"></span>
    </button>
    
    {#if showPicker}
      <div class="picker-popup" role="dialog" aria-label="Color picker" aria-modal="true">
        <div class="picker-header">
          <span>Select Color</span>
          <button type="button" class="close-btn" on:click={() => showPicker = false}>×</button>
        </div>
        
        <div class="picker-body">
          <div class="hue-slider-container">
            <label for="hue-slider">Hue</label>
            <input 
              id="hue-slider"
              type="range" 
              min="0" 
              max="360" 
              bind:value={hue}
              on:input={updateColor}
              class="hue-slider"
            />
          </div>
          
          <div class="slider-container">
            <label for="saturation-slider">Saturation</label>
            <input 
              id="saturation-slider"
              type="range" 
              min="0" 
              max="100" 
              bind:value={saturation}
              on:input={updateColor}
            />
          </div>
          
          <div class="slider-container">
            <label for="lightness-slider">Lightness</label>
            <input 
              id="lightness-slider"
              type="range" 
              min="0" 
              max="100" 
              bind:value={lightness}
              on:input={updateColor}
            />
          </div>
          
          <div class="color-preview-large" style="background: hsl({hue}, {saturation}%, {lightness}%);" role="img" aria-label="Color preview"></div>
          
          <div class="hex-input-container">
            <label for="hex-input">Hex</label>
            <input 
              id="hex-input"
              type="text" 
              value={value}
              on:input={(e) => {
                const target = e.target;
                if (target && target instanceof HTMLInputElement) {
                  const newValue = target.value;
                  if (/^#[0-9A-Fa-f]{6}$/.test(newValue)) {
                    value = newValue;
                    updateFromHex(newValue);
                    dispatch('change', newValue);
                  }
                }
              }}
              class="hex-input"
            />
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .color-picker-wrapper {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  
  .color-label {
    font-size: 12px;
    font-weight: 450;
  }
  
  .color-picker-container {
    position: relative;
  }
  
  .color-button {
    width: 50px;
    height: 32px;
    border: 1px solid var(--border);
    border-radius: 5px;
    cursor: pointer;
    padding: 0;
    position: relative;
    overflow: hidden;
  }
  
  .color-preview {
    display: block;
    width: 100%;
    height: 100%;
  }
  
  .picker-popup {
    position: absolute;
    top: 100%;
    left: 0;
    margin-top: 8px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    z-index: 1000;
    min-width: 250px;
    padding: 12px;
  }
  
  .picker-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
    font-size: 13px;
    font-weight: 500;
  }
  
  .close-btn {
    background: none;
    border: none;
    font-size: 20px;
    cursor: pointer;
    color: var(--fg);
    padding: 0;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
  }
  
  .close-btn:hover {
    background: var(--input-bg);
  }
  
  .picker-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  
  .hue-slider-container,
  .slider-container {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  
  .hue-slider-container label,
  .slider-container label {
    font-size: 11px;
    opacity: 0.8;
  }
  
  .color-preview-large {
    width: 100%;
    height: 60px;
    border-radius: 6px;
    border: 1px solid var(--border);
  }
  
  .hex-input-container {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  
  .hex-input-container label {
    font-size: 11px;
    opacity: 0.8;
  }
  
  .hex-input {
    width: 100%;
    padding: 6px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--input-bg);
    color: var(--fg);
    font-size: 12px;
    font-family: monospace;
  }
</style>

