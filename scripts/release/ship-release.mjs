#!/usr/bin/env node

/**
 * Release helpers for itzTerm.
 *
 * Modes:
 *   manifest         - scan ./dist release artifacts and emit updater latest.json
 *   bump-changelog   - rewrite CHANGELOG.md for the current tag from ./notes.md
 *   mirror           - upload updater artifacts to the public Cloudflare R2
 *                      mirror so the GitHub repo stays private (the updater
 *                      endpoint reads latest.json from R2, not from GitHub)
 *
 * Modes print a concise summary; no secrets are ever read or logged.
 */

import { execFileSync } from 'node:child_process'
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
  writeFileSync(out, `${JSON.stringify(manifest, null, 2)}\n`)
  console.log(`latest.json: written (version ${manifest.version})`)
}

function bucketName() {
  return pull(rest, '--bucket') ?? process.env.R2_MIRROR_BUCKET ?? 'itzterm-updates'
}

function contentType(name) {
  if (name.endsWith('.json')) return 'application/json'
  if (name.endsWith('.gz')) return 'application/gzip'
  return 'application/octet-stream'
}

function r2Credentials() {
  const token = process.env.CLOUDFLARE_API_TOKEN
  const accountId = process.env.CLOUDFLARE_ACCOUNT_ID
  if (!token || !accountId) {
    throw new Error('mirror: CLOUDFLARE_API_TOKEN / CLOUDFLARE_ACCOUNT_ID are not set')
  }
  return { token, accountId }
}

// Resolve the bucket's public base URL. `R2_MIRROR_BASE_URL` wins if set
// (custom domain); otherwise read the managed r2.dev domain via the API.
async function r2PublicBase(bucket) {
  const override = process.env.R2_MIRROR_BASE_URL
  if (override) return override.replace(/\/+$/, '')
  const { token, accountId } = r2Credentials()
  const res = await fetch(
    `https://api.cloudflare.com/client/v4/accounts/${accountId}/r2/buckets/${bucket}/domains/managed`,
    { headers: { Authorization: `Bearer ${token}` } }
  )
  const data = await res.json()
  if (!data?.success) {
    throw new Error(`mirror: managed domain lookup failed (HTTP ${res.status})`)
  }
  const info = data.result ?? {}
  const domain = info.domain ?? info.managed_domain
  if (!domain) {
    throw new Error(`mirror: bucket ${bucket} has no public r2.dev domain (enable Public access)`)
  }
  if (info.enabled === false) {
    throw new Error(
      `mirror: public access is disabled for domain ${domain} (enable it in R2 settings)`
    )
  }
  return `https://${domain}`
}

export async function mirrorToR2() {
  const dist = distDir()
  const repo = process.env.GITHUB_REPOSITORY
  const tag = process.env.GITHUB_REF_NAME
  if (!repo) throw new Error('mirror: GITHUB_REPOSITORY is not set')
  if (!tag || !tag.startsWith('v')) {
    throw new Error(`mirror: GITHUB_REF_NAME must start with v (got ${tag})`)
  }
  const bucket = bucketName()
  const base = await r2PublicBase(bucket)
  console.log(`mirror: bucket ${bucket}`)

  const files = readdirSync(dist)
  if (!files.includes('latest.json')) {
    throw new Error('mirror: dist/latest.json is missing (run the manifest mode first)')
  }

  // Rewrite updater URLs from github.com to the R2 mirror so clients never
  // touch the private repo.
  const ghPrefix = `https://github.com/${repo}/releases/download/${tag}/`
  const ghLatestPrefix = `https://github.com/${repo}/releases/latest/download/`
  const manifestRaw = readFileSync(join(dist, 'latest.json'), 'utf8')
  let swapped = 0
  const manifestMirror = manifestRaw.replace(/"url":\s*"([^"]+)"/g, (_, url) => {
    if (url.startsWith(ghPrefix)) {
      swapped++
      return `"url":"${base}/releases/${tag}/${url.slice(ghPrefix.length)}"`
    }
    if (url.startsWith(ghLatestPrefix)) {
      swapped++
      return `"url":"${base}/releases/${tag}/${url.slice(ghLatestPrefix.length)}"`
    }
    return `"url":"${url}"`
  })
  if (swapped === 0) {
    throw new Error(`mirror: no GitHub URLs found in latest.json to rewrite`)
  }
  const mirrorFile = join(dist, 'latest-mirror.json')
  writeFileSync(mirrorFile, manifestMirror)

  const uploads = files
    .filter((name) => name !== 'latest-mirror.json')
    .map((name) => ({ key: `releases/${tag}/${name}`, file: join(dist, name) }))
  uploads.push(
    { key: `releases/${tag}/latest.json`, file: mirrorFile },
    { key: 'latest.json', file: mirrorFile }
  )
  for (const { key, file } of uploads) {
    console.log(`mirror: up ${key}`)
    execFileSync(
      'bunx',
      [
        'wrangler@4.95.0',
        'r2',
        'object',
        'put',
        `${bucket}/${key}`,
        '--file',
        file,
        '--remote',
        '--content-type',
        contentType(key)
      ],
      { stdio: ['ignore', 'ignore', 'inherit'] }
    )
  }
  console.log(`mirror: done (${uploads.length} objects) -> ${base}/latest.json`)
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
  const text = `${rows.join('\n')}\n`
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
  case 'mirror':
    mirrorToR2().catch((error) => {
      console.error(String(error?.message ?? error))
      process.exit(1)
    })
    break
  default:
    throw new Error(`unknown mode: ${mode ?? '(none)'}; use manifest, bump-changelog, or mirror`)
}
