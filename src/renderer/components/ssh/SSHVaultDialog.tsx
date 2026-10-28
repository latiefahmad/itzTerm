import type {
  SSHVaultExportSummary,
  SSHVaultImportMode,
  SSHVaultImportSummary,
  SSHVaultKeyWarning
} from '@shared/types/ssh.types'
import { useEffect, useId, useMemo, useState } from 'react'
import { toast } from 'sonner'
import { Archive, ArchiveRestore, Download, Eye, EyeOff, ShieldCheck } from '@/components/icons'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { dialogApi } from '@/lib/api'
import { useSSHActions, useSSHProfiles } from '@/stores/ssh-store'

/** Extension the save/open dialogs filter on. */
export const VAULT_EXTENSION = 'itzterm-backup'

export type VaultDialogMode = 'export' | 'import'

interface SSHVaultDialogProps {
  open: boolean
  mode: VaultDialogMode
  onOpenChange: (open: boolean) => void
}

/** Minimum passphrase length, mirroring the backend rule. */
const MIN_PASSPHRASE_LENGTH = 8

/**
 * A key the vault references but this machine does not have. Export reports it
 * so the user is not surprised after moving to the new computer; import
 * reports it because those hosts will fail to connect until the key is copied.
 */
function KeyWarningList({
  warnings
}: {
  warnings: SSHVaultKeyWarning[]
}): React.JSX.Element | null {
  if (warnings.length === 0) return null
  return (
    <div className="rounded-lg border border-border bg-card px-3 py-2.5">
      <p className="text-xs font-medium text-foreground">
        {warnings.length === 1 ? 'A private key is missing' : 'Private keys are missing'}
      </p>
      <p className="mt-1 text-xs text-muted-foreground text-pretty">
        The backup records where each key lived, not the key itself. Copy these files to the new
        computer, or point the host at a different key after importing.
      </p>
      <ul className="mt-2 space-y-1">
        {warnings.map((warning) => (
          <li key={`${warning.profileId}:${warning.path}`} className="min-w-0 text-xs">
            <span className="font-medium text-foreground">{warning.name}</span>
            <span className="ml-1.5 break-all font-mono text-muted-foreground">
              {warning.path || 'no path set'}
            </span>
          </li>
        ))}
      </ul>
    </div>
  )
}

/** Secret input with a show/hide toggle, matching the host editor. */
function PassphraseInput({
  id,
  label,
  value,
  onChange,
  placeholder,
  autoFocus
}: {
  id: string
  label: string
  value: string
  onChange: (value: string) => void
  placeholder: string
  autoFocus?: boolean
}): React.JSX.Element {
  const [visible, setVisible] = useState(false)
  return (
    <div className="space-y-1.5">
      <Label htmlFor={id}>{label}</Label>
      <div className="relative">
        <Input
          id={id}
          type={visible ? 'text' : 'password'}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          autoComplete="new-password"
          autoFocus={autoFocus}
          className="pr-10"
        />
        <button
          type="button"
          onClick={() => setVisible((v) => !v)}
          className="absolute right-1.5 top-1/2 flex size-7 -translate-y-1/2 items-center justify-center rounded-md text-muted-foreground hover:text-foreground"
          aria-label={visible ? 'Hide passphrase' : 'Show passphrase'}
        >
          {visible ? <EyeOff size={14} /> : <Eye size={14} />}
        </button>
      </div>
    </div>
  )
}

function countLabel(count: number, singular: string, plural = `${singular}s`): string {
  return `${count} ${count === 1 ? singular : plural}`
}

