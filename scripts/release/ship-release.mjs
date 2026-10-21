#!/usr/bin/env node
/**
 * Release helpers for itzTerm.
 *
 * Modes:
 *   manifest         - scan ./dist release artifacts and emit updater latest.json
 *   bump-changelog   - rewrite CHANGELOG.md for the current tag from ./notes.md
 *
 * Both modes print a concise summary; no secrets are ever read or logged.
 */

import { readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const [, , mode, ...rest] = process.argv

function pull(args, flag) {
  const i = args.indexOf(flag)
  return i === -1 ? null : args[i + 1]
}

function pkgVersion() {
  const pkg = JSON.parse(readFileSync('package.json', 'utf8'))
  if (!pkg.version) throw new Error('package.json has no version field')
  return pkg.version
}

function distDir() {
  return pull(rest, '--dist') ?? 'dist'
}

function findFirst(dir, test) {
  for (const name of readdirSync(dir)) {
    if (test(name)) return name
  }
  return null
}

// Tauri v2 updater platform keys: windows-x86_64, darwin-aarch64, linux-x86_64
// (NSIS/MSI on Windows, .app.tar.gz on macOS, AppImage on Linux).
function writeManifest() {
  const dist = distDir()
  const version = pkgVersion()
  const repo = process.env.GITHUB_REPOSITORY
  const tag = process.env.GITHUB_REF_NAME
  if (!repo) throw new Error('GITHUB_REPOSITORY is not set')
  if (!tag || !tag.startsWith('v'))
    throw new Error(`GITHUB_REF_NAME must start with v (got ${tag})`)

  const pick = (suffix) => {
    const name = findFirst(dist, (n) => n.endsWith(suffix))
    if (!name) return null
    const sigName = `${name}.sig`
    if (!findFirst(dist, (n) => n === sigName)) {
      console.warn(
        `latest.json: artifact ${name} has no updater signature (${sigName}); skipping its platform`
      )
      return null
    }
    return { name, sigName }
  }
  const macos = pick('.app.tar.gz')
  const windows = pick('-setup.exe') ?? pick('.msi')
  const linux = pick('.AppImage')

  const base = `https://github.com/${repo}/releases/download/${tag}`
  const platforms = {}
  const addPlatform = (key, res) => {
    if (!res) {
      console.log(`latest.json: no artifact for ${key}; platform skipped`)
      return
    }
    platforms[key] = {
      signature: readFileSync(join(dist, res.sigName), 'utf8').trim(),
      url: `${base}/${res.name}`
    }
    console.log(`latest.json: ${key} -> ${res.name}`)
  }
  addPlatform('darwin-aarch64', macos)
  addPlatform('windows-x86_64', windows)
  addPlatform('linux-x86_64', linux)
  if (Object.keys(platforms).length === 0) {
    throw new Error('latest.json: no updater artifacts found in dist/')
  }

  const manifest = { version, pub_date: new Date().toISOString(), platforms }
  const out = join(dist, 'latest.json')
  writeFileSync(out, JSON.stringify(manifest, null, 2) + '\n')
  console.log(`latest.json: written (version ${manifest.version})`)
}

// CHANGELOG.md update: turn "## [Unreleased]" into "## [1.0.0] - 2026-10-21",
// keep the existing unreleased bullets (they ship in this release), append the
// generated release notes, and re-open a fresh Unreleased section on top.
function bumpChangelog() {
  const version = pull(rest, '--version') ?? pkgVersion()
  const date = pull(rest, '--date') ?? new Date().toISOString().slice(0, 10)
  const notesPath = pull(rest, '--notes') ?? 'notes.md'
  const notes = readFileSync(notesPath, 'utf8').trim()

  const src = readFileSync('CHANGELOG.md', 'utf8')
  const [title, ...bodyLines] = src.split('\n')
  const body = bodyLines.join('\n')

  const sectionStart = body.indexOf('## [Unreleased]')
  if (sectionStart === -1) throw new Error('CHANGELOG.md has no "## [Unreleased]" section')
  const intro = body.slice(0, sectionStart).trimEnd()
  const unreleasedContent = body
    .slice(sectionStart + '## [Unreleased]'.length)
    .replace(/^[\r\n]+/, '')
    .trimEnd()

  const rows = [title, intro, '', '## [Unreleased]', '', `## [${version}] - ${date}`]
  if (notes) {
    rows.push('', notes)
  }
  if (unreleasedContent) {
    rows.push('', unreleasedContent)
  }
  const text = rows.join('\n') + '\n'
  writeFileSync('CHANGELOG.md', text)
  console.log(`CHANGELOG.md: opened [Unreleased], pinned [${version}] - ${date}`)
}

switch (mode) {
  case 'manifest':
    writeManifest()
    break
  case 'bump-changelog':
    bumpChangelog()
    break
  default:
    throw new Error(`unknown mode: ${mode ?? '(none)'}; use manifest or bump-changelog`)
}
