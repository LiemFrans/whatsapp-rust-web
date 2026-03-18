# WhatsApp Web Clone

A full-stack WhatsApp Web clone built with **Rust (Axum)** for the backend and **Next.js + Tailwind CSS** for the frontend.

Uses [`whatsapp-rust`](https://github.com/jlucaso1/whatsapp-rust) for WhatsApp protocol handling, QR code pairing, and end-to-end encrypted messaging.

## Features

- **Real-time messaging** — send and receive text messages with read receipts, typing indicators, and presence updates
- **Media support** — view incoming images, videos, audio, documents, and stickers inline
- **Emoji picker** — WhatsApp Web-style picker with search, categories, and skin-tone selection
- **GIF picker** — search and send GIFs via the Tenor API (sent as looping video messages)
- **Sticker picker** — browse and re-send stickers collected from received messages
- **Media sending** — send images, GIFs, and stickers to any chat
- **SKDM warm-up** — automatic Sender Key Distribution Message pre-warming for group chats to prevent retry loops
- **@Mentions** — type `@` to autocomplete group member mentions
- **Contact & group sync** — automatic contact resolution, group metadata, and profile pictures
- **QR code pairing** — scan to link your WhatsApp account (multi-device protocol)

## Architecture

```
whatsapp-rust-web/
├── backend/                      Rust (Axum) REST API
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs               Entry point, router, tests
│       ├── models.rs             Types, DTOs, AppState
│       ├── helpers.rs            Pure utility functions
│       ├── store.rs              In-memory DataStore
│       ├── handlers.rs           HTTP handler functions
│       ├── sync.rs               Contact/group resolution
│       └── events.rs             WhatsApp event dispatcher
│
├── frontend/                     Next.js 14 + Tailwind CSS
│   ├── app/
│   │   ├── page.tsx              Main Home component
│   │   ├── lib/
│   │   │   ├── types.ts          Shared TypeScript interfaces
│   │   │   ├── helpers.ts        Pure utility functions
│   │   │   ├── normalizers.ts    Defensive API normalizers
│   │   │   └── emoji-data.ts     Comprehensive emoji dataset
│   │   └── components/
│   │       ├── Icons.tsx          SVG icon components
│   │       ├── MessageMedia.tsx   Rich media preview
│   │       ├── EmojiPicker.tsx    WhatsApp-style emoji picker
│   │       ├── GifPicker.tsx      Tenor GIF search + send
│   │       ├── StickerPicker.tsx  Sticker grid from received messages
│   │       └── MediaPicker.tsx    Tabbed container (Emoji/GIF/Sticker)
│   ├── tests/                    Playwright E2E tests
│   └── playwright.config.ts
│
├── run.sh                        One-command startup script
├── Makefile                      Build / test / clean targets
└── ARCHITECTURE.md               Detailed architecture docs
```

## Prerequisites

| Tool | Version |
|------|---------|
| Rust | 1.85+ (for `edition = "2024"` dependency) |
| Node.js | 18+ |
| npm | 9+ |
| libsqlite3-dev | (Linux only: `sudo apt install libsqlite3-dev pkg-config`) |

## Quick Start

```bash
# One-command startup (installs deps, builds, starts both servers)
./run.sh

# — OR use Make —
make install   # Install all dependencies
make dev       # Start backend (port 8080) + frontend (port 3030)
```

Then open **http://localhost:3030** and scan the QR code with your WhatsApp mobile app.

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/auth/qr` | Returns the current QR code string + connection status |
| `GET` | `/api/auth/status` | Returns `{ is_connected: bool }` |
| `POST` | `/api/auth/logout` | Disconnects session, deletes DB, resets state |
| `GET` | `/api/bootstrap` | Full initial payload (QR, flags, chats, contacts) |
| `GET` | `/api/chats` | Sorted chat summaries |
| `GET` | `/api/contacts` | Sorted contact list |
| `GET` | `/api/chats/:jid/messages` | Messages for a specific chat |
| `POST` | `/api/chats/:jid/read` | Mark a chat as read |
| `POST` | `/api/chats/:jid/typing` | Forward typing state to WhatsApp |
| `POST` | `/api/messages/send` | Send a text message (with optional mentions) |
| `POST` | `/api/messages/send-media` | Send media: `{ "jid", "phone", "url", "media_type", "caption", "width", "height" }` |
| `GET` | `/api/media/:chat_jid/:msg_id` | Download + proxy media from WhatsApp servers |
| `GET` | `/api/stickers` | Returns recently received stickers |

## Running Tests

```bash
# Backend unit tests (12 tests)
make test-backend
# or: cd backend && cargo test

# Frontend E2E tests (4 tests, requires Chromium)
make test-e2e
# or: cd frontend && npx playwright install chromium && npx playwright test

# All tests
make test
```

## Tech Stack

- **Backend:** Rust, Axum 0.7, `whatsapp-rust` (WhatsApp Web protocol), SQLite (session persistence), ureq (HTTP client for media fetching)
- **Frontend:** Next.js 14, React 18, Tailwind CSS 3, `react-qr-code`, Tenor API v2 (GIF search)
- **Testing:** Rust native `#[test]` (backend), Playwright (frontend E2E)

## Disclaimer

This is an unofficial, open-source project. Using custom WhatsApp clients may violate Meta's Terms of Service and could result in account suspension. **Use at your own risk.**