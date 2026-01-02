import type { Profile, Areas } from './types'
import { AreasFlag } from './types'

export function areasForProfile(profile: Profile): Areas {
  switch (profile) {
    case 'Normal':
      // Profilo Normal: Working Set + Registry Cache + Standby List (Low Priority)
      // - Liberazione immediata senza latenza percepibile
      // ~540MB Working Set + ~1.86MB Registry Cache
      // NOTA: MODIFIED_PAGE_LIST non è incluso nel profilo Normal (come da specifiche utente)
      return AreasFlag.WORKING_SET | AreasFlag.REGISTRY_CACHE | AreasFlag.STANDBY_LIST_LOW
    case 'Balanced':
      // Profilo Balanced: Include Normal + System File Cache + File Cache + Standby List (Full)
      // - Refresh profondo del sistema dopo uso intenso
      return (
        AreasFlag.WORKING_SET |
        AreasFlag.REGISTRY_CACHE |
        AreasFlag.STANDBY_LIST |
        AreasFlag.SYSTEM_FILE_CACHE |
        AreasFlag.MODIFIED_FILE_CACHE |
        AreasFlag.STANDBY_LIST_LOW
      )
    case 'Gaming':
      // Profilo Gaming: Include Balanced + Modified Page List + Combined Page List
      // - Reset totale per gaming, tabula rasa della RAM
      return (
        AreasFlag.WORKING_SET |
        AreasFlag.REGISTRY_CACHE |
        AreasFlag.STANDBY_LIST |
        AreasFlag.SYSTEM_FILE_CACHE |
        AreasFlag.MODIFIED_FILE_CACHE |
        AreasFlag.STANDBY_LIST_LOW |
        AreasFlag.MODIFIED_PAGE_LIST |
        AreasFlag.COMBINED_PAGE_LIST
      )
    default:
      return AreasFlag.WORKING_SET
  }
}

export function areaNamesForAreas(areas: Areas): string[] {
  const names: string[] = []
  if (areas & AreasFlag.WORKING_SET) names.push('Working Set')
  if (areas & AreasFlag.MODIFIED_PAGE_LIST) names.push('Modified Pages')
  if (areas & AreasFlag.STANDBY_LIST) names.push('Standby List')
  if (areas & AreasFlag.STANDBY_LIST_LOW) names.push('Low Priority Standby')
  if (areas & AreasFlag.SYSTEM_FILE_CACHE) names.push('System Cache')
  if (areas & AreasFlag.COMBINED_PAGE_LIST) names.push('Combined Pages')
  if (areas & AreasFlag.MODIFIED_FILE_CACHE) names.push('File Cache')
  if (areas & AreasFlag.REGISTRY_CACHE) names.push('Registry Cache')
  return names
}

// Nuova funzione per convertire il numero in stringa di flag
export function areasToString(areas: Areas): string {
  const flags: string[] = []

  if (areas & AreasFlag.COMBINED_PAGE_LIST) flags.push('COMBINED_PAGE_LIST')
  if (areas & AreasFlag.MODIFIED_FILE_CACHE) flags.push('MODIFIED_FILE_CACHE')
  if (areas & AreasFlag.MODIFIED_PAGE_LIST) flags.push('MODIFIED_PAGE_LIST')
  if (areas & AreasFlag.REGISTRY_CACHE) flags.push('REGISTRY_CACHE')
  if (areas & AreasFlag.STANDBY_LIST) flags.push('STANDBY_LIST')
  if (areas & AreasFlag.STANDBY_LIST_LOW) flags.push('STANDBY_LIST_LOW')
  if (areas & AreasFlag.SYSTEM_FILE_CACHE) flags.push('SYSTEM_FILE_CACHE')
  if (areas & AreasFlag.WORKING_SET) flags.push('WORKING_SET')

  return flags.join('|')
}

// Funzione per convertire da stringa a numero (se necessario)
export function stringToAreas(flagString: string): Areas {
  const flags = flagString.split('|')
  let areas = 0

  for (const flag of flags) {
    switch (flag.trim()) {
      case 'COMBINED_PAGE_LIST':
        areas |= AreasFlag.COMBINED_PAGE_LIST
        break
      case 'MODIFIED_FILE_CACHE':
        areas |= AreasFlag.MODIFIED_FILE_CACHE
        break
      case 'MODIFIED_PAGE_LIST':
        areas |= AreasFlag.MODIFIED_PAGE_LIST
        break
      case 'REGISTRY_CACHE':
        areas |= AreasFlag.REGISTRY_CACHE
        break
      case 'STANDBY_LIST':
        areas |= AreasFlag.STANDBY_LIST
        break
      case 'STANDBY_LIST_LOW':
        areas |= AreasFlag.STANDBY_LIST_LOW
        break
      case 'SYSTEM_FILE_CACHE':
        areas |= AreasFlag.SYSTEM_FILE_CACHE
        break
      case 'WORKING_SET':
        areas |= AreasFlag.WORKING_SET
        break
    }
  }

  return areas
}
