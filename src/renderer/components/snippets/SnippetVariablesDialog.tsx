import { useEffect, useMemo, useRef, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

interface SnippetVariablesDialogProps {
  isOpen: boolean
  /** Variable names extracted from the snippet content, in order. */
  variables: string[]
  snippetName: string
  onConfirm: (values: Record<string, string>) => void
  onCancel: () => void
}

/**
 * Insert-time prompt: one combined dialog asks for every `{{var}}` in the
 * snippet, then the caller expands and inserts once.
 */
export function SnippetVariablesDialog({
  isOpen,
  variables,
  snippetName,
  onConfirm,
  onCancel
}: SnippetVariablesDialogProps): React.JSX.Element | null {
  const [values, setValues] = useState<Record<string, string>>({})
  const inputRefs = useRef<(HTMLInputElement | null)[]>([])

  useEffect(() => {
    if (isOpen) {
      setValues({})
      inputRefs.current[0]?.focus()
    }
  }, [isOpen])

  const fields = useMemo(() => variables, [variables])
  if (!isOpen || fields.length === 0) return null

  return (
    <div
      aria-label={`Fill variables for ${snippetName}`}
      aria-modal="true"
      className="fixed inset-0 z-[60] flex items-center justify-center bg-overlay/40 p-4 backdrop-blur-sm"
      role="dialog"
      onClick={(e) => {
        if (e.target === e.currentTarget) onCancel()
      }}
      onKeyDown={(e) => {
        if (e.key === 'Escape') {
          e.preventDefault()
          onCancel()
        }
      }}
    >
      <div className="w-full max-w-sm rounded-md border border-border bg-card p-4 shadow-lg">
        <h2 className="mb-1 text-sm font-medium">Fill values — {snippetName}</h2>
        <p className="mb-3 text-xs text-fd-muted-foreground">
          Placeholder values are used once and are not saved.
        </p>
        <div className="flex flex-col gap-2">
          {fields.map((name, index) => (
            <label
              className="flex items-center gap-2 text-xs"
              htmlFor={`snippet-var-${name}`}
              key={name}
            >
              <span className="w-28 shrink-0 truncate text-right font-mono">{`{{${name}}}`}</span>
              <Input
                autoComplete="off"
                id={`snippet-var-${name}`}
                ref={(node) => {
                  inputRefs.current[index] = node
                }}
                value={values[name] ?? ''}
                onClick={(e) => e.stopPropagation()}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault()
                    const next = inputRefs.current[index + 1]
                    if (next) next.focus()
                    else onConfirm(values)
                  }
                }}
                onChange={(e) => setValues((prev) => ({ ...prev, [name]: e.target.value }))}
              />
            </label>
          ))}
        </div>
        <div className="mt-4 flex justify-end gap-2">
          <Button size="sm" variant="outline" onClick={onCancel}>
            Cancel
          </Button>
          <Button size="sm" onClick={() => onConfirm(values)}>
            Insert
          </Button>
        </div>
      </div>
    </div>
  )
}
