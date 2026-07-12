import type { Profile, Areas } from './types'
import { AreasFlag } from './types'

export function areasForProfile(profile: Profile): Areas {
  switch (profile) {
    case 'Normal':
      // Normal profile: Working Set + Registry Cache + Standby List (Low Priority)
      // - Immediate memory release with no perceptible latency
      // ~540MB Working Set + ~1.86MB Registry Cache
      // NOTE: MODIFIED_PAGE_LIST is intentionally excluded from the Normal profile (per user spec)
      return AreasFlag.WORKING_SET | AreasFlag.REGISTRY_CACHE | AreasFlag.STANDBY_LIST_LOW
    case 'Balanced':
      // Balanced profile: Normal areas + System File Cache + File Cache + Standby List (full)
      // - Deep system refresh after heavy usage
      return (
        AreasFlag.WORKING_SET |
        AreasFlag.REGISTRY_CACHE |
        AreasFlag.STANDBY_LIST |
        AreasFlag.SYSTEM_FILE_CACHE |
        AreasFlag.MODIFIED_FILE_CACHE |
        AreasFlag.STANDBY_LIST_LOW
      )
    case 'Gaming':
      // Gaming profile: Balanced areas + Modified Page List + Combined Page List
      // - Full RAM reset, clean slate for gaming sessions
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

// Converts the numeric bitmask into a pipe-separated flag string for the backend
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

// Converts a pipe-separated flag string back into the numeric bitmask
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
