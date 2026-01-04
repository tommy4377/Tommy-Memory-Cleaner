export type Unit = 'B' | 'KB' | 'MB' | 'GB' | 'TB'

export interface MemorySize {
  value: number
  unit: Unit
  percentage: number
  bytes: number
}

export interface MemoryStats {
  free: MemorySize
  used: MemorySize
  total: MemorySize
}

export interface MemoryInfo {
  physical: MemoryStats
  commit: MemoryStats
  load_percent: number
}

export enum Reason {
  LowMemory = 'LowMemory',
  Manual = 'Manual',
  Schedule = 'Schedule',
}

export enum AreasFlag {
  COMBINED_PAGE_LIST = 1 << 0,
  MODIFIED_FILE_CACHE = 1 << 1,
  MODIFIED_PAGE_LIST = 1 << 2,
  REGISTRY_CACHE = 1 << 3,
  STANDBY_LIST = 1 << 4,
  STANDBY_LIST_LOW = 1 << 5,
  SYSTEM_FILE_CACHE = 1 << 6,
  WORKING_SET = 1 << 7,
}

export type Areas = number
export type Priority = 'Low' | 'Normal' | 'High'
export type Profile = 'Normal' | 'Balanced' | 'Gaming'

export interface TrayConfig {
  show_mem_usage: boolean
  text_color_hex: string
  background_color_hex: string
  transparent_bg: boolean
  warning_level: number
  warning_color_hex: string
  danger_level: number
  danger_color_hex: string
}

export interface Config {
  always_on_top: boolean
  auto_opt_interval_hours: number
  auto_opt_free_threshold: number
  auto_update: boolean

  close_after_opt: boolean
  minimize_to_tray: boolean
  compact_mode: boolean

  font_size: number
  language: string
  theme: string // "light" or "dark"
  main_color_hex: string // Colore principale personalizzabile (deprecated, usa main_color_hex_light/dark)
  main_color_hex_light: string // Colore principale per light theme
  main_color_hex_dark: string // Colore principale per dark theme

  profile: Profile
  memory_areas: Areas
  hotkey: string
  process_exclusion_list: string[]

  run_priority: Priority
  run_on_startup: boolean

  show_opt_notifications: boolean
  request_elevation_on_startup: boolean

  tray: TrayConfig
  
  // Platform detection fields
  platform_detected?: boolean
  is_windows_10?: boolean
}
