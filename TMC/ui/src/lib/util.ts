import type { Config } from './types'
import type { AreasFlag } from './types'

export function setArea(config: Config, flag: AreasFlag, on: boolean): Config {
  const areas = on ? config.memory_areas | flag : config.memory_areas & ~flag
  return { ...config, memory_areas: areas }
}
