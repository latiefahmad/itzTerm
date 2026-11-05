import { describe, expect, it } from 'vitest'
import {
  CODEX_AUTH_POLL_MS,
  CODEX_AUTH_POLL_UNAVAILABLE_MS,
  codexAuthPollDelay,
  codexAuthSyncDecision,
  codexAuthUnavailableStreak,
  isCodexAcpConfig
} from './codex-cli-auth'

describe('isCodexAcpConfig', () => {
  it('matches the registry id and the package argument', () => {
    expect(isCodexAcpConfig({ id: 'acp-registry:codex-acp' })).toBe(true)
    expect(isCodexAcpConfig({ args: ['-y', '@agentclientprotocol/codex-acp@1.12.0'] })).toBe(true)
    expect(isCodexAcpConfig({ id: 'acp-registry:claude-acp' })).toBe(false)
    expect(isCodexAcpConfig({ id: 'custom-codex-acp-wrapper' })).toBe(false)
    expect(isCodexAcpConfig({ args: ['/opt/not-codex-acp/bin'] })).toBe(false)
  })
})

describe('codexAuthSyncDecision', () => {
  it('records the first sample and does not restart', () => {
    expect(
      codexAuthSyncDecision({
        previous: null,
        next: 'signed-out',
        authBusy: false,
        hasLiveAgent: true,
        hasAuthError: false
      }).action
    ).toBe('none')
  })

  it('restarts a live agent when the CLI logs out', () => {
    expect(
      codexAuthSyncDecision({
        previous: 'signed-in',
        next: 'signed-out',
        authBusy: false,
        hasLiveAgent: true,
        hasAuthError: false
      }).action
    ).toBe('refresh-after-cli-logout')
  })

  it('restarts when the CLI logs in over an auth error', () => {
    expect(
      codexAuthSyncDecision({
        previous: 'signed-out',
        next: 'signed-in',
        authBusy: false,
        hasLiveAgent: true,
        hasAuthError: true
      }).action
    ).toBe('refresh-after-cli-login')
  })

  it('leaves a working agent alone when the CLI is already signed in', () => {
    expect(
      codexAuthSyncDecision({
        previous: 'signed-out',
        next: 'signed-in',
        authBusy: false,
        hasLiveAgent: true,
        hasAuthError: false
      }).action
    ).toBe('none')
  })

  it('does nothing while itzTerm sign-in is in progress', () => {
    expect(
      codexAuthSyncDecision({
        previous: 'signed-out',
        next: 'signed-in',
        authBusy: true,
        hasLiveAgent: true,
        hasAuthError: true
      }).action
    ).toBe('none')
  })
})

describe('codexAuthPollDelay', () => {
  it('polls at the normal rate while the CLI answers', () => {
    expect(codexAuthPollDelay(0)).toBe(CODEX_AUTH_POLL_MS)
  })

  it('backs off hard once the CLI is unavailable', () => {
    expect(codexAuthPollDelay(1)).toBe(CODEX_AUTH_POLL_UNAVAILABLE_MS)
    expect(codexAuthPollDelay(500)).toBe(CODEX_AUTH_POLL_UNAVAILABLE_MS)
  })

  it('keeps the backed-off interval well above the old 4s tick', () => {
    // 900 spawns/hour was the reported cost of the old fixed interval.
    expect(CODEX_AUTH_POLL_MS).toBeGreaterThanOrEqual(30_000)
    expect(CODEX_AUTH_POLL_UNAVAILABLE_MS).toBeGreaterThanOrEqual(300_000)
  })
})

describe('codexAuthUnavailableStreak', () => {
  it('counts consecutive unavailable answers', () => {
    expect(codexAuthUnavailableStreak(0, 'unavailable')).toBe(1)
    expect(codexAuthUnavailableStreak(4, 'unavailable')).toBe(5)
  })

  it('resets as soon as the CLI answers', () => {
    expect(codexAuthUnavailableStreak(9, 'signed-in')).toBe(0)
    expect(codexAuthUnavailableStreak(9, 'signed-out')).toBe(0)
  })

  it('drives the delay back to normal after a successful probe', () => {
    const streak = codexAuthUnavailableStreak(3, 'signed-in')
    expect(codexAuthPollDelay(streak)).toBe(CODEX_AUTH_POLL_MS)
  })
})
