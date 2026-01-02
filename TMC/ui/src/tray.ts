import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { listen } from '@tauri-apps/api/event';
import { areasForProfile, areasToString } from './lib/profiles';
import { dict, setLanguage, lang } from './i18n';
import { get } from 'svelte/store';

const win = getCurrentWebviewWindow();

// Esponi win globalmente per il codice inline Rust
(window as any).win = win;

// Funzione per aggiornare le traduzioni nel DOM usando il sistema i18n
function updateTrayTranslations() {
    // Ottieni il dizionario delle traduzioni
    const translations = get(dict);
    
    console.log('=== TRAY TRANSLATIONS DEBUG ===');
    console.log('Current language:', get(lang));
    console.log('Dict sample:', {
        'Open TMC': translations['Open TMC'],
        'Optimize Memory': translations['Optimize Memory'],
        'Exit': translations['Exit']
    });
    
    // Traduci tutti gli elementi con data-i18n
    document.querySelectorAll('[data-i18n]').forEach(el => {
        const key = el.getAttribute('data-i18n');
        if (key && translations[key]) {
            console.log(`✓ Translating "${key}" -> "${translations[key]}"`);
            el.textContent = translations[key];
        } else if (key && !translations[key]) {
            // Fallback: mostra la chiave se la traduzione non esiste
            console.warn(`✗ Missing translation for "${key}" in language ${get(lang)}`);
            el.textContent = key;
        }
    });
    console.log('=== END DEBUG ===');
}

// Registra i listener eventi una sola volta all'avvio
async function setupEventListeners() {
    // Ascolta eventi di cambio lingua dal backend
    await listen('language-changed', async (event: any) => {
        const newLanguage = event.payload;
        console.log('Language changed in tray:', newLanguage);
        await setLanguage(newLanguage);
        // Aspetta che le traduzioni siano caricate
        await new Promise(resolve => setTimeout(resolve, 50));
        updateTrayTranslations();
    });
    
    // Ascolta eventi di apertura menu per ricaricare la configurazione
    await listen('tray-menu-open', async () => {
        console.log('Tray menu opened, reloading config...');
        // Ricarica solo la configurazione, non i listener
        await reloadTrayConfig();
    });
}

// Funzione separata per ricaricare solo la config (senza registrare listener)
async function reloadTrayConfig() {
    try {
        const config = await invoke('cmd_get_config') as any;
        document.body.setAttribute('data-theme', config.theme || 'dark');
        
        // Applica mainColor ai menu items (non danger)
        const mainColor = config.theme === 'light' 
            ? (config.main_color_hex_light || '#9a8a72')
            : (config.main_color_hex_dark || '#0a84ff');
        document.documentElement.style.setProperty('--main-color', mainColor);
        
        // Imposta la lingua usando il sistema i18n
        await setLanguage(config.language || 'en');
        
        // Aspetta che le traduzioni siano caricate prima di aggiornare
        await new Promise(resolve => setTimeout(resolve, 50));
        
        // Aggiorna subito le traduzioni
        updateTrayTranslations();
    } catch (err: any) {
        console.error('Config reload failed:', err);
    }
}

async function loadConfig() {
    try {
        await reloadTrayConfig();
        
        // Ascolta i cambiamenti futuri del dizionario
        const unsubscribe = dict.subscribe(() => {
            // Aspetta un tick per assicurarsi che il DOM sia aggiornato
            requestAnimationFrame(() => {
                updateTrayTranslations();
            });
        });
    } catch (err: any) {
        console.error('Config load failed:', err);
    }
}

async function handleAction(action: string) {
    if (!action) return;
    
    try {
        // Chiudi il menu prima di eseguire l'azione
        await win.hide();
        
        // Piccolo delay per assicurarsi che il menu sia chiuso
        await new Promise(resolve => setTimeout(resolve, 50));
        
        // Esegui l'azione
        if (action === 'open') {
            await invoke('cmd_show_or_create_window');
        } else if (action === 'optimize') {
            // FIX: Leggi il profilo corrente dalla configurazione e usa le aree corrette
            try {
                const config = await invoke('cmd_get_config') as any;
                const profile = config.profile || 'Balanced';
                
                // Usa la funzione areasForProfile per ottenere le aree corrette
                const areas = areasForProfile(profile);
                const areasString = areasToString(areas);
                
                await invoke('cmd_optimize_async', { 
                    reason: 'Manual', 
                    areas: areasString 
                });
            } catch (err: any) {
                console.error('Failed to get config for optimization, using default balanced profile:', err);
                // Fallback a balanced se non riesce a leggere la config
                const defaultAreas = areasForProfile('Balanced');
                const defaultAreasString = areasToString(defaultAreas);
                await invoke('cmd_optimize_async', { 
                    reason: 'Manual', 
                    areas: defaultAreasString 
                });
            }
        } else if (action === 'exit') {
            await invoke('cmd_exit');
        }
    } catch (err: any) {
        console.error('Action failed:', err);
    } finally {
        // Assicurati che il menu sia sempre chiuso dopo un'azione
        win.hide().catch(() => {});
    }
}

