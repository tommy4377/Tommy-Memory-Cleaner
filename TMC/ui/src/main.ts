// If using Svelte 4 (check package.json)
import App from './App.svelte'
// Import themes first, then tokens (tokens has base variables, themes override them)
import './theme/tokens.css'
import './theme/common.css'

// For Svelte 4:
const app = new App({
  target: document.getElementById('app')!,
})

// Disable context menu
document.addEventListener('contextmenu', (e) => {
  e.preventDefault()
  return false
})

// Disable dev tools shortcuts only in production
if (import.meta.env.PROD) {
  document.addEventListener('keydown', (e) => {
    if (
      e.key === 'F12' ||
      (e.ctrlKey && e.shiftKey && e.key === 'I') ||
      (e.ctrlKey && e.shiftKey && e.key === 'C') ||
      (e.ctrlKey && e.shiftKey && e.key === 'J') ||
      (e.ctrlKey && e.key === 'u')
    ) {
      e.preventDefault()
      return false
    }
    return true
  })
}

// Rimuove il loading
setTimeout(() => {
  document.getElementById('app')?.classList.add('loaded')
}, 100)

export default app
