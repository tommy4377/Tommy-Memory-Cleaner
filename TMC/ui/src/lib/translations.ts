import { invoke } from '@tauri-apps/api/tauri';
import { dict, lang } from '../i18n/index';
import { get } from 'svelte/store';

/**
 * Send current translations to backend for caching
 * This allows backend to use translated strings for notifications
 */
export async function cacheTranslationsInBackend(): Promise<void> {
  try {
    const currentLang = get(lang);
    const currentDict = get(dict);
    
    // Send all translations to backend
    await invoke('cmd_set_translations', {
      language: currentLang,
      translations: currentDict
    });
    
    console.log(`Cached ${Object.keys(currentDict).length} translations for language: ${currentLang}`);
  } catch (error) {
    console.error('Failed to cache translations in backend:', error);
  }
}

/**
 * Update backend translations when language changes
 */
export async function updateBackendTranslations(): Promise<void> {
  await cacheTranslationsInBackend();
}
