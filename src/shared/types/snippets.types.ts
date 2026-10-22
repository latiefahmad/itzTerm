// Snippets (Termius-style): named, grouped pieces of text or commands the
// user inserts into a terminal (or copies out) with optional `{{var}}`
// placeholders prompted at insert time.
//
// Storage: one shared-LWW record `snippets/library` (see KEY_SCOPES in
// persistence.types.ts) so every device and browser tab sees the same
// library without reloads. The whole list ships each write — like the
// pinned-commands record, values stay small.

export interface SnippetDefinition {
  id: string
  name: string
  /** An optional group label joining snippets in the picker. */
  group?: string
  /** The text inserted. `{{name}}` tokens become insert-time prompts. */
  content: string
  /** One-line hint shown in the picker; may mention the placeholders. */
  description?: string
  updatedAt: string // ISO timestamp
}

/** The persisted value of `snippets/library`. */
export interface SnippetLibrary {
  snippets: SnippetDefinition[]
  updatedAt: string // ISO timestamp
}

/**
 * Placeholder tokens inside a snippet content: `{{name}}` up to a closing
 * pair of double braces on the same line. Markdown/JSON braces `{x}` stay
 * untouched; `{{` must open and `}}` must close.
 */
export function extractSnippetVariables(content: string): string[] {
  const out: string[] = []
  const seen = new Set<string>()
  const re = /\{\{\s*([^{}]+?)\s*\}\}/g
  for (const match of content.matchAll(re)) {
    const name = match[1].trim()
    if (name && !seen.has(name)) {
      seen.add(name)
      out.push(name)
    }
  }
  return out
}

/** Replace placeholders (first occurrence rules of the extraction) with values. */
export function expandSnippet(content: string, values: Record<string, string>): string {
  return content.replace(/\{\{\s*([^{}]+?)\s*\}\}/g, (_whole, name: string) => {
    const value = values[name.trim()]
    return value === undefined ? _whole : value
  })
}
