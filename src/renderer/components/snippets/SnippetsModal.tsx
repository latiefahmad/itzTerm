import type { SnippetDefinition } from '@shared/types/snippets.types'
import { expandSnippet, extractSnippetVariables } from '@shared/types/snippets.types'
import { AnimatePresence, motion } from 'framer-motion'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { toast } from 'sonner'
import { ConfirmDialog } from '@/components/ConfirmDialog'
import { Pencil, Plus, Search, Terminal, Trash2 } from '@/components/icons'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { useDeleteSnippet, useSaveSnippet } from '@/hooks/use-snippets'
import { terminalApi } from '@/lib/api'
import { logFrontendError } from '@/lib/log-api'
import { randomUUID } from '@/lib/uuid'
import { useProjectStore } from '@/stores/project-store'
import { useSnippetsStore } from '@/stores/snippets-store'
import { useTerminalStore } from '@/stores/terminal-store'
import type { Terminal as TerminalRecord } from '@/types/project'
import { SnippetEditor } from './SnippetEditor'
import { SnippetVariablesDialog } from './SnippetVariablesDialog'
import { EMPTY_SNIPPET_FIELD, type SnippetEditorField } from './snippet-fields'

function toField(snippet: SnippetDefinition): SnippetEditorField {
  return {
    id: snippet.id,
    name: snippet.name,
    group: snippet.group ?? '',
    content: snippet.content,
    description: snippet.description ?? ''
  }
}

function newId(): string {
  return `sn-${randomUUID()}`
}

/**
 * Pick a write target: the project's connected terminal (active first), else
 * any connected terminal. Restored terminals only gain a ptyId after their
 * PTY re-spawns, so a "first terminal" without one is not a real target.
 */
function pickWriteTarget(
  terminals: TerminalRecord[],
  projectId: string,
  activeTerminalId: string
): TerminalRecord | undefined {
  const inProject = terminals.filter((t) => t.projectId === projectId)
  const connected = inProject.filter((t) => t.ptyId !== undefined)
  return (
    connected.find((t) => t.id === activeTerminalId) ??
    connected[0] ??
    inProject[0] ??
    terminals.find((t) => t.ptyId !== undefined)
  )
}

/**
 * The snippets picker/manager (Termius-style). Browse by group, insert into
 * the active terminal (with one combined `{{var}}` prompt) and manage the
 * library. Reads/writes the shared snippets record via use-snippets.
 */
