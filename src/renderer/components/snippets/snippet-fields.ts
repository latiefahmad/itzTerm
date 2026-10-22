// Field shape shared by the snippets editor form and the picker modal.
export interface SnippetEditorField {
  id: string
  name: string
  group: string
  content: string
  description: string
}

export const EMPTY_SNIPPET_FIELD: SnippetEditorField = {
  id: '',
  name: '',
  group: '',
  content: '',
  description: ''
}
