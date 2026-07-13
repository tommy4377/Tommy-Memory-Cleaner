import { writable, derived } from 'svelte/store'

export type UpdateStatus =
  | 'idle'
  | 'checking'
  | 'downloading'
  | 'ready'
  | 'installing'
  | 'error'

export interface UpdateState {
  status: UpdateStatus
  availableVersion: string | null
  error: string | null
}

function createUpdateStore() {
  const { subscribe, set, update } = writable<UpdateState>({
    status: 'idle',
    availableVersion: null,
    error: null,
  })

  return {
    subscribe,
    set,
    update,
    reset() {
      set({ status: 'idle', availableVersion: null, error: null })
    },
  }
}

export const updateStore = createUpdateStore()

export const isUpdateReady = derived(updateStore, ($s) => $s.status === 'ready')
export const isUpdating = derived(
  updateStore,
  ($s) => $s.status === 'checking' || $s.status === 'downloading' || $s.status === 'installing',
)
