import { invoke } from '@tauri-apps/api/core';
import { dict, lang } from '../i18n/index';
import { get } from 'svelte/store';

/**
 * Send current translations to backend for caching
 * This allows backend to use translated strings for notifications
 */
export async function cacheTranslationsInBackend(): Promise<void> {
  const maxRetries = 5;
  const retryDelay = 500; // ms
  
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const currentLang = get(lang);
      const currentDict = get(dict);
      
      // Send all translations to backend
      await invoke('cmd_set_translations', {
        language: currentLang,
        translations: currentDict
      });
      
      console.log(`Cached ${Object.keys(currentDict).length} translations for language: ${currentLang}`);
      return; // Success, exit the function
    } catch (error: any) {
      console.error(`Attempt ${attempt} failed to cache translations in backend:`, error);
      
      if (attempt === maxRetries) {
        console.error('Failed to cache translations after maximum retries');
        return;
      }
      
      // If error contains "state not managed", wait and retry
      if (error.message?.includes('state not managed')) {
        console.log(`Retrying in ${retryDelay}ms...`);
        await new Promise(resolve => setTimeout(resolve, retryDelay));
      } else {
        // For other errors, don't retry
        console.error('Non-retryable error occurred:', error);
        return;
      }
    }
  }
}

/**
 * Update backend translations when language changes
 */
export async function updateBackendTranslations(): Promise<void> {
  await cacheTranslationsInBackend();
}
