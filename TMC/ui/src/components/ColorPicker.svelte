<!--
  ColorPicker Component
  
  A professional color picker component with:
  - Floating UI positioning for smart popup placement
  - HSL color space manipulation
  - Hex color input/output
  - Drag gestures for saturation/lightness and hue
  - EyeDropper API for color picking from screen
  - Dynamic border coloring based on theme
  - Smooth animations and transitions
  
  @author Tommy Memory Cleaner
  @version 2.2.0
-->
<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte'
  import { computePosition, flip, shift, offset, autoUpdate } from '@floating-ui/dom'
  import { openColorPicker, closeOtherPickers, closePicker } from '../stores/colorPickerStore'
  
  export let value: string = '#000000'
  export let disabled: boolean = false
  
  const dispatch = createEventDispatcher()
  const pickerId = `color-picker-${Math.random().toString(36).substr(2, 9)}` 
  
  let isOpen = false
  let isPositioned = false
  let hue = 0
  let saturation = 100
  let lightness = 50
  let isDragging = false
  let isEyeDropping = false
  let isUpdatingFromParent = false // Flag to prevent update loops
  
  // Internal HSL values that DON'T trigger reactive statement
  let internalHue = 0
  let internalSaturation = 100
  let internalLightness = 50
  
  let buttonElement: HTMLButtonElement
  let popupElement: HTMLDivElement
  let cleanup: (() => void) | null = null
  
  // DEFINITIVE FIX: Update HSL ONLY when value changes from outside
  // and not during any internal operations
  $: if (!isDragging && !isEyeDropping && !isUpdatingFromParent && value !== previousValue) {
    previousValue = value
    const hsl = hexToHsl(value)
    internalHue = hsl.h
    internalSaturation = hsl.s
    internalLightness = hsl.l
    
    // Also update visual variables
    hue = hsl.h
    saturation = hsl.s
    lightness = hsl.l
  }
  
  let previousValue = value
  let lastDispatchedValue = value // Track last dispatched value
  
  const supportsEyeDropper = typeof window !== 'undefined' && 'EyeDropper' in window
  
  /**
   * Convert HEX color to HSL
   * @param hex - Hex color string (e.g., "#FF0000")
   * @returns HSL values object
   */
  function hexToHsl(hex: string) {
    const r = parseInt(hex.slice(1, 3), 16) / 255
    const g = parseInt(hex.slice(3, 5), 16) / 255
    const b = parseInt(hex.slice(5, 7), 16) / 255
    
    const max = Math.max(r, g, b)
    const min = Math.min(r, g, b)
    let h = 0
    let s = 0
    const l = (max + min) / 2
    
    if (max !== min) {
      const d = max - min
      s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
      
      switch (max) {
        case r: h = ((g - b) / d + (g < b ? 6 : 0)) / 6; break
        case g: h = ((b - r) / d + 2) / 6; break
        case b: h = ((r - g) / d + 4) / 6; break
      }
    }
    
    return {
      h: Math.round(h * 360),
      s: Math.round(s * 100),
      l: Math.round(l * 100)
    }
  }
  
  /**
   * Convert HSL color to HEX
   * @param h - Hue (0-360)
   * @param s - Saturation (0-100)
   * @param l - Lightness (0-100)
   * @returns Hex color string
   */
  function hslToHex(h: number, s: number, l: number) {
    s /= 100
    l /= 100
    
    const c = (1 - Math.abs(2 * l - 1)) * s
    const x = c * (1 - Math.abs((h / 60) % 2 - 1))
    const m = l - c / 2
    
    let r = 0, g = 0, b = 0
    
    if (0 <= h && h < 60) {
      r = c; g = x; b = 0
    } else if (60 <= h && h < 120) {
      r = x; g = c; b = 0
    } else if (120 <= h && h < 180) {
      r = 0; g = c; b = x
    } else if (180 <= h && h < 240) {
      r = 0; g = x; b = c
    } else if (240 <= h && h < 300) {
      r = x; g = 0; b = c
    } else if (300 <= h && h < 360) {
      r = c; g = 0; b = x
    }
    
    r = Math.round((r + m) * 255)
    g = Math.round((g + m) * 255)
    b = Math.round((b + m) * 255)
    
    return '#' + [r, g, b].map(x => {
      const hex = x.toString(16)
      return hex.length === 1 ? '0' + hex : hex
    }).join('')
  }
  
  /**
   * Update popup position using Floating UI
   */
  async function updatePosition() {
    if (!buttonElement || !popupElement) return
    
    const { x, y } = await computePosition(buttonElement, popupElement, {
      placement: 'top',
      middleware: [
        offset(8), // 8px offset from button
        flip({
          fallbackPlacements: ['bottom', 'left', 'right'],
        }),
        shift({
          padding: 8, // 8px padding from viewport edges
        }),
      ],
    })
    
    // Apply calculated position
    Object.assign(popupElement.style, {
      left: `${x}px`,
      top: `${y}px`,
    })
    
    // Show popup after first position calculation
    if (!isPositioned) {
      isPositioned = true
    }
  }
  
  /**
   * Open the color picker popup
   */
  function openPicker() {
    if (disabled) return
    
    closeOtherPickers(pickerId)
    
    isOpen = true
    isPositioned = false
    
    // Initialize internal values when opening
    const hsl = hexToHsl(value)
    internalHue = hsl.h
    internalSaturation = hsl.s
    internalLightness = hsl.l
    hue = hsl.h
    saturation = hsl.s
    lightness = hsl.l
    
    requestAnimationFrame(() => {
      if (buttonElement && popupElement) {
        updatePosition().then(() => {
          cleanup = autoUpdate(buttonElement, popupElement, updatePosition)
        })
      }
    })
    
    openColorPicker.set(pickerId)
  }
  
  /**
   * Close the color picker popup
   */
  function closePickerLocal() {
    isOpen = false
    isPositioned = false
    // Clean up autoUpdate
    if (cleanup) {
      cleanup()
      cleanup = null
    }
    closePicker(pickerId)
  }
  
  /**
   * Pause autoUpdate during drag to prevent position recalculation
   */
  function pauseAutoUpdate() {
    if (cleanup) {
      cleanup()
      cleanup = null
    }
  }
  
  /**
   * Resume autoUpdate after drag
   */
  function resumeAutoUpdate() {
    if (buttonElement && popupElement && !cleanup) {
      cleanup = autoUpdate(buttonElement, popupElement, updatePosition)
    }
  }
  
  /**
   * Open EyeDropper to pick color from screen
   */
  async function openEyeDropper() {
    if (!supportsEyeDropper) {
      return
    }
    
    try {
      isEyeDropping = true
      // @ts-ignore - EyeDropper not yet in all types
      const eyeDropper = new EyeDropper()
      const result = await eyeDropper.open()
      
      // Only update and dispatch if value actually changed
      if (result.sRGBHex !== value && result.sRGBHex !== lastDispatchedValue) {
        // Set flag to prevent reactive update loop
        isUpdatingFromParent = true
        value = result.sRGBHex
        const hsl = hexToHsl(result.sRGBHex)
        internalHue = hsl.h
        internalSaturation = hsl.s
        internalLightness = hsl.l
        hue = hsl.h
        saturation = hsl.s
        lightness = hsl.l
        lastDispatchedValue = result.sRGBHex
        
        dispatch('input', { value: result.sRGBHex })
        isUpdatingFromParent = false
      }
    } catch (error) {
      // User cancelled or error occurred
    } finally {
      isEyeDropping = false
    }
  }
  
  /**
   * Start dragging saturation/lightness selector
   */
  function startDragSatLight(e: MouseEvent) {
    e.preventDefault() // Prevent default behavior
    isDragging = true
    pauseAutoUpdate() // Prevent position updates during drag
    
    const satLightElement = e.currentTarget as HTMLElement
    updateFromSatLightWithElement(e, satLightElement)
    
    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging) return
      e.preventDefault()
      updateFromSatLightWithElement(e, satLightElement)
    }
    
    const handleMouseUp = () => {
      isDragging = false
      // IMPORTANT: Sync internal values with visual ones
      internalSaturation = saturation
      internalLightness = lightness
      resumeAutoUpdate() // Resume position updates after drag
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }
    
    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
  }
  
  /**
   * Update color from saturation/lightness selection
   */
  function updateFromSatLightWithElement(e: MouseEvent, element: HTMLElement) {
    const rect = element.getBoundingClientRect()
    const x = Math.max(0, Math.min(e.clientX - rect.left, rect.width))
    const y = Math.max(0, Math.min(e.clientY - rect.top, rect.height))
    
    // Update ONLY visual variables during drag
    saturation = Math.round((x / rect.width) * 100)
    lightness = Math.round((1 - y / rect.height) * 100)
    
    // Use internalHue (not hue) to calculate color
    const newColor = hslToHex(internalHue, saturation, lightness)
    
    // Only update and dispatch if value actually changed
    if (newColor !== value && newColor !== lastDispatchedValue) {
      // Set flag to prevent reactive update loop
      isUpdatingFromParent = true
      value = newColor
      lastDispatchedValue = newColor
      dispatch('input', { value: newColor })
      isUpdatingFromParent = false
    } else {
    }
  }
  
  /**
   * Start dragging hue selector
   */
  function startDragHue(e: MouseEvent) {
    e.preventDefault() // Prevent default behavior
    isDragging = true
    pauseAutoUpdate() // Prevent position updates during drag
    
    const hueElement = e.currentTarget as HTMLElement
    updateFromHueWithElement(e, hueElement)
    
    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging) return
      e.preventDefault()
      updateFromHueWithElement(e, hueElement)
    }
    
    const handleMouseUp = () => {
      isDragging = false
      // IMPORTANT: Sync internal values with visual ones
      internalHue = hue
      resumeAutoUpdate() // Resume position updates after drag
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }
    
    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
  }
  
  /**
   * Update color from hue selection
   */
  function updateFromHueWithElement(e: MouseEvent, element: HTMLElement) {
    const rect = element.getBoundingClientRect()
    const x = Math.max(0, Math.min(e.clientX - rect.left, rect.width))
    
    // Update ONLY hue visual during drag
    hue = Math.round((x / rect.width) * 360)
    
    // Use internalSaturation and internalLightness to calculate color
    const newColor = hslToHex(hue, internalSaturation, internalLightness)
    
    // Only update and dispatch if value actually changed
    if (newColor !== value && newColor !== lastDispatchedValue) {
      // Set flag to prevent reactive update loop
      isUpdatingFromParent = true
      value = newColor
      lastDispatchedValue = newColor
      dispatch('input', { value: newColor })
      isUpdatingFromParent = false
    }
  }
  
  /**
   * Handle hex color input
   */
  function handleHexInput(e: Event) {
    const target = e.target as HTMLInputElement
    const hex = target.value
    
    if (/^#[0-9A-F]{6}$/i.test(hex)) {
      // Only update and dispatch if value actually changed
      if (hex !== value && hex !== lastDispatchedValue) {
        // Set flag to prevent reactive update loop
        isUpdatingFromParent = true
        value = hex
        const hsl = hexToHsl(hex)
        internalHue = hsl.h
        internalSaturation = hsl.s
        internalLightness = hsl.l
        hue = hsl.h
        saturation = hsl.s
        lightness = hsl.l
        lastDispatchedValue = hex
        dispatch('input', { value: hex })
        isUpdatingFromParent = false
      }
    }
  }
  
  // Handle closing when another picker opens
  onMount(() => {
    const unsubscribe = openColorPicker.subscribe(openId => {
      if (openId !== pickerId && isOpen) {
        closePickerLocal()
      }
    })
    
    return () => {
      unsubscribe()
      if (cleanup) cleanup()
    }
  })
  
  // Close on outside click
  function handleClickOutside(e: MouseEvent) {
    if (!e.target || !(e.target as Element).closest('.color-picker-wrapper')) {
      closePickerLocal()
    }
  }
  
  $: if (isOpen) {
    document.addEventListener('click', handleClickOutside)
  } else {
    document.removeEventListener('click', handleClickOutside)
  }
