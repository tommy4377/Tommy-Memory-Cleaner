import './theme/common.css'
import './theme/tokens.css'
import Setup from './components/Setup.svelte'
import { invoke } from '@tauri-apps/api/core'

// Disable the default WebView2 context menu in production (same as main.ts)
if (import.meta.env.PROD) {
  document.addEventListener('contextmenu', (e) => e.preventDefault())
}

const app = document.getElementById('app')
if (app) {
  new Setup({ target: app })
}
