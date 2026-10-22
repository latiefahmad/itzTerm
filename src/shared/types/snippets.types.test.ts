import { describe, expect, it } from 'vitest'
import { expandSnippet, extractSnippetVariables } from './snippets.types'

describe('extractSnippetVariables', () => {
  it('finds unique placeholder names in order', () => {
    expect(extractSnippetVariables('docker run {{image}} -p {{port}}:{{port}}')).toEqual([
      'image',
      'port'
    ])
  })

  it('trims whitespace inside the braces', () => {
    expect(extractSnippetVariables('ssh {{ host }}')).toEqual(['host'])
  })

  it('ignores single braces (shell scripts / markdown)', () => {
    expect(extractSnippetVariables('echo $' + '{VAR} {a:b} git {push}')).toEqual([])
  })

  it('ignores unbalanced double braces', () => {
    expect(extractSnippetVariables('{{open only')).toEqual([])
    expect(extractSnippetVariables('close only}}')).toEqual([])
  })
})

describe('expandSnippet', () => {
  it('replaces every occurrence with the provided value', () => {
    expect(
      expandSnippet('docker run -p {{port}}:{{port}} {{image}}', { port: '8080', image: 'nginx' })
    ).toBe('docker run -p 8080:8080 nginx')
  })

  it('leaves placeholders without values untouched', () => {
    expect(expandSnippet('deploy {{target}} now', {})).toBe('deploy {{target}} now')
  })

  it('keeps shell variable syntax intact', () => {
    const input = 'echo $' + '{HOME} && cat {file}'
    expect(expandSnippet(input, {})).toBe(input)
  })
})
