import type { Profile, Areas } from './types';
import { AreasFlag } from './types';

export function areasForProfile(profile: Profile): Areas {
  switch (profile) {
    case 'Normal':
      // Profilo leggero: aree essenziali + registry cache (molto leggero e efficace)
      return AreasFlag.WORKING_SET | AreasFlag.MODIFIED_PAGE_LIST | AreasFlag.REGISTRY_CACHE;
    case 'Balanced':
      // Profilo bilanciato: aree principali + modified file cache + registry cache per efficienza
      return (
        AreasFlag.WORKING_SET |
        AreasFlag.MODIFIED_PAGE_LIST |
        AreasFlag.STANDBY_LIST |
        AreasFlag.SYSTEM_FILE_CACHE |
        AreasFlag.MODIFIED_FILE_CACHE |
        AreasFlag.REGISTRY_CACHE
      );
    case 'Gaming':
      // Tutte le aree per massime prestazioni
      return (
        AreasFlag.WORKING_SET |
        AreasFlag.MODIFIED_PAGE_LIST |
        AreasFlag.STANDBY_LIST |
        AreasFlag.STANDBY_LIST_LOW |
        AreasFlag.SYSTEM_FILE_CACHE |
        AreasFlag.COMBINED_PAGE_LIST |
        AreasFlag.MODIFIED_FILE_CACHE |
        AreasFlag.REGISTRY_CACHE
      );
    default:
      return AreasFlag.WORKING_SET;
  }
}

export function areaNamesForAreas(areas: Areas): string[] {
  const names: string[] = [];
  if (areas & AreasFlag.WORKING_SET) names.push('Working Set');
  if (areas & AreasFlag.MODIFIED_PAGE_LIST) names.push('Modified Pages');
  if (areas & AreasFlag.STANDBY_LIST) names.push('Standby List');
  if (areas & AreasFlag.STANDBY_LIST_LOW) names.push('Low Priority Standby');
  if (areas & AreasFlag.SYSTEM_FILE_CACHE) names.push('System Cache');
  if (areas & AreasFlag.COMBINED_PAGE_LIST) names.push('Combined Pages');
  if (areas & AreasFlag.MODIFIED_FILE_CACHE) names.push('File Cache');
  if (areas & AreasFlag.REGISTRY_CACHE) names.push('Registry Cache');
  return names;
}

// Nuova funzione per convertire il numero in stringa di flag
export function areasToString(areas: Areas): string {
  const flags: string[] = [];
  
  if (areas & AreasFlag.COMBINED_PAGE_LIST) flags.push('COMBINED_PAGE_LIST');
  if (areas & AreasFlag.MODIFIED_FILE_CACHE) flags.push('MODIFIED_FILE_CACHE');
  if (areas & AreasFlag.MODIFIED_PAGE_LIST) flags.push('MODIFIED_PAGE_LIST');
  if (areas & AreasFlag.REGISTRY_CACHE) flags.push('REGISTRY_CACHE');
  if (areas & AreasFlag.STANDBY_LIST) flags.push('STANDBY_LIST');
  if (areas & AreasFlag.STANDBY_LIST_LOW) flags.push('STANDBY_LIST_LOW');
  if (areas & AreasFlag.SYSTEM_FILE_CACHE) flags.push('SYSTEM_FILE_CACHE');
  if (areas & AreasFlag.WORKING_SET) flags.push('WORKING_SET');
  
  return flags.join('|');
}

// Funzione per convertire da stringa a numero (se necessario)
export function stringToAreas(flagString: string): Areas {
  const flags = flagString.split('|');
  let areas = 0;
  
  for (const flag of flags) {
    switch (flag.trim()) {
      case 'COMBINED_PAGE_LIST': areas |= AreasFlag.COMBINED_PAGE_LIST; break;
      case 'MODIFIED_FILE_CACHE': areas |= AreasFlag.MODIFIED_FILE_CACHE; break;
      case 'MODIFIED_PAGE_LIST': areas |= AreasFlag.MODIFIED_PAGE_LIST; break;
      case 'REGISTRY_CACHE': areas |= AreasFlag.REGISTRY_CACHE; break;
      case 'STANDBY_LIST': areas |= AreasFlag.STANDBY_LIST; break;
      case 'STANDBY_LIST_LOW': areas |= AreasFlag.STANDBY_LIST_LOW; break;
      case 'SYSTEM_FILE_CACHE': areas |= AreasFlag.SYSTEM_FILE_CACHE; break;
      case 'WORKING_SET': areas |= AreasFlag.WORKING_SET; break;
    }
  }
  
  return areas;
}