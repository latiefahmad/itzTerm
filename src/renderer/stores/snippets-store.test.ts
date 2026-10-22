import { beforeEach, describe, expect, it } from 'vitest'
import { useSnippetsStore } from './snippets-store'

const base = { content: 'ls -la', updatedAt: '2026-01-01T00:00:00Z' }

describe('snippets-store', () => {
  beforeEach(() => {
    useSnippetsStore.setState({ snippets: [], isOpen: false })
  })

  it('starts empty and closed', () => {
    expect(useSnippetsStore.getState().snippets).toEqual([])
    expect(useSnippetsStore.getState().isOpen).toBe(false)
  })

  it('upserts a new snippet and sorts by name', () => {
    const { upsert } = useSnippetsStore.getState()
    upsert({ id: '1', name: 'lint', ...base })
    upsert({ id: '2', name: 'build', ...base })

    expect(useSnippetsStore.getState().snippets.map((s) => s.name)).toEqual(['build', 'lint'])
  })

  it('upsert replaces an existing snippet in place (re-sorted)', () => {
    const { upsert } = useSnippetsStore.getState()
    upsert({ id: '1', name: 'lint', ...base })
    upsert({ id: '2', name: 'build', ...base })
    upsert({ id: '1', name: 'aaa', ...base })

    expect(useSnippetsStore.getState().snippets.map((s) => s.id)).toEqual(['1', '2'])
  })

  it('remove drops only the matching snippet', () => {
    const { upsert, remove } = useSnippetsStore.getState()
    upsert({ id: '1', name: 'lint', ...base })
    upsert({ id: '2', name: 'build', ...base })
    remove('1')

    expect(useSnippetsStore.getState().snippets.map((s) => s.id)).toEqual(['2'])
  })

  it('setSnippets replaces the whole list (remote sync path)', () => {
    const { setSnippets } = useSnippetsStore.getState()
    setSnippets([{ id: 'r1', name: 'remote', ...base }])

    expect(useSnippetsStore.getState().snippets).toHaveLength(1)
  })

  it('setOpen toggles the picker visibility', () => {
    const { setOpen } = useSnippetsStore.getState()
    setOpen(true)
    expect(useSnippetsStore.getState().isOpen).toBe(true)
    setOpen(false)
    expect(useSnippetsStore.getState().isOpen).toBe(false)
  })
})
