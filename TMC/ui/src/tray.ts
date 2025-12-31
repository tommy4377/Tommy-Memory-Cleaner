import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { areasForProfile, areasToString } from './lib/profiles';

const win = getCurrentWebviewWindow();

// Esponi win globalmente per il codice inline Rust
(window as any).win = win;

const translations = {
    it: { dashboard: 'Dashboard', optimize: 'Ottimizza', exit: 'Esci' },
    en: { dashboard: 'Dashboard', optimize: 'Optimize', exit: 'Exit' },
    es: { dashboard: 'Panel', optimize: 'Optimizar', exit: 'Salir' },
    fr: { dashboard: 'Tableau', optimize: 'Optimiser', exit: 'Quitter' },
    pt: { dashboard: 'Painel', optimize: 'Otimizar', exit: 'Sair' },
    de: { dashboard: 'Dashboard', optimize: 'Optimieren', exit: 'Beenden' },
    ar: { dashboard: 'لوحة التحكم', optimize: 'تحسين', exit: 'خروج' },
    ja: { dashboard: 'ダッシュボード', optimize: '最適化', exit: '終了' },
    zh: { dashboard: '仪表板', optimize: '优化', exit: '退出' }
};

async function loadConfig() {
    try {
        const config = await invoke('cmd_get_config') as any;
        document.body.setAttribute('data-theme', config.theme || 'dark');
        
        // Applica mainColor ai menu items (non danger)
        const mainColor = config.theme === 'light' 
            ? (config.main_color_hex_light || '#9a8a72')
            : (config.main_color_hex_dark || '#0a84ff');
        document.documentElement.style.setProperty('--main-color', mainColor);
        
        const t = translations[config.language] || translations.it;
        document.querySelectorAll('[data-i18n]').forEach(el => {
            const key = el.getAttribute('data-i18n');
            if (key && t[key as keyof typeof t]) el.textContent = t[key as keyof typeof t];
        });
    } catch (err) {
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
            } catch (err) {
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
    } catch (err) {
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
    } catch (err) {
        // Ignora errori silenziosamente
    }
}, 500);

// Carica configurazione all'avvio
loadConfig();

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

