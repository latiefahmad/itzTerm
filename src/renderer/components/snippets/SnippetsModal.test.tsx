import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { toast } from 'sonner'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useProjectStore } from '@/stores/project-store'
import { useSnippetsStore } from '@/stores/snippets-store'
import { useSSHStore } from '@/stores/ssh-store'
import { useTerminalStore } from '@/stores/terminal-store'
import { SnippetsModal } from './SnippetsModal'

const { terminalApi } = vi.hoisted(() => ({
  terminalApi: { write: vi.fn(async () => ({ success: true as const, data: undefined })) }
}))

vi.mock('@/lib/api', () => ({ terminalApi }))
vi.mock('@/lib/log-api', () => ({ logFrontendError: vi.fn() }))
vi.mock('@/hooks/use-snippets', () => ({
  useSaveSnippet: () => vi.fn(),
  useDeleteSnippet: () => vi.fn()
}))
vi.mock('sonner', () => ({ toast: { error: vi.fn(), success: vi.fn() } }))

const SNIPPET = {
  id: 'sn-1',
  name: 'htop',
  content: 'htop',
  updatedAt: '2026-01-01T00:00:00.000Z'
}

function openPicker() {
  useSnippetsStore.setState({ snippets: [SNIPPET], isOpen: true })
  return render(<SnippetsModal />)
}

function clickSnippet() {
  const title = screen.getAllByText('htop', { selector: '.truncate' })[0]
  fireEvent.click(title)
}

describe('SnippetsModal insert target', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useProjectStore.setState({ activeProjectId: 'proj-1' })
    useTerminalStore.setState({ terminals: [], activeTerminalId: '', ptyIdIndex: new Map() })
    useSSHStore.setState({
      profiles: [
        {
          id: 'profile-1',
          name: 'prod-box',
          host: '10.0.0.5',
          port: 22,
          username: 'root',
          authMethod: 'agent'
        }
      ] as never,
      sessions: [],
      activeTab: 'hosts',
      workspaceOpen: false
    } as never)
  })

  it('writes into the SSH shell that is on screen, not the workspace terminal', async () => {
    useTerminalStore.setState({
      terminals: [
        {
          id: 'term-1',
          name: 'local',
          projectId: 'proj-1',
          ptyId: 'pty-local',
          shell: 'bash',
          cwd: '/w'
        }
      ],
      activeTerminalId: 'term-1'
    } as never)
    useSSHStore.setState({
      workspaceOpen: true,
      sessions: [
        {
          id: 'ssh-tab',
          attemptId: 'ssh-tab',
          profileId: 'profile-1',
          status: 'connected',
          stage: null,
          hostKeyPrompt: null,
          credentialPrompt: null,
          error: null,
          ended: null,
          ptyId: 'pty-ssh',
          claim: 'c',
          attached: true,
          connectedAt: '2026-01-01T00:00:00.000Z'
        }
      ],
      activeTab: 'ssh-tab'
    } as never)

    openPicker()
    clickSnippet()

    await waitFor(() => expect(terminalApi.write).toHaveBeenCalledWith('pty-ssh', 'htop'))
    expect(terminalApi.write).not.toHaveBeenCalledWith('pty-local', expect.anything())
  })

  it('writes into the SSH shell when no workspace terminal is connected', async () => {
    useSSHStore.setState({
      workspaceOpen: true,
      sessions: [
        {
          id: 'ssh-tab',
          attemptId: 'ssh-tab',
          profileId: 'profile-1',
          status: 'connected',
          stage: null,
          hostKeyPrompt: null,
          credentialPrompt: null,
          error: null,
          ended: null,
          ptyId: 'pty-ssh',
          claim: 'c',
          attached: true,
          connectedAt: '2026-01-01T00:00:00.000Z'
        }
      ],
      activeTab: 'ssh-tab'
    } as never)

    openPicker()
    clickSnippet()

    await waitFor(() => expect(terminalApi.write).toHaveBeenCalledWith('pty-ssh', 'htop'))
  })

  it('keeps the workspace terminal while the SSH host list is on screen', async () => {
    useTerminalStore.setState({
      terminals: [
        {
          id: 'term-1',
          name: 'local',
          projectId: 'proj-1',
          ptyId: 'pty-local',
          shell: 'bash',
          cwd: '/w'
        }
      ],
      activeTerminalId: 'term-1'
    } as never)
    useSSHStore.setState({
      workspaceOpen: true,
      sessions: [
        {
          id: 'ssh-tab',
          attemptId: 'ssh-tab',
          profileId: 'profile-1',
          status: 'connected',
          stage: null,
          hostKeyPrompt: null,
          credentialPrompt: null,
          error: null,
          ended: null,
          ptyId: 'pty-ssh',
          claim: 'c',
          attached: true,
          connectedAt: '2026-01-01T00:00:00.000Z'
        }
      ],
      activeTab: 'hosts'
    } as never)

    openPicker()
    clickSnippet()

    await waitFor(() => expect(terminalApi.write).toHaveBeenCalledWith('pty-local', 'htop'))
  })

  it('falls back to the workspace terminal once the SSH workspace is closed', async () => {
    useTerminalStore.setState({
      terminals: [
        {
          id: 'term-1',
          name: 'local',
          projectId: 'proj-1',
          ptyId: 'pty-local',
          shell: 'bash',
          cwd: '/w'
        }
      ],
      activeTerminalId: 'term-1'
    } as never)

    openPicker()
    clickSnippet()

    await waitFor(() => expect(terminalApi.write).toHaveBeenCalledWith('pty-local', 'htop'))
  })

  it('toasts instead of writing when no pty is reachable', async () => {
    openPicker()
    clickSnippet()

    await waitFor(() => expect(toast.error).toHaveBeenCalled())
    expect(terminalApi.write).not.toHaveBeenCalled()
  })
})
