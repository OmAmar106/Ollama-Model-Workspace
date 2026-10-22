# Ollama Model Workspace

A lightweight Linux desktop client for [Ollama](https://ollama.com), built with Tauri.

Multi-model conversations, cross-model context switching, parallel queries, and configurable thinking modes, all stored locally.

## Overview

Ollama Model Workspace is a native desktop interface for chatting with locally hosted Ollama models. It focuses on working with **multiple models at once**: switch models mid-conversation, branch off any message into a focused sub-chat, and let background sessions keep generating while you continue working elsewhere.

Everything is persisted locally in SQLite — no accounts, no telemetry, no cloud.

## Screenshots



## Features

- **Multi-model conversations** — pick a model per chat and switch models without leaving the conversation.
- **Parallel chats** — every session tracks its own generation stream, so background chats can finish while you work in another one.
- **Anchored branch sub-chats** — branch from any message and explore an alternative path in an inline side panel, without disturbing the main thread.
- **Thinking modes** — toggle reasoning output for models that support it, with a collapsible "Thinking Process" section.
- **Per-chat system instructions** — give each conversation its own persona or context.
- **Session management** — search, rename, delete, and browse chats grouped by Today, Yesterday, Last Week, and Older.
- **Ollama health indicator** — see at a glance whether Ollama is reachable and which version is running.
- **Streaming markdown** — GitHub-flavored Markdown rendered as tokens arrive.
- **Local persistence** — sessions, messages, and generation metadata stored in SQLite, with automatic cleanup of inactive sessions after 14 days.
- **Keyboard shortcuts** — for new chats, sidebar toggling, and stopping generation.

## Requirements

- Linux (`.deb` builds are published for Ubuntu 22.04+)
- [Ollama](https://ollama.com/download) running locally (default: `http://127.0.0.1:11434`)
- At least one model pulled, e.g.:

  ```bash
  ollama pull llama3.2
  ```

## Installation

Download the latest `.deb` package from the [Releases page](https://github.com/OmAmar106/Ollama-Model-Workspace/releases), then install it:

```bash
sudo apt install ./Ollama\ Model\ Workspace_*.deb
```

Then launch **Ollama Model Workspace** from your application menu.

## Build from Source

### Prerequisites

- [Node.js](https://nodejs.org) 22+
- [Rust](https://rustup.rs) (stable, 1.90+)
- Tauri's Linux system dependencies:

  ```bash
  sudo apt-get update
  sudo apt-get install -y \
    build-essential curl wget file \
    libxdo-dev libssl-dev librsvg2-dev \
    libayatana-appindicator3-dev libwebkit2gtk-4.1-dev
  ```

### Build

```bash
git clone https://github.com/OmAmar106/Ollama-Model-Workspace.git
cd Ollama-Model-Workspace/app

npm ci
npx tauri build --bundles deb
```

The installer will be written to `app/src-tauri/target/release/bundle/deb/`.

## Development

```bash
cd app
npm ci
npx tauri dev
```

The Vite dev server runs at `http://localhost:5173`, and the Tauri window reloads on changes. Frontend-only commands:

```bash
npm run build   # type-check + production build
npm run lint    # oxlint
npm run preview # preview the built frontend
```

## Configuration

The Ollama endpoint defaults to `http://127.0.0.1:11434` and can be overridden with environment variables:

| Variable | Description |
| --- | --- |
| `OLLAMA_BASE_URL` | Full base URL, e.g. `http://192.168.1.10:11434` |
| `OLLAMA_HOST` | Host or URL; `host:port` is prefixed with `http://` |

```bash
# The installed executable is `app`; the app menu entry is "Ollama Model Workspace"
OLLAMA_BASE_URL=http://192.168.1.10:11434 app
```

Alternatively, export the variable in your shell profile or desktop launcher.

## Data & Storage

Sessions, messages, and generation records are stored in SQLite under the app data directory:

```
~/.local/share/com.dev.ollamarunnerui/ollama_workspace.db
```

Development builds use `ollama_workspace_dev.db` in the same directory.

Sessions that have been inactive for **14 days** are pruned automatically at startup and once per day. Deleting a session cascades to its messages and generation records.

## Keyboard Shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl`/`Cmd` + `N` | New chat |
| `Ctrl`/`Cmd` + `B` | Toggle sidebar |
| `Esc` | Stop generation / close drawer or branch panel |

## Tech Stack

| Layer | Technology |
| --- | --- |
| Desktop shell | [Tauri 2](https://tauri.app) |
| Frontend | [React 19](https://react.dev) + [TypeScript](https://www.typescriptlang.org) |
| Build tool | [Vite](https://vite.dev) |
| Backend | [Rust](https://www.rust-lang.org) |
| Database | SQLite via [rusqlite](https://github.com/rusqlite/rusqlite) |
| HTTP client | [reqwest](https://github.com/seanmonstar/reqwest) |
| Icons | [Lucide](https://lucide.dev) |
| Markdown | [react-markdown](https://github.com/remarkjs/react-markdown) + [remark-gfm](https://github.com/remarkjs/remark-gfm) |

## Project Structure

```
.
├── app/
│   ├── src/                     # React frontend
│   │   ├── api/                 # Tauri command wrappers and types
│   │   ├── components/          # Chat, layout, and panel components
│   │   ├── hooks/               # useChat, useModels, useOllamaHealth
│   │   └── styles/              # Theme and global styles
│   └── src-tauri/               # Rust backend
│       └── src/
│           ├── commands.rs      # Tauri commands
│           ├── db/              # SQLite schema and queries
│           ├── ollama/          # Ollama API client
│           └── generation_manager.rs
└── .github/workflows/           # Release automation
```

## Acknowledgements

- [Ollama](https://ollama.com) for making local LLMs accessible.
- [Tauri](https://tauri.app) for the lightweight desktop runtime.
- The open-source projects that power this app's frontend and backend.
