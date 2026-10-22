import { PersistenceKeys } from '@shared/types/persistence.types'
import type { SnippetDefinition, SnippetLibrary } from '@shared/types/snippets.types'
import { useCallback, useEffect } from 'react'
import { persistenceApi } from '@/lib/api'
import { useSharedKeySync } from '@/lib/shared-key-sync'
import { useSnippetsStore } from '@/stores/snippets-store'

// Guard against corrupt or legacy persisted data: only accept a well-formed
// library record (id/name/content strings, optional group/description).
function isSnippetLibrary(data: unknown): data is SnippetLibrary {
  if (!data || typeof data !== 'object') return false
  const record = data as { snippets?: unknown; updatedAt?: unknown }
  if (typeof record.updatedAt !== 'string') return false
  return (
    Array.isArray(record.snippets) &&
    record.snippets.every(
      (s) =>
        !!s &&
        typeof s === 'object' &&
        typeof (s as SnippetDefinition).id === 'string' &&
        typeof (s as SnippetDefinition).name === 'string' &&
        typeof (s as SnippetDefinition).content === 'string'
    )
  )
}

/** Read the library once and keep it live as other devices write it. */
export function useSnippetsLoader(): void {
  const setSnippets = useSnippetsStore((state) => state.setSnippets)

  // A change another device or tab saved: apply it without a reload.
  useSharedKeySync<unknown>(PersistenceKeys.snippets, (change) => {
    if (change.exists && isSnippetLibrary(change.data)) setSnippets(change.data.snippets)
  })

  useEffect(() => {
    async function load(): Promise<void> {
      const result = await persistenceApi.read<unknown>(PersistenceKeys.snippets)
      if (result.success && isSnippetLibrary(result.data)) {
        setSnippets(result.data.snippets)
      }
    }
    load()
  }, [setSnippets])
}

/** Optimistically upsert one snippet, then persist the whole record. */
export function useSaveSnippet(): (snippet: SnippetDefinition) => Promise<void> {
  const upsert = useSnippetsStore((state) => state.upsert)
  return useCallback(
    async (snippet) => {
      upsert(snippet)
      await persistLibrary()
    },
    [upsert]
  )
}

export function useDeleteSnippet(): (id: string) => Promise<void> {
  const remove = useSnippetsStore((state) => state.remove)
  return useCallback(
    async (id) => {
      remove(id)
      await persistLibrary()
    },
    [remove]
  )
}

async function persistLibrary(): Promise<void> {
  const { snippets } = useSnippetsStore.getState()
  await persistenceApi.write(PersistenceKeys.snippets, {
    snippets,
    updatedAt: new Date().toISOString()
  } satisfies SnippetLibrary)
}
