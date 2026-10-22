import type { SnippetDefinition } from '@shared/types/snippets.types'
import { create } from 'zustand'

interface SnippetsState {
  snippets: SnippetDefinition[]
  /** Whether the snippets picker/manager modal is open. */
  isOpen: boolean
  setSnippets: (snippets: SnippetDefinition[]) => void
  upsert: (snippet: SnippetDefinition) => void
  remove: (id: string) => void
  setOpen: (open: boolean) => void
}

/**
 * In-memory mirror of the shared `snippets/library` record plus the picker
 * open state. Persistence and cross-device sync live in hooks/use-snippets.
 */
export const useSnippetsStore = create<SnippetsState>((set) => ({
  snippets: [],
  isOpen: false,
  setSnippets: (snippets) => set({ snippets }),
  upsert: (snippet) =>
    set((state) => {
      const snippets = state.snippets.filter((s) => s.id !== snippet.id)
      snippets.push(snippet)
      // Stable order by name inside the record; groups render grouped anyway.
      snippets.sort((a, b) => a.name.localeCompare(b.name))
      return { snippets }
    }),
  remove: (id) => set((state) => ({ snippets: state.snippets.filter((s) => s.id !== id) })),
  setOpen: (open) => set({ isOpen: open })
}))