export function SnippetsModal(): React.JSX.Element {
  const isOpen = useSnippetsStore((state) => state.isOpen)
  const setOpen = useSnippetsStore((state) => state.setOpen)
  const snippets = useSnippetsStore((state) => state.snippets)
  const saveSnippet = useSaveSnippet()
  const deleteSnippet = useDeleteSnippet()
  const terminals = useTerminalStore((state) => state.terminals)
  const activeProjectId = useProjectStore((state) => state.activeProjectId)
  const activeTerminalId = useTerminalStore((state) => state.activeTerminalId)
  const writeTarget = useMemo(
    () => pickWriteTarget(terminals, activeProjectId, activeTerminalId),
    [terminals, activeProjectId, activeTerminalId]
  )

  const [query, setQuery] = useState('')
  const [editing, setEditing] = useState<SnippetEditorField | null>(null)
  const [insertTarget, setInsertTarget] = useState<SnippetDefinition | null>(null)
  const [deleting, setDeleting] = useState<SnippetDefinition | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (isOpen) {
      setQuery('')
      setEditing(null)
      setInsertTarget(null)
      setDeleting(null)
      if (document.activeElement instanceof HTMLElement) document.activeElement.blur()
      setTimeout(() => inputRef.current?.focus(), 0)
    }
  }, [isOpen])

  const grouped = useMemo(() => {
    const lowerQuery = query.toLowerCase()
    const matched = !query
      ? snippets
      : snippets.filter(
          (s) =>
            s.name.toLowerCase().includes(lowerQuery) ||
            s.content.toLowerCase().includes(lowerQuery) ||
            (s.group ?? '').toLowerCase().includes(lowerQuery) ||
            (s.description ?? '').toLowerCase().includes(lowerQuery)
        )
    const groups = new Map<string, SnippetDefinition[]>()
    for (const snippet of matched) {
      const key = snippet.group || 'General'
      const list = groups.get(key) ?? []
      list.push(snippet)
      groups.set(key, list)
    }
    return [...groups.entries()].sort(([a], [b]) => a.localeCompare(b))
  }, [snippets, query])

  const insertSnippet = useCallback(
    async (snippet: SnippetDefinition) => {
      const vars = extractSnippetVariables(snippet.content)
      if (vars.length > 0) {
        setInsertTarget(snippet)
        return
      }
      await sendToTerminal(snippet.content, snippet, writeTarget)
      setOpen(false)
    },
    [writeTarget, setOpen]
  )

  const confirmVariables = useCallback(
    async (values: Record<string, string>) => {
      if (!insertTarget) return
      const expanded = expandSnippet(insertTarget.content, values)
      await sendToTerminal(expanded, insertTarget, writeTarget)
      setInsertTarget(null)
      setOpen(false)
    },
    [insertTarget, writeTarget, setOpen]
  )

  const handleSave = useCallback(
    async (field: SnippetEditorField) => {
      const snippet: SnippetDefinition = {
        id: field.id || newId(),
        name: field.name.trim(),
        group: field.group.trim() || undefined,
        content: field.content,
        description: field.description.trim() || undefined,
        updatedAt: new Date().toISOString()
      }
      await saveSnippet(snippet)
      setEditing(null)
    },
    [saveSnippet]
  )

  const handleDelete = useCallback(async () => {
    if (!deleting) return
    await deleteSnippet(deleting.id)
    setDeleting(null)
  }, [deleting, deleteSnippet])

  return (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          animate={{ opacity: 1 }}
          className="fixed inset-0 z-[60] flex flex-col items-center pt-[10vh] bg-overlay/40 backdrop-blur-sm"
          exit={{ opacity: 0 }}
          initial={{ opacity: 0 }}
          transition={{ duration: 0.15 }}
          onClick={(e) => {
            if (e.target === e.currentTarget) setOpen(false)
          }}
          onKeyDown={(e) => {
            if (e.key === 'Escape' && !editing && !deleting) {
              e.preventDefault()
              setOpen(false)
            }
          }}
        >
          {editing ? (
            <SnippetEditor
              field={editing}
              onCancel={() => setEditing(null)}
              onSave={handleSave}
              onPatch={(patch) => setEditing((prev) => (prev ? { ...prev, ...patch } : prev))}
            />
          ) : (
            <div
              aria-label="Snippets"
              className="flex w-full max-w-xl flex-col overflow-hidden rounded-lg border border-border bg-card shadow-xl"
              role="dialog"
            >
              <div className="flex h-12 shrink-0 items-center gap-2 border-b border-border pl-4 pr-2">
                <Terminal aria-hidden="true" size={16} className="shrink-0 text-muted-foreground" />
                <span className="text-sm font-medium">Snippets</span>
                <div className="ml-auto flex items-center gap-1">
                  <Button
                    aria-label="New snippet"
                    size="icon"
                    title="New snippet"
                    variant="ghost"
                    onClick={() => setEditing({ ...EMPTY_SNIPPET_FIELD })}
                  >
                    <Plus aria-hidden="true" size={16} />
                  </Button>
                </div>
              </div>
              <div className="flex items-center gap-2 border-b border-border px-4 py-2">
                <Search aria-hidden="true" size={14} className="shrink-0 text-muted-foreground" />
                <Input
                  className="h-8 border-none bg-transparent focus-visible:outline-hidden"
                  placeholder="Search snippets..."
                  ref={inputRef}
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                />
              </div>
              <div className="max-h-[50vh] min-h-24 overflow-y-auto p-2">
                {grouped.length === 0 ? (
                  <p className="p-4 text-center text-xs text-muted-foreground">
                    {snippets.length === 0
                      ? 'No snippets yet — create one with +'
                      : 'No snippets match your search'}
                  </p>
                ) : (
                  grouped.map(([group, list]) => (
                    <div key={group}>
                      <div className="px-2 pb-1 pt-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                        {group}
                      </div>
                      {list.map((snippet) => (
                        <div
                          key={snippet.id}
                          className="group flex items-center gap-2 rounded-md px-2 py-1.5 hover:bg-popover"
                        >
                          <button
                            className="min-w-0 flex-1 text-left"
                            onClick={() =>
                              insertSnippet(snippet).catch(() => {
                                /* failure already toasted */
                              })
                            }
                            type="button"
                          >
                            <div className="truncate text-sm">{snippet.name}</div>
                            <div className="truncate text-xs text-muted-foreground">
                              {snippet.description?.trim() ||
                                snippet.content.split('\n')[0].slice(0, 80)}
                            </div>
                          </button>
                          <div className="flex shrink-0 items-center opacity-0 transition-opacity group-hover:opacity-100">
                            <Button
                              aria-label={`Edit ${snippet.name}`}
                              size="icon"
                              variant="ghost"
                              onClick={() => setEditing(toField(snippet))}
                            >
                              <Pencil aria-hidden="true" size={14} />
                            </Button>
                            <Button
                              aria-label={`Delete ${snippet.name}`}
                              size="icon"
                              variant="ghost"
                              onClick={() => setDeleting(snippet)}
                            >
                              <Trash2 aria-hidden="true" size={14} />
                            </Button>
                          </div>
                        </div>
                      ))}
                    </div>
                  ))
                )}
              </div>
            </div>
          )}
        </motion.div>
      )}

      <SnippetVariablesDialog
        isOpen={insertTarget !== null}
        snippetName={insertTarget?.name ?? ''}
        variables={insertTarget ? extractSnippetVariables(insertTarget.content) : []}
        onCancel={() => setInsertTarget(null)}
        onConfirm={(values) => {
          confirmVariables(values).catch(() => {
            /* failure already toasted */
          })
        }}
      />

      <ConfirmDialog
        isOpen={deleting !== null}
        message={`Delete "${deleting?.name ?? ''}"? This removes it from every device after sync.`}
        title="Delete snippet"
        variant="danger"
        onCancel={() => setDeleting(null)}
        onConfirm={() => {
          handleDelete().catch(() => {
            /* failure already toasted */
          })
        }}
      />
    </AnimatePresence>
  )
}

async function sendToTerminal(
  text: string,
  snippet: SnippetDefinition,
  target: TerminalRecord | undefined
): Promise<void> {
  if (!target?.ptyId) {
    toast.error(
      `No connected terminal to insert "${snippet.name}" into — open a terminal tab first`
    )
    void logFrontendError({
      level: 'warn',
      message: `snippets: insert target missing (name=${snippet.name}, p=${target?.projectId ?? '-'})`,
      source: 'snippets:insert'
    })
    return
  }
  const result = await terminalApi.write(target.ptyId, text)
  if (!result.success) {
    toast.error(`Failed to insert "${snippet.name}"`)
    void logFrontendError({
      level: 'warn',
      message: `snippets: PTY write failed (name=${snippet.name}, pty=${target.ptyId})`,
      source: 'snippets:insert'
    })
  }
}