// Setup semplice e diretto degli event listener
function setupMenuItems() {
    const items = document.querySelectorAll('.menu-item');
    items.forEach((item) => {
        const action = item.getAttribute('data-action');
        if (action) {
            (item as HTMLElement).onclick = (e) => {
                e.preventDefault();
                e.stopPropagation();
                handleAction(action);
            };
        }
    });
}

// Setup iniziale
setupMenuItems();

// Flag per prevenire chiusura immediata durante setup
let isInitializing = true;
setTimeout(() => { isInitializing = false; }, 500);

// Funzione per chiudere il menu
function closeMenu() {
    if (isInitializing) return;
    
    document.body.classList.remove('menu-open');
    
    // Chiudi la finestra con retry
    win.hide().catch((err) => {
        console.warn('Failed to hide tray menu window:', err);
        // Retry dopo un breve delay
        setTimeout(() => {
            win.hide().catch(() => {});
        }, 100);
    });
}

// Esponi closeMenu globalmente
(window as any).closeMenu = closeMenu;

// Funzione per mostrare il menu
function showMenu() {
    document.body.classList.add('menu-open');
}

// Esponi showMenu globalmente per permettere chiamate dal backend
(window as any).showMenu = showMenu;

// Mostra il menu quando la finestra diventa visibile
if (!document.hidden) {
    // Piccolo delay per assicurarsi che il DOM sia pronto
    setTimeout(() => {
        showMenu();
    }, 50);
}

document.addEventListener('visibilitychange', () => {
    if (!document.hidden) {
        showMenu();
    } else {
        // Chiudi solo se la finestra diventa nascosta (es. alt-tab)
        closeMenu();
    }
});

// ⭐ Chiusura automatica quando la finestra perde il focus (click fuori)
// Usa l'API Tauri invece di window.addEventListener per maggiore affidabilità
win.onFocusChanged((event: any) => {
    const isFocused = event.payload;
    if (!isFocused && document.body.classList.contains('menu-open')) {
        // Piccolo delay per permettere ai click sui menu items di funzionare
        setTimeout(() => {
            closeMenu();
        }, 100);
    }
});

// Fallback per click su overlay (se presente)
document.querySelector('.click-overlay')?.addEventListener('click', () => {
    if (document.body.classList.contains('menu-open')) {
        win.hide();
    }
});

// Gestione click fuori dal menu container - unico modo per chiudere il menu
document.addEventListener('click', (e) => {
    const menuContainer = document.querySelector('.menu-container');
    const clickOverlay = document.getElementById('click-overlay');
    
    // Se il click è sull'overlay (fuori dal menu), chiudi il menu
    if (clickOverlay && e.target === clickOverlay) {
        if (document.body.classList.contains('menu-open')) {
            closeMenu();
        }
        return;
    }
    
    // Se il click è fuori dal menu container, chiudilo
    if (menuContainer && !menuContainer.contains(e.target as Node)) {
        if (document.body.classList.contains('menu-open')) {
            closeMenu();
        }
    }
});

// Chiudi quando si preme ESC
document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
        closeMenu();
    }
});

// Esponi loadConfig globalmente per permettere chiamate esterne
(window as any).loadConfig = loadConfig;

// Polling periodico per controllare i cambiamenti di tema dalla configurazione
setInterval(async () => {
    try {
        const config = await invoke('cmd_get_config') as any;
        const newTheme = config.theme || 'dark';
        const currentTheme = document.body.getAttribute('data-theme');
        if (currentTheme !== newTheme) {
            loadConfig();
        }
    } catch (err: any) {
        // Ignora errori silenziosamente
    }
}, 500);

// Carica configurazione all'avvio
async function initializeTray() {
    // Prima registra i listener eventi
    await setupEventListeners();
    // Poi carica la configurazione
    await loadConfig();
}

// Inizializza all'avvio
initializeTray();

// Posiziona il menu container in base alla posizione della finestra
// La finestra è fullscreen, quindi dobbiamo posizionare il container
window.addEventListener('load', () => {
    // Ottieni la posizione della finestra (che è già posizionata sopra la tray icon)
    // Il menu container deve essere posizionato in alto a sinistra della finestra
    const menuContainer = document.querySelector('.menu-container') as HTMLElement;
    if (menuContainer) {
        menuContainer.style.position = 'absolute';
        menuContainer.style.top = '0';
        menuContainer.style.left = '0';
    }
});

