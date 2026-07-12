import { writable } from 'svelte/store'

// Global store tracking which color picker is currently open
export const openColorPicker = writable<string | null>(null)

// Closes every color picker except the specified one
export function closeOtherPickers(currentId: string) {
  openColorPicker.update(openId => {
    if (openId && openId !== currentId) {
      return currentId
    }
    return currentId
  })
}

// Closes a specific color picker if it is the one currently open
export function closePicker(pickerId: string) {
  openColorPicker.update(openId => {
    if (openId === pickerId) {
      return null
    }
    return openId
  })
}
