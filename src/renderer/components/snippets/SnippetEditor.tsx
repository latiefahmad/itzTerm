import { useEffect, useRef } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { FIELD_FOCUS_CLASS } from '@/components/ui/panel-styles'
import type { SnippetEditorField } from './snippet-fields'

/** Form used for both "New snippet" and "Edit snippet" inside the picker. */
export function SnippetEditor({
  field,
  onPatch,
  onCancel,
  onSave
}: {
  field: SnippetEditorField
  onPatch: (patch: Partial<SnippetEditorField>) => void
  onCancel: () => void
  onSave: (field: SnippetEditorField) => Promise<void> | void
}): React.JSX.Element {
  const nameRef = useRef<HTMLInputElement>(null)
  const isEdit = field.id !== ''

  useEffect(() => {
    nameRef.current?.focus()
  }, [])

  const submit = (e: React.FormEvent): void => {
    e.preventDefault()
    if (!field.name.trim()) {
      nameRef.current?.focus()
      return
    }
    void onSave(field)
  }

  return (
    <form
      aria-label={isEdit ? 'Edit snippet' : 'New snippet'}
      className="flex w-full max-w-xl flex-col overflow-hidden rounded-lg border border-border bg-card shadow-xl"
      onSubmit={submit}
    >
      <div className="flex h-12 shrink-0 items-center border-b border-border pl-4 pr-1.5">
        <span className="text-sm font-medium">{isEdit ? 'Edit snippet' : 'New snippet'}</span>
      </div>
      <div className="flex flex-col gap-3 p-4">
        <label className="flex flex-col gap-1 text-xs" htmlFor="snippet-name">
          <span className="text-muted-foreground">Name</span>
          <Input
            className={FIELD_FOCUS_CLASS}
            id="snippet-name"
            ref={nameRef}
            value={field.name}
            onChange={(e) => onPatch({ name: e.target.value })}
          />
        </label>
        <label className="flex flex-col gap-1 text-xs" htmlFor="snippet-group">
          <span className="text-muted-foreground">Group</span>
          <Input
            className={FIELD_FOCUS_CLASS}
            placeholder="General"
            id="snippet-group"
            value={field.group}
            onChange={(e) => onPatch({ group: e.target.value })}
          />
        </label>
        <label className="flex flex-col gap-1 text-xs" htmlFor="snippet-content">
          <span className="text-muted-foreground">
            Content — use {'{{name}}'} placeholders to ask at insert time
          </span>
          <textarea
            aria-label="Snippet content"
            className={`${FIELD_FOCUS_CLASS} pointer-coarse:text-base min-h-28 resize-y rounded-lg border border-field-border bg-card px-3 py-2 text-xs`}
            id="snippet-content"
            spellCheck={false}
            value={field.content}
            onChange={(e) => onPatch({ content: e.target.value })}
          />
        </label>
        <label className="flex flex-col gap-1 text-xs" htmlFor="snippet-description">
          <span className="text-muted-foreground">Description (optional)</span>
          <Input
            className={FIELD_FOCUS_CLASS}
            id="snippet-description"
            value={field.description}
            onChange={(e) => onPatch({ description: e.target.value })}
          />
        </label>
        <div className="mt-1 flex justify-end gap-2">
          <Button size="sm" variant="outline" onClick={onCancel} type="button">
            Cancel
          </Button>
          <Button size="sm" type="submit">
            {isEdit ? 'Save' : 'Create'}
          </Button>
        </div>
      </div>
    </form>
  )
}
