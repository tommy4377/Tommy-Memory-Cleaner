// Se usi Svelte 4 (controlla package.json)
import App from './App.svelte';
// Importa prima i temi, poi tokens (tokens ha le variabili base, i temi le sovrascrivono)
import './theme/tokens.css';
import './theme/common.css';
import './theme/light.css';

// Per Svelte 4:
const app = new App({
  target: document.getElementById('app')!
});

// Disabilita menu contestuale
document.addEventListener('contextmenu', (e) => {
  e.preventDefault();
  return false;
});

// Disabilita dev tools shortcuts solo in produzione
if (import.meta.env.PROD) {
  document.addEventListener('keydown', (e) => {
    if (e.key === 'F12' || 
        (e.ctrlKey && e.shiftKey && e.key === 'I') ||
        (e.ctrlKey && e.shiftKey && e.key === 'C') ||
        (e.ctrlKey && e.shiftKey && e.key === 'J') ||
        (e.ctrlKey && e.key === 'u')) {
      e.preventDefault();
      return false;
    }
    return true;
  });
}

// Rimuove il loading
setTimeout(() => {
  document.getElementById('app')?.classList.add('loaded');
}, 100);

export default app;