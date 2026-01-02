import { writable } from 'svelte/store'

// Store globale per gestire quale color picker è aperto
export const openColorPicker = writable<string | null>(null)

// Funzione per chiudere tutti i color picker tranne uno specifico
export function closeOtherPickers(currentId: string) {
  openColorPicker.update(openId => {
    if (openId && openId !== currentId) {
      return currentId
    }
    return currentId
  })
}

// Funzione per chiudere un color picker specifico
export function closePicker(pickerId: string) {
  openColorPicker.update(openId => {
    if (openId === pickerId) {
      return null
    }
    return openId
  })
}
