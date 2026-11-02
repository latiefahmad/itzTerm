<div align="center">

# 🖥️ itzTerm

**A modern, project-aware terminal manager built with Tauri**

itzTerm treats workspaces as first-class citizens. Organize terminals by project with persistent sessions, snapshots, and a clean tabbed interface — then add ACP agent chats, Git and worktree tools, SSH sessions, an embedded browser, and remote access from your phone.

[![GitHub Stars](https://img.shields.io/github/stars/latiefahmad/itzTerm?style=social)](https://github.com/latiefahmad/itzTerm/stargazers)
[![GitHub Issues](https://img.shields.io/github/issues/latiefahmad/itzTerm)](https://github.com/latiefahmad/itzTerm/issues)
[![License](https://img.shields.io/github/license/latiefahmad/itzTerm)](LICENSE)
[![Latest Release](https://img.shields.io/github/v/release/latiefahmad/itzTerm?include_prereleases)](https://github.com/latiefahmad/itzTerm/releases)

[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)](https://github.com/latiefahmad/itzTerm)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-FFC131?logo=tauri&logoColor=white)](https://tauri.app)
[![React](https://img.shields.io/badge/React-18-61DAFB?logo=react&logoColor=white)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org)

[Getting Started](#-getting-started) · [Features](#-features) · [Shortcuts](#-keyboard-shortcuts) · [Website](https://latiefahmad.github.io/itzTerm/) · [Documentation](#-documentation) · [Contributing](CONTRIBUTING.md) · [Report Bug](https://github.com/latiefahmad/itzTerm/issues/new?template=bug_report.md) · [Request Feature](https://github.com/latiefahmad/itzTerm/issues/new?template=feature_request.md)

</div>

---

## ✨ Features

### 🪟 Workspace & Terminal Management

| Feature | Description |
| --- | --- |
| **Project-Based Workspaces** | Organize terminals by project with dedicated workspace directories, separate state, and per-project configuration |
| **Pane-Based Split Layout** | Split your workspace into resizable panes and arrange terminals, editors, browser tabs, and canvases side by side |
| **Tabbed Interface** | Windows Terminal-style tab bar with drag-and-drop reordering within and across panes, terminal rename, and a context menu |
| **Multiple Shell Support** | Detects PowerShell, CMD, Git Bash, and WSL on Windows, and bash, zsh, fish, and sh on macOS and Linux. Pick a shell for each new terminal |
| **Terminal Profiles** | Save named shell profiles with a shell path, arguments, working directory, and environment variables, then open them from the new-tab menu |
| **Snapshots** | Save the layout and tabs of a workspace as a named snapshot and restore it later |
| **Command Snippets** | Keep a reusable library of commands and text in named groups, insert one into the active terminal with `Ctrl+Shift+S`, and fill `{{placeholders}}` at insert time. The library syncs across devices |

### 🤖 AI Agents

| Feature | Description |
| --- | --- |
| **Agent Chat** | Chat with ACP agents such as Claude Agent, Codex, Gemini CLI, Cursor, and OpenCode in any pane, with a model picker, slash commands, `@` file mentions, permission prompts, and a plan view |
| **Saved Chats** | itzTerm saves chats per project and lists them in the sidebar. Reopen a chat when its agent can reload a past session |
| **Agent Launcher** | Start a chat from a prompt in the active pane (`Ctrl+T`), pick the agent and model, and optionally isolate the chat in a new Git worktree |
| **Custom ACP Agents** | Add your own ACP agents in Settings, or paste a Zed-style `agent_servers` config |
| **Terminal AI Assist** | Right-click selected terminal output and choose **Explain with AI** or **Fix Command with AI**. Suggested commands insert at the prompt without running |
| **Turn Guardrails** | Agent turns stop at a configurable step budget, on a repeated identical tool call, and on idle or turn timeouts, so a stuck agent cannot run forever |
| **Deletion Guardrail** | Permission requests that would delete files or directories never offer an allow-always grant, and are flagged by the command they run rather than the tool name |
| **Run on a Remote Host** | Drive the same agent chats from a browser or phone through remote access, with the session living on your desktop |

### 🌿 Git & Worktrees

| Feature | Description |
| --- | --- |
| **Changes & Commits** | Stage, unstage, and discard files, then commit, amend, push, or publish the branch to `origin` |
| **Partial Staging** | Stage or unstage a single hunk or a selection of lines from the diff view |
| **Diff Views** | Inline and side-by-side diffs with word-level highlighting, opened as tabs |
| **History Graph** | Commit graph for all branches or the current branch, with search by message, author, or hash |
| **Branches & Stash** | Search, switch, and create branches, and save, apply, pop, or drop stashes |
| **AI Commit Messages** | Generate a commit message from the staged diff with the selected ACP agent |
| **Worktrees** | Create a worktree from any base branch and switch between worktrees from the workspace switcher |
| **Status Bar** | Shows the current branch, working directory, git status, and the last exit code |

### 🔐 SSH & SFTP

| Feature | Description |
| --- | --- |
| **Native SSH** | SSH runs on [`russh`](https://crates.io/crates/russh) inside the app, so no system `ssh`, OpenSSL, or Perl is required on any platform |
| **Saved Hosts** | Save hosts with password, key file, or SSH agent authentication, or import them from `~/.ssh/config`. Passwords and passphrases go to the OS keychain |
| **SSH Sessions** | Each session tab is one connection, and its remote shell runs as a regular itzTerm terminal. itzTerm asks you to confirm a new or changed host key by its SHA256 fingerprint |
| **SFTP Browser & Editor** | Browse, upload, download, rename, and delete remote files, and edit them in place in a built-in editor |
| **Port Forwarding** | Local port forwarding over the session's connection, with `direct-tcpip` channels |

### 📱 Remote & Mobile

| Feature | Description |
| --- | --- |
| **Remote Access** | Turn on remote access from the desktop status bar to open a cloudflared quick tunnel, then scan the QR code to drive your live agent chats from a phone. Requires `cloudflared` on your `PATH` |
| **itzterm-server** | Standalone Linux server that serves the itzTerm web client from a headless host, with token auth and opt-in self-update. See [Installing itzterm-server](docs/deployment-guide.md#installing-itzterm-server) |
| **Mobile UI & PWA** | Phone-width layout with a chat drawer, a file explorer, and extra terminal keys. Install the web client as a PWA over HTTPS or `localhost` |

### 📝 Editor, Canvas & Files

| Feature | Description |
| --- | --- |
| **Code Editor** | Built-in code editor with syntax highlighting, file buffers, dirty-state tracking, and save/reload |
| **Markdown Editor** | Rich markdown editing powered by BlockNote with a preview and source toggle and an outline for heading navigation |
| **Mermaid Diagrams** | Render Mermaid diagrams inline in the markdown preview |
| **Canvas** | Open a [FigCraft](https://github.com/storytold/figcraft) design canvas as a tab, with theme and locale inherited from itzTerm and autosaved documents |
| **File Explorer** | Full file tree with create, rename, delete, clipboard operations, drag-and-drop, and context menus |
| **Project Search** | Search file names and file contents from the file explorer with a bundled ripgrep. Results stream in as ripgrep finds them |
| **File Watching** | Live file watching for real-time updates as files change on disk |

### 🌐 Browser & Annotation

| Feature | Description |
| --- | --- |
| **Embedded Browser Tabs** | Browse the web inside your workspace in child webview tabs, with no app switching |
| **Page Annotations** | The Agentation toolbar loads into each browser tab. Click elements, text, or areas to attach notes |

### ⚡ Power User Tools

| Feature | Description |
| --- | --- |
| **Command Palette** | Global command launcher (`Ctrl+K` / `Ctrl+Shift+P`) for project switching, workspace actions, snippets, and more |
| **Command History** | Per-project and aggregate command history viewer with search (`Ctrl+R`) |
| **Keyboard Shortcuts** | Rebind most shortcuts in Settings, with a recorder that shows conflicts |
| **Notifications** | Desktop notifications when a terminal goes idle, an agent turn finishes, or a chat needs your input |
| **Custom Title Bar** | Frameless window with built-in window controls on Windows and Linux, and native traffic lights on macOS |

### 🔧 System & Reliability

| Feature | Description |
| --- | --- |
| **Auto-Updater** | Built-in update infrastructure with signed artifacts — get notified and update without leaving the app |
| **State Management** | Zustand-powered reactive stores for projects, terminals, workspace layout, editor buffers, browser sessions, and settings |
| **Configurable Settings** | Terminal and UI preferences, color themes and appearance, UI zoom, agent timeouts, notifications, shell profiles, and keyboard shortcuts |
| **Cross-Platform** | Works on Windows, macOS, and Linux with native platform packaging |
| **Error Boundaries** | Graceful error handling with runtime error boundaries and user-friendly fallback UI |

<details>
<summary>🗺️ Feature Map — Component Overview</summary>

| Domain | Key Components | Zustand Store |
| --- | --- | --- |
| **Workspace** | `WorkspaceLayout`, `PaneRenderer`, `PaneContent`, `WorkspaceTabBar`, `NewTabMenu` | `workspace-store` |
| **Terminal** | `ConnectedTerminal`, `TerminalAssistPanel` | `terminal-store` |
| **Snippets** | `SnippetsModal`, `SnippetEditor`, `SnippetVariablesDialog` | `snippets-store` |
| **Agents** | `AgentLauncher`, `AgentChatPanel`, `ChatInputBar`, `CustomAcpAgentDialog` | `acp-store` |
| **Git** | `ChangesPanel`, `GitPanel`, `GitDiffView`, `GitHistoryPanel`, `GitBranchPicker` | `git-status-store`, `git-history-store` |
| **Worktrees** | `WorkspaceSwitcher`, `NewWorktreeModal` | — |
| **SSH & SFTP** | `SSHWorkspace`, `SSHPanel`, `SSHFileExplorer`, `SSHFileEditor` | `ssh-store` |
| **Remote & Mobile** | `RemoteAccessPopover`, `MobileWorkspaceShell`, `MobileChatShell` | `remote-status-store` |
| **Editor** | `EditorPanel`, `CodeEditor`, `MarkdownEditor`, `EditorToolbar`, `MermaidBlock` | `editor-store` |
| **Canvas** | `CanvasPanel`, `CanvasMountLayer`, `CanvasRecovery` | `canvas-store` |
| **Browser** | `BrowserPanel`, `BrowserControls` | `browser-session-store` |
| **File Explorer** | `FileExplorer`, `FileTreeNode`, `FileTreeContextMenu` | `file-explorer-store` |
| **Snapshots** | `CreateSnapshotModal`, `RestoreSnapshotModal`, `DeleteSnapshotModal` | `snapshot-store` |
| **Projects** | `ProjectSidebar`, `NewProjectModal` | `project-store` |
| **Settings** | `ShortcutRecorder`, `ColorPickerPopover`, `ContextBarSettingsPopover` | `app-settings-store`, `context-bar-settings-store`, `keyboard-shortcuts-store` |
| **Updates** | `UpdateAvailableToast` | `updater-store` |
| **Shared** | `CommandPalette`, `GlobalContextMenu`, `ConfirmDialog`, `ErrorBoundary` | — |

</details>

## 📦 Install

### Homebrew (macOS)

```bash
brew tap latiefahmad/itzTerm
brew install --cask itzterm
```

### curl (macOS/Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/latiefahmad/itzTerm/main/scripts/install.sh | bash
```

Windows users should install the `.exe` or `.msi` from [GitHub Releases](https://github.com/latiefahmad/itzTerm/releases). Manual DMG downloads in a browser may still hit Gatekeeper, so macOS users should prefer Homebrew or curl.

## 🚀 Getting Started

### Prerequisites

| Dependency | Version | Notes |
| --- | --- | --- |
| [Bun](https://bun.sh) | 1.3+ | JavaScript runtime and package manager |
| [Rust](https://www.rust-lang.org/tools/install) | Latest stable | Required for Tauri builds |

#### Platform-Specific Requirements

<details>
<summary>Windows</summary>

- Microsoft Visual C++ Build Tools (included in Visual Studio 2022)
- [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (pre-installed on Windows 10+)

</details>

<details>
<summary>macOS</summary>

```bash
xcode-select --install
```

</details>

<details>
<summary>Linux (Debian/Ubuntu)</summary>

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev \
    build-essential curl wget file \
    libxdo-dev libssl-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev patchelf
```

</details>

<details>
<summary>Linux (Fedora)</summary>

```bash
sudo dnf install webkit2gtk4.1-devel \
    gcc gcc-c++ libopenssl-devel \
    appindicator-devel librsvg2-devel \
    patchelf
```

</details>

### Install Rust Toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustc --version && cargo --version
```

### Quick Start

```bash
# Clone the repository
git clone https://github.com/latiefahmad/itzTerm.git
cd itzterm

# Install dependencies
bun install

# Launch in development mode
bun run dev
```

### Landing Page

There are two landing pages in this repository.

**`site/`** — the project site, served by GitHub Pages at <https://latiefahmad.github.io/itzTerm/>.
It is a single self-contained `index.html` with no build step: download buttons that match the
visitor's OS plus a feature and shortcut summary. Edit the file and refresh — there is nothing
to compile.

```bash
# Preview locally (any static server works)
bunx serve site

# Publish: push to main (only site/ changes trigger the workflow)
git add site && git commit -m "docs(site): ..." && git push
```

The deploy workflow is `.github/workflows/deploy-site.yml`. GitHub Pages must be set to
**Source: GitHub Actions** in the repository settings; nothing is committed as build output.

**`landing/`** — the Cloudflare Pages marketing site (`itzterm.dev`), a Vite + React app with
prerendering and D1/R2-backed testimonials.

```bash
# Install landing page dependencies (from landing/)
cd landing && bun install

# Start the landing page dev server
bun run landing:dev

# Lint the landing page
bun run landing:lint

# Build the landing page for production
bun run landing:build
```

### Building for Production

```bash
# Build for your current platform
bun run build

# Platform-specific builds
bun run build:tauri:win        # Windows (x64)
bun run build:tauri:mac-arm    # macOS (Apple Silicon)
bun run build:tauri:mac-x64    # macOS (Intel)
bun run build:tauri:linux      # Linux (x64)

# Debug build (faster compilation, larger binary)
bun run build:tauri:debug
```

Build output: `src-tauri/target/release/bundle/`

## 📖 Documentation

### Usage

#### Creating a Project

1. Click the **+** button in the sidebar and choose **New project**, or press `Ctrl+N`
2. Select a workspace directory
3. Configure your default shell under **Advanced** (optional)

#### Terminal Tabs

| Action | How |
| --- | --- |
| New terminal | Click **+** next to the tabs and pick a shell |
| Reorder tabs | Drag and drop, within a pane or to another pane |
| Rename tab | Double-click a terminal tab |
| Context menu | Right-click (rename, close, close others, kill process) |

#### Snippets

Press `Ctrl+Shift+S` (or pick **Snippets** in the command palette) to open the library.

| Action | How |
| --- | --- |
| Insert a snippet | Click it; the text is typed into the terminal you are looking at — a local terminal or the active SSH session |
| Use placeholders | Write `{{name}}` in the content; itzTerm asks for each value before inserting |
| Organize | Give snippets a group so the picker lists them together |
| Create or edit | Use **+** in the modal, or the pencil on a row |

#### SSH Sessions

Add a host from the SSH workspace sidebar, entering a host, port, user, and auth method (password,
key file, or agent). On connect, itzTerm shows the host key fingerprint for a first-seen or changed
key and waits for your confirmation. Each open tab is one connection, and its shell behaves like a
local terminal — including snippets, search, and AI assist.

### Keyboard Shortcuts

| Action | Default Shortcut |
| --- | --- |
| Command Palette | `Ctrl+K` |
| Command Palette (VS Code style) | `Ctrl+Shift+P` |
| Snippets | `Ctrl+Shift+S` |
| Command History | `Ctrl+R` |
| Agent Launcher | `Ctrl+T` |
| New Project | `Ctrl+N` |
| New Browser Tab | `Ctrl+Shift+N` |
| Next Tab | `Ctrl+PageDown` |
| Previous Tab | `Ctrl+PageUp` |
| Close Tab | `Ctrl+W` |
| Terminal Search | `Ctrl+F` |
| Save File | `Ctrl+S` |
| Toggle Side Panel | `Ctrl+B` |
| Toggle Projects | `Ctrl+Shift+B` |
| Create Worktree | `Ctrl+Shift+Alt+N` |
| Open Terminal in Worktree | `Ctrl+Shift+Alt+T` |
| Next / Previous Worktree | `Ctrl+Shift+↓` / `Ctrl+Shift+↑` |
| Switch to Project Root | `Ctrl+Shift+Home` |
| Change Color Theme | `Ctrl+Alt+T` |
| Settings | `Ctrl+,` |
| Zoom In / Out / Reset | `Ctrl+=` / `Ctrl+-` / `Ctrl+0` |
| Rename File | `F2` |
| Delete Files | `Delete` |

> Every shortcut above is rebindable in **Settings → Keyboard Shortcuts**. On macOS, use `Command`
> in place of `Ctrl`. Browser-reserved combinations are not used as defaults because a webview
> cannot reliably intercept them. In the browser client those defaults move to `Alt`-based keys
> (`Alt+N`, `Alt+T`, `Alt+Shift+N`, `Alt+W`, `Alt+PageUp/PageDown`, `Alt+=/-/0`, `Alt+,`), and
> `Ctrl+R` is left unbound so it does not fight the browser's reload.

### Architecture

#### Tech Stack

| Layer | Technology |
| --- | --- |
| Desktop Runtime | [Tauri 2.0](https://tauri.app) |
| Backend | [Rust](https://www.rust-lang.org) |
| UI Framework | [React 18](https://react.dev) |
| Type System | [TypeScript](https://www.typescriptlang.org) |
| Build Tool | [Vite](https://vitejs.dev) |
| Styling | [Tailwind CSS](https://tailwindcss.com) + [shadcn/ui](https://ui.shadcn.com) |
| State Management | [Zustand](https://zustand-demo.pmnd.rs) |
| Terminal Emulation | [tauri-pty](https://github.com/nicegram/tauri-pty) + [xterm.js](https://xtermjs.org) |
| SSH | [russh](https://crates.io/crates/russh) |
| Animations | [Framer Motion](https://www.framer.com/motion) |

#### Tauri Plugins

| Plugin | Purpose |
| --- | --- |
| `@tauri-apps/plugin-fs` | Filesystem access |
| `@tauri-apps/plugin-store` | Configuration persistence |
| `@tauri-apps/plugin-os` | OS information |
| `@tauri-apps/plugin-dialog` | Native dialogs |
| `@tauri-apps/plugin-clipboard-manager` | Clipboard operations |
| `@tauri-apps/plugin-updater` | Automatic updates |
| `@tauri-apps/plugin-process` | Process management |

#### Project Structure

```text
src/
├── renderer/           # React frontend (desktop and browser client)
│   ├── components/     # UI components
│   ├── hooks/          # Custom React hooks
│   ├── layouts/        # Workspace shell and its hooks
│   ├── lib/            # Runtime adapters & desktop integration
│   ├── pages/          # Page components
│   └── stores/         # Zustand stores
├── shared/             # Runtime-neutral contracts (types shared by renderer and server)
src-tauri/              # Rust backend, config & bundling
│   ├── src/commands/   # Tauri commands
│   ├── src/pty/        # PTY manager (local and adopted remote shells)
│   ├── src/ssh/        # Native SSH and SFTP
│   ├── src/acp/        # Agent Client Protocol host
│   └── src/web/        # HTTP/WebSocket server for the browser client
docs/                   # Design and architecture notes
```

#### Platform Adapters

The renderer uses an adapter/service layer to keep desktop integrations isolated from UI code, and
to serve the same UI from the desktop app, the standalone `itzterm-server`, and the browser client:

```text
src/renderer/lib/
├── ipc/              # Canonical invokeIpc / serverBase helpers
├── tauri-*.ts        # Tauri-native integrations
├── *-api.ts          # Facades that pick the Tauri or HTTP transport
├── *.ts              # Runtime-safe helpers
└── __tests__/        # Regression & parity coverage
```

## 🛠️ Development

```bash
bun run dev              # Development mode with hot reload
bun run test             # Run tests
bun run test:quick       # Run tests capped at 2 workers (lighter on laptop)
bun run test:watch       # Tests in watch mode
bun run typecheck        # Type checking
bun run lint             # Linting
bun run tauri <command>  # Direct Tauri CLI access
```

Hooks keep the heavy work off your machine: `pre-commit` lints the staged files and typechecks
the renderer, `pre-push` runs all three tsconfigs and the test suite capped at two workers. Every
push to `main` is then verified in CI.

## 🚀 Releases

Pushing a tag `v*` builds all three platforms and publishes the GitHub release, changelog and updater manifest automatically:

```bash
git tag v1.2.0           # version must match package.json and tauri.conf.json
git push origin v1.2.0
```

- CI runs the Windows (MSI + NSIS), Linux (AppImage/deb/rpm) and macOS (dmg) builds, then creates the Release with all bundles, signatures and `latest.json` for the auto-updater.
- Release notes come from `git cliff` over conventional commits since the previous tag (fallback: full git log).
- `CHANGELOG.md` is pinned to the released version by the workflow.
- The updater signing key lives in the `TAURI_SIGNING_PRIVATE_KEY` repository secret; locally set the same variable (key content) to build updater artifacts.

## SSH Development Notes

Design and decisions: [`docs/ssh-native.md`](docs/ssh-native.md).

- SSH runs natively on [`russh`](https://crates.io/crates/russh) (a Rust SSH library; its crypto comes from `aws-lc-rs`, which the app already builds): no system `ssh`, OpenSSL or Perl is needed on any platform. Each session tab owns one connection; the shell, the SFTP browser and port forwards are channels on it.
- The remote shell is adopted by `PtyManager` as a regular terminal (`pty/manager/external.rs`), so xterm, scrollback replay, claims, resize and kill work as for local terminals.
- SSH passwords and key passphrases are stored through the OS keychain, never in `ssh-profiles.json`. A secret that worked stays in process memory only while a session to that host is open (so a second tab does not ask again); SSH agent authentication avoids it entirely.
- Host keys are never accepted silently: a first-seen or changed key is shown with its SHA256 fingerprint and must be confirmed. Trust is written to `~/.ssh/known_hosts_itzterm`; `~/.ssh/known_hosts` is read but never modified.
- Local port forwarding uses `direct-tcpip` channels on the session's connection; remote/reverse forwarding is not supported yet.
- Real-server tests are opt-in: see `ssh::connect::tests::real_server_auth_matrix` and `ssh::session::tests::real_server_shell_round_trip` (`cargo test --lib -- --ignored real_server` with the `ITZTERM_SSH_TEST_*` variables).

## ⭐ Star History

[![Star History Chart](https://api.star-history.com/chart?repos=latiefahmad/itzTerm&type=date&legend=top-left)](https://www.star-history.com/?type=date&repos=latiefahmad%2FitzTerm)

## 🤝 Contributing

Contributions are welcome! Please read the [Contributing Guide](CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.

## 📄 License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Windows Terminal](https://github.com/microsoft/terminal) — Tab bar UX inspiration
- [Hyper](https://github.com/vercel/hyper) — Extensible terminal design inspiration
- [xterm.js](https://github.com/xtermjs/xterm.js) — Terminal rendering
- [shadcn/ui](https://ui.shadcn.com/) — UI components
- [Tauri](https://tauri.app/) — Desktop app framework
- [FigCraft](https://github.com/storytold/figcraft) — Canvas based on FigCraft by the ArtCraft team (MIT OR Apache-2.0)
- [Inter](https://github.com/rsms/inter) — Font embedded in the canvas, Copyright (c) 2016 The Inter Project Authors, SIL Open Font License 1.1 ([licence](canvas/licenses/OFL-inter.txt))

---

<div align="center">

Built with ❤️ by [latiefahmad](https://github.com/latiefahmad) — forked from Termul by [gnoviawan](https://github.com/gnoviawan)

</div>