</script>

<div class="color-picker-wrapper" id={pickerId}>
  <button 
    bind:this={buttonElement}
    class="color-input" 
    class:disabled
    style="background-color: {value}; --current-color: {value};"
    on:click={openPicker}
    disabled={disabled}
  >
  </button>
  
  {#if isOpen}
    <div 
      bind:this={popupElement}
      class="color-picker-popup"
      class:positioned={isPositioned}
    >
      <div class="picker-header">
        <div class="hex-input-wrapper">
          <input 
            type="text" 
            class="hex-input" 
            bind:value={value}
            on:input={handleHexInput}
            placeholder="#000000"
          />
          
          <!-- Eyedropper button -->
          {#if supportsEyeDropper}
            <button 
              class="eyedropper-btn"
              on:click={openEyeDropper}
              title="Pick color from screen"
              type="button"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="m2 22 1-1h3l9-9"/>
                <path d="M3 21v-3l9-9"/>
                <path d="m15 6 3.4-3.4a2.1 2.1 0 1 1 3 3L18 9l.4.4a2.1 2.1 0 1 1-3 3l-3.8-3.8a2.1 2.1 0 1 1 3-3l.4.4Z"/>
              </svg>
            </button>
          {/if}
        </div>
      </div>
      
      <div class="sat-light-wrapper" on:mousedown={startDragSatLight}>
        <div 
          class="sat-light-gradient" 
          style="background: linear-gradient(to right, hsl(0, 0%, 50%), hsl({hue}, 100%, 50%))"
        ></div>
        <div class="sat-light-overlay">
          <div class="white-gradient"></div>
          <div class="black-gradient"></div>
        </div>
        <div 
          class="sat-light-pointer" 
          style="left: {saturation}%; top: {100 - lightness}%"
        ></div>
      </div>
      
      <div class="hue-slider" on:mousedown={startDragHue}>
        <div class="hue-gradient"></div>
        <div 
          class="hue-pointer" 
          style="left: {(hue / 360) * 100}%"
        ></div>
      </div>
    </div>
  {/if}
</div>

<style>
  .color-picker-wrapper {
    position: relative;
    display: inline-block;
  }
  
  .color-input {
    width: 45px;
    height: 30px;
    border-radius: 10px;
    cursor: url('/cursors/light/hand.cur'), pointer !important;
    position: relative;
    transition: all 0.2s;
    /* Use a variable to manage border color */
    border: 2px solid var(--border-color);
  }

  :global(html[data-theme='dark']) .color-input {
    cursor: url('/cursors/dark/hand.cur'), pointer !important;
  }

  /* Light mode - darker border using color-mix */
  :global(html[data-theme="light"]) .color-input {
    --border-color: color-mix(in srgb, var(--current-color), black 20%);
  }
  
  @media (prefers-color-scheme: light) {
    :global(html:not([data-theme="dark"])) .color-input {
      --border-color: color-mix(in srgb, var(--current-color), black 20%);
    }
  }

  /* Dark mode - lighter border using color-mix */
  :global(html[data-theme="dark"]) .color-input {
    --border-color: color-mix(in srgb, var(--current-color), white 30%);
  }

  @media (prefers-color-scheme: dark) {
    :global(html:not([data-theme="light"])) .color-input {
      --border-color: color-mix(in srgb, var(--current-color), white 30%);
    }
  }

  .color-input:hover:not(.disabled) {
    transform: translateY(-1px);
    /* On hover, border becomes pure color */
    --border-color: var(--current-color);
  }

  .color-input.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  .color-picker-popup {
    position: fixed;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 12px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    z-index: 1000;
    min-width: 240px;
    
    /* Initially hidden with opacity */
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.15s ease-out;
    
    /* Performance optimization */
    will-change: transform;
  }
  
  /* Show popup when positioned */
  .color-picker-popup.positioned {
    opacity: 1;
    pointer-events: auto;
  }
  
  .picker-header {
    margin-bottom: 12px;
  }
  
  /* Wrapper for input + eyedropper */
  .hex-input-wrapper {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  
  .hex-input {
    flex: 1;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--fg);
    font-size: 12px;
    text-align: center;
    font-family: inherit;
  }
  
  .hex-input:focus {
    outline: none;
    border-color: var(--btn-bg);
  }
  
  /* Eyedropper button styling */
  .eyedropper-btn {
    width: 36px;
    height: 36px;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--fg);
    cursor: url('/cursors/light/hand.cur'), pointer !important;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
    flex-shrink: 0;
  }

  :global(html[data-theme='dark']) .eyedropper-btn {
    cursor: url('/cursors/dark/hand.cur'), pointer !important;
  }
  
  .eyedropper-btn:hover {
    background: var(--btn-bg);
    color: var(--btn-fg);
    border-color: var(--btn-bg);
    transform: translateY(-1px);
  }
  
  .eyedropper-btn:active {
    transform: translateY(0);
  }
  
  .sat-light-wrapper {
    position: relative;
    width: 100%;
    height: 120px;
    border-radius: 8px;
    margin-bottom: 12px;
    cursor: url('/cursors/light/bussola.cur'), crosshair !important;
    /* Prevent text selection during drag */
    user-select: none;
    -webkit-user-select: none;
    /* Touch action for mobile */
    touch-action: none;
  }

  :global(html[data-theme='dark']) .sat-light-wrapper {
    cursor: url('/cursors/dark/bussola.cur'), crosshair !important;
  }
  
  .sat-light-gradient {
    position: absolute;
    inset: 0;
    border-radius: 8px;
  }
  
  .sat-light-overlay {
    position: absolute;
    inset: 0;
    border-radius: 8px;
  }
  
  .white-gradient {
    position: absolute;
    inset: 0;
    background: linear-gradient(to right, white, transparent);
    border-radius: 8px;
  }
  
  .black-gradient {
    position: absolute;
    inset: 0;
    background: linear-gradient(to top, black, transparent);
    border-radius: 8px;
  }
  
  .sat-light-pointer {
    position: absolute;
    width: 16px;
    height: 16px;
    border: 3px solid white;
    border-radius: 50%;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    transform: translate(-50%, -50%);
    pointer-events: none;
    /* Performance optimization for drag */
    will-change: left, top;
  }
  
  .hue-slider {
    position: relative;
    width: 100%;
    height: 20px;
    border-radius: 10px;
    cursor: url('/cursors/light/sizewe.cur), ew-resize !important;
    /* Prevent text selection during drag */
    user-select: none;
    -webkit-user-select: none;
    /* Touch action for mobile */
    touch-action: none;
  }

  :global(html[data-theme='dark']) .hue-slider {
    cursor: url('/cursors/dark/sizewe.cur), ew-resize !important;
  }
  
  .hue-gradient {
    position: absolute;
    inset: 0;
    background: linear-gradient(to right, 
      hsl(0, 100%, 50%), 
      hsl(60, 100%, 50%), 
      hsl(120, 100%, 50%), 
      hsl(180, 100%, 50%), 
      hsl(240, 100%, 50%), 
      hsl(300, 100%, 50%), 
      hsl(360, 100%, 50%)
    );
    border-radius: 10px;
  }
  
  .hue-pointer {
    position: absolute;
    top: 50%;
    width: 20px;
    height: 20px;
    background: white;
    border: 2px solid var(--border);
    border-radius: 50%;
    transform: translate(-50%, -50%);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    pointer-events: none;
    /* Performance optimization for drag */
    will-change: left;
  }
</style>
