import { describe, expect, it } from 'vitest'
import { BUNDLED_COLOR_THEMES } from './bundled-themes'
import { contrastRatio, hexToOklch } from './color-utils'
import { ITZTERM_DARK_CHROME } from './itzterm-dark-chrome'

describe('itzterm dark chrome', () => {
  it('holds hue across the neutral ramp', () => {
    const steps = [
      ITZTERM_DARK_CHROME.background,
      ITZTERM_DARK_CHROME.card,
      ITZTERM_DARK_CHROME.elevated,
      ITZTERM_DARK_CHROME.muted,
      ITZTERM_DARK_CHROME.border
    ]
    const voidHue = hexToOklch(steps[0]).h
    for (const hex of steps) {
      expect(Math.abs(hexToOklch(hex).h - voidHue)).toBeLessThan(3)
    }
  })

  it('keeps the border at graphite lightness on the void hue', () => {
    const border = hexToOklch(ITZTERM_DARK_CHROME.border)
    const graphite = hexToOklch('#23252a')
    const voidHue = hexToOklch(ITZTERM_DARK_CHROME.background).h
    expect(Math.abs(border.l - graphite.l)).toBeLessThan(0.01)
    expect(Math.abs(border.h - voidHue)).toBeLessThan(3)
    expect(Math.abs(graphite.h - voidHue)).toBeGreaterThan(15)
  })

  it('places the recessed well between obsidian and the border', () => {
    const muted = hexToOklch(ITZTERM_DARK_CHROME.muted).l
    expect(muted).toBeGreaterThan(hexToOklch(ITZTERM_DARK_CHROME.elevated).l)
    expect(muted).toBeLessThan(hexToOklch(ITZTERM_DARK_CHROME.border).l)
  })

  it('keeps bone on void and fog on carbon at AA', () => {
    expect(
      contrastRatio(ITZTERM_DARK_CHROME.foreground, ITZTERM_DARK_CHROME.background)
    ).toBeGreaterThanOrEqual(4.5)
    expect(
      contrastRatio(ITZTERM_DARK_CHROME.mutedForeground, ITZTERM_DARK_CHROME.card)
    ).toBeGreaterThanOrEqual(4.5)
    expect(
      contrastRatio(ITZTERM_DARK_CHROME.secondaryForeground, ITZTERM_DARK_CHROME.elevated)
    ).toBeGreaterThanOrEqual(4.5)
  })

  it('is attached only to the default dark theme', () => {
    expect(BUNDLED_COLOR_THEMES.itzterm.dark.chrome).toEqual(ITZTERM_DARK_CHROME)
    expect(BUNDLED_COLOR_THEMES.itzterm.dark.palette.primary).toBe('#3b82f6')
    expect(BUNDLED_COLOR_THEMES.nord.dark.chrome).toBeUndefined()
    expect(BUNDLED_COLOR_THEMES['itzterm-light'].dark.chrome?.flatElevation).toBe(true)
  })
})