/** Export summary shown after a dry run, or after writing the file. */
function ExportResult({
  summary,
  wroteFile
}: {
  summary: SSHVaultExportSummary
  wroteFile: boolean
}): React.JSX.Element {
  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2 rounded-lg border border-border bg-card px-3 py-2.5">
        <ShieldCheck size={16} className="shrink-0 text-muted-foreground" aria-hidden="true" />
        <p className="text-xs text-foreground">
          {wroteFile
            ? `${countLabel(summary.profileCount, 'host')} backed up, encrypted with your passphrase.`
            : `${countLabel(summary.profileCount, 'host')} will be backed up.`}
        </p>
      </div>
      {(summary.withPassword > 0 || summary.withPassphrase > 0) && (
        <p className="text-xs text-muted-foreground">
          Includes {countLabel(summary.withPassword, 'saved password')} and{' '}
          {countLabel(summary.withPassphrase, 'saved key passphrase')}.
        </p>
      )}
      {summary.withKeyMaterial > 0 && (
        <p className="text-xs text-muted-foreground">
          {countLabel(summary.withKeyMaterial, 'private key')} is carried inside the backup, so{' '}
          {summary.withKeyMaterial === 1 ? 'it' : 'these hosts'} will connect without copying key
          files by hand.
        </p>
      )}
      {summary.unreadableSecrets.length > 0 && (
        <div className="rounded-lg border border-border bg-card px-3 py-2.5">
          <p className="text-xs font-medium text-foreground">
            {countLabel(summary.unreadableSecrets.length, 'host')} could not give up its saved
            secret
          </p>
          <p className="mt-1 text-xs text-muted-foreground text-pretty">
            The system keychain refused to read it, so these hosts are in the backup without their
            password. You will be asked for it on the new computer.
          </p>
        </div>
      )}
      <KeyWarningList warnings={summary.missingKeys} />
    </div>
  )
}

/** Import summary: what the merge did, and any keys still needed. */
function ImportResult({ summary }: { summary: SSHVaultImportSummary }): React.JSX.Element {
  const parts = [
    summary.created > 0 ? `${countLabel(summary.created, 'host')} added` : null,
    summary.updated > 0 ? `${countLabel(summary.updated, 'host')} updated` : null,
    summary.skipped > 0 ? `${countLabel(summary.skipped, 'host')} already present` : null,
    summary.deleted > 0 ? `${countLabel(summary.deleted, 'host')} removed` : null
  ].filter(Boolean)

  const restored = [
    summary.credentialsRestored > 0
      ? countLabel(summary.credentialsRestored, 'saved password or passphrase')
      : null,
    summary.keysRestored > 0 ? countLabel(summary.keysRestored, 'private key') : null
  ]
    .filter(Boolean)
    .join(' and ')

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2 rounded-lg border border-border bg-card px-3 py-2.5">
        <ShieldCheck size={16} className="shrink-0 text-muted-foreground" aria-hidden="true" />
        <p className="text-xs text-foreground">
          {parts.length > 0 ? parts.join(', ') : 'Nothing changed'}
          {restored ? `. ${restored} restored.` : '.'}
        </p>
      </div>
      <KeyWarningList warnings={summary.missingKeys} />
    </div>
  )
}

/**
 * Backup and restore of the saved host list, encrypted with a user passphrase.
 *
 * Desktop only: the credentials live in the OS keychain, which the browser
 * client cannot reach. The caller gates on `isTauriContext()`.
 */
