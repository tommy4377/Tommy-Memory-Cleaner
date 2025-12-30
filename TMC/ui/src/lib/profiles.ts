import type { Profile, Areas } from './types';
import { AreasFlag } from './types';

export function areasForProfile(profile: Profile): Areas {
  switch (profile) {
    case 'Normal':
      // Light profile: Essential and safest areas only
      // - WORKING_SET: Core optimization, high impact, safe (critical processes protected)
      // - MODIFIED_PAGE_LIST: Very safe, clears pages waiting for disk write
      // - REGISTRY_CACHE: Lightweight, very safe, cache rebuilds automatically
      // Excludes aggressive areas for minimal system impact
      return AreasFlag.WORKING_SET | AreasFlag.MODIFIED_PAGE_LIST | AreasFlag.REGISTRY_CACHE;
    case 'Balanced':
      // Balanced profile: Good balance between memory freed and system performance
      // Includes all Normal areas plus:
      // - STANDBY_LIST: High memory freed, safe, low-medium performance impact
      // - SYSTEM_FILE_CACHE: High memory freed, safe with auto-rebuild
      // - MODIFIED_FILE_CACHE: More aggressive cache flush, high impact
      // Note: Backend will validate availability of MODIFIED_FILE_CACHE at runtime
      return (
        AreasFlag.WORKING_SET |
        AreasFlag.MODIFIED_PAGE_LIST |
        AreasFlag.STANDBY_LIST |
        AreasFlag.SYSTEM_FILE_CACHE |
        AreasFlag.MODIFIED_FILE_CACHE |
        AreasFlag.REGISTRY_CACHE
      );
    case 'Gaming':
      // Aggressive profile: All available areas for maximum memory freeing
      // Suitable for gaming and resource-intensive applications
      // Includes all areas from Balanced plus:
      // - STANDBY_LIST_LOW: Low-priority standby memory
      // - COMBINED_PAGE_LIST: Most aggressive optimization
      // Note: Backend will validate availability of version-dependent areas at runtime
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