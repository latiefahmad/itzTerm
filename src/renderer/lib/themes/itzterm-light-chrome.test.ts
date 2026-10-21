import { describe, expect, it } from 'vitest'
import { BUNDLED_COLOR_THEMES } from './bundled-themes'
import { contrastRatio, hexToOklchComponents, oklchComponentsToHex } from './color-utils'
import { ITZTERM_LIGHT_CHROME } from './itzterm-light-chrome'

function emitted(hex: string): string {
  return oklchComponentsToHex(hexToOklchComponents(hex))
}

describe('itzterm light chrome', () => {
  it('uses paper, mist, graphite ink, and a hairline border', () => {
    expect(emitted(ITZTERM_LIGHT_CHROME.background)).toBe('#ffffff')
    expect(emitted(ITZTERM_LIGHT_CHROME.card)).toBe('#ffffff')
    expect(emitted(ITZTERM_LIGHT_CHROME.sidebar ?? '')).toBe('#f9f9f9')
    expect(emitted(ITZTERM_LIGHT_CHROME.foreground)).toBe('#0d0d0d')
    expect(emitted(ITZTERM_LIGHT_CHROME.border)).toBe('#e5e5e5')
    expect(emitted(ITZTERM_LIGHT_CHROME.secondary ?? '')).toBe('#f2f2f2')
    expect(ITZTERM_LIGHT_CHROME.flatElevation).toBe(true)
  })

  it('keeps ink and ash readable on paper', () => {
    expect(
      contrastRatio(ITZTERM_LIGHT_CHROME.foreground, ITZTERM_LIGHT_CHROME.background)
    ).toBeGreaterThanOrEqual(4.5)
    expect(
      contrastRatio(ITZTERM_LIGHT_CHROME.secondaryForeground, ITZTERM_LIGHT_CHROME.background)
    ).toBeGreaterThanOrEqual(4.5)
  })

  it('is attached only to itzTerm light and keeps the current blue', () => {
    expect(BUNDLED_COLOR_THEMES['itzterm-light'].dark.chrome).toEqual(ITZTERM_LIGHT_CHROME)
    expect(BUNDLED_COLOR_THEMES['itzterm-light'].dark.palette.primary).toBe('#3b82f6')
    expect(BUNDLED_COLOR_THEMES['itzterm-light'].dark.palette.success).toBe('#098658')
    expect(BUNDLED_COLOR_THEMES['itzterm-light'].dark.palette.warning).toBe('#cd9731')
    expect(BUNDLED_COLOR_THEMES['itzterm-light'].dark.palette.error).toBe('#cd3131')
    expect(BUNDLED_COLOR_THEMES['github-light'].dark.chrome).toBeUndefined()
    expect(BUNDLED_COLOR_THEMES.itzterm.dark.chrome?.flatElevation).toBeUndefined()
  })
})