export function SSHVaultDialog({
  open,
  mode,
  onOpenChange
}: SSHVaultDialogProps): React.JSX.Element {
  const profiles = useSSHProfiles()
  const { exportVault, importVault } = useSSHActions()
  const passphraseId = useId()
  const confirmId = useId()

  const [passphrase, setPassphrase] = useState('')
  const [confirmation, setConfirmation] = useState('')
  const [importMode, setImportMode] = useState<SSHVaultImportMode>('merge')
  const [busy, setBusy] = useState(false)
  const [exportResult, setExportResult] = useState<SSHVaultExportSummary | null>(null)
  const [importResult, setImportResult] = useState<SSHVaultImportSummary | null>(null)

  // Reset each time the dialog opens so a previous passphrase never lingers.
  useEffect(() => {
    if (!open) return
    setPassphrase('')
    setConfirmation('')
    setImportMode('merge')
    setBusy(false)
    setExportResult(null)
    setImportResult(null)
  }, [open])

  const isExport = mode === 'export'

  const passphraseProblem = useMemo(() => {
    if (!passphrase) return 'Enter a passphrase.'
    if (isExport && passphrase.length < MIN_PASSPHRASE_LENGTH) {
      return `Use at least ${MIN_PASSPHRASE_LENGTH} characters.`
    }
    if (isExport && confirmation !== passphrase) return 'The passphrases do not match.'
    return null
  }, [passphrase, confirmation, isExport])

  const canSubmit = !busy && passphraseProblem === null

  const handleExport = async (): Promise<void> => {
    const destination = await dialogApi.saveFile({
      title: 'Save host backup',
      defaultPath: `itzterm-hosts-${new Date().toISOString().slice(0, 10)}.${VAULT_EXTENSION}`,
      filters: [{ name: 'itzTerm host backup', extensions: [VAULT_EXTENSION] }]
    })
    if (!destination.success) return

    setBusy(true)
    try {
      const summary = await exportVault({ path: destination.data, passphrase })
      if (!summary) {
        toast.error('Could not create the backup.')
        return
      }
      setExportResult(summary)
      toast.success(`Backed up ${countLabel(summary.profileCount, 'host')}`)
    } finally {
      setBusy(false)
    }
  }

  const handleImport = async (): Promise<void> => {
    const source = await dialogApi.selectFile({
      title: 'Open host backup',
      filters: [{ name: 'itzTerm host backup', extensions: [VAULT_EXTENSION] }]
    })
    if (!source.success) return

    setBusy(true)
    try {
      const summary = await importVault({
        path: source.data,
        passphrase,
        mode: importMode
      })
      if (!summary) {
        toast.error('Could not read the backup. Check the passphrase and try again.')
        return
      }
      setImportResult(summary)
      toast.success(
        summary.created + summary.updated > 0 ? 'Hosts restored' : 'No new hosts in that backup'
      )
    } finally {
      setBusy(false)
    }
  }

  const done = exportResult !== null || importResult !== null

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md" data-testid="ssh-vault-dialog">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-base">
            {isExport ? <Archive size={16} /> : <ArchiveRestore size={16} />}
            {isExport ? 'Back up hosts' : 'Restore hosts'}
          </DialogTitle>
          <DialogDescription className="text-xs">
            {isExport
              ? `Saved hosts and their keychain secrets, sealed with a passphrase you choose. ${
                  profiles.length > 0 ? `${countLabel(profiles.length, 'host')} included.` : ''
                }`
              : 'Open a backup from another computer. Hosts are matched by id, then by address and user.'}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {done ? (
            exportResult ? (
              <ExportResult summary={exportResult} wroteFile />
            ) : (
              importResult && <ImportResult summary={importResult} />
            )
          ) : (
            <>
              <PassphraseInput
                id={passphraseId}
                label={isExport ? 'Passphrase' : 'Backup passphrase'}
                value={passphrase}
                onChange={setPassphrase}
                placeholder={isExport ? 'Choose a passphrase' : 'Passphrase for this file'}
                autoFocus
              />

              {isExport && (
                <>
                  <PassphraseInput
                    id={confirmId}
                    label="Confirm passphrase"
                    value={confirmation}
                    onChange={setConfirmation}
                    placeholder="Type it again"
                  />
                  <p className="text-xs text-muted-foreground text-pretty">
                    Without this passphrase the file cannot be opened. There is no recovery.
                  </p>
                </>
              )}

              {!isExport && (
                <fieldset className="space-y-2">
                  <legend className="label-panel mb-2">Existing hosts</legend>
                  {(
                    [
                      {
                        value: 'merge' as const,
                        label: 'Keep them',
                        detail: 'Add unknown hosts; leave the rest alone.'
                      },
                      {
                        value: 'replace' as const,
                        label: 'Replace all',
                        detail: 'Delete hosts that are not in the backup.'
                      }
                    ] satisfies Array<{ value: SSHVaultImportMode; label: string; detail: string }>
                  ).map((option) => (
                    <label
                      key={option.value}
                      className="flex cursor-pointer items-start gap-2.5 rounded-lg border border-border px-3 py-2 transition-colors hover:bg-foreground/[0.03]"
                    >
                      <input
                        type="radio"
                        name="vault-import-mode"
                        value={option.value}
                        checked={importMode === option.value}
                        onChange={() => setImportMode(option.value)}
                        className="mt-0.5"
                      />
                      <span className="min-w-0">
                        <span className="block text-xs font-medium text-foreground">
                          {option.label}
                        </span>
                        <span className="block text-xs text-muted-foreground">{option.detail}</span>
                      </span>
                    </label>
                  ))}
                </fieldset>
              )}

              {passphraseProblem && passphrase.length > 0 && (
                <p className="text-xs text-destructive">{passphraseProblem}</p>
              )}
            </>
          )}
        </div>

        <DialogFooter>
          {done ? (
            <Button type="button" onClick={() => onOpenChange(false)}>
              Done
            </Button>
          ) : (
            <>
              <Button
                type="button"
                variant="ghost"
                onClick={() => onOpenChange(false)}
                disabled={busy}
              >
                Cancel
              </Button>
              <Button
                type="button"
                onClick={() => void (isExport ? handleExport() : handleImport())}
                disabled={!canSubmit}
              >
                {isExport ? <Download /> : <ArchiveRestore />}
                {busy ? 'Working…' : isExport ? 'Choose file…' : 'Choose backup…'}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
