import { writable, derived } from 'svelte/store';
import en from './en.json';
import it from './it.json';
import es from './es.json';
import fr from './fr.json';
import pt from './pt.json';
import de from './de.json';
import ar from './ar.json';
import ja from './ja.json';
import zh from './zh.json';

export type Language = 'en' | 'it' | 'es' | 'fr' | 'pt' | 'de' | 'ar' | 'ja' | 'zh';

export const lang = writable<Language>('en');
export const dict = writable<Record<string, string>>(en as any);

const translations: Record<Language, any> = {
  en: en,
  it: it,
  es: es,
  fr: fr,
  pt: pt,
  de: de,
  ar: ar,
  ja: ja,
  zh: zh
};

export function setLanguage(code: Language) {
  const selected = translations[code] || translations.en;
  lang.set(code);
  dict.set(selected);
  
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('lang', code);
    // Set RTL for Arabic
    if (code === 'ar') {
      document.documentElement.setAttribute('dir', 'rtl');
    } else {
      document.documentElement.setAttribute('dir', 'ltr');
    }
  }
}

export const t = derived(dict, d => (k: string) => d[k] ?? k);