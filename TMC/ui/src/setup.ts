import './theme/common.css'
import './theme/tokens.css'
import Setup from './components/Setup.svelte'
import { invoke } from '@tauri-apps/api/core'

const app = document.getElementById('app')
if (app) {
  new Setup({ target: app })
}
