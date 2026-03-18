# Architecture Documentation

> **WhatsApp Web Clone** — Full-stack real-time messaging client  
> Rust (Axum) backend + Next.js (React / Tailwind CSS) frontend

---

## Table of Contents

1. [High-Level Overview](#high-level-overview)
2. [Technology Stack](#technology-stack)
3. [Project Structure](#project-structure)
4. [Backend Architecture](#backend-architecture)
   - [Module Map](#module-map)
   - [Application State](#application-state)
   - [Data Models](#data-models)
   - [API Endpoints](#api-endpoints)
   - [Event Pipeline](#event-pipeline)
   - [Media Pipeline](#media-pipeline)
   - [Contact / Group Resolution](#contact--group-resolution)
5. [Frontend Architecture](#frontend-architecture)
   - [Module Map](#frontend-module-map)
   - [Component Tree](#component-tree)
   - [State Management](#state-management)
   - [Normalizers](#normalizers)
   - [API Polling Strategy](#api-polling-strategy)
6. [Data Flow](#data-flow)
7. [Development Guide](#development-guide)
8. [Testing](#testing)

---

## High-Level Overview

```
┌─────────────────────┐  HTTP/JSON   ┌──────────────────────────────┐
│  Next.js Frontend   │◄────────────►│   Axum REST API (Rust)       │
│  (React + Tailwind) │  :3030→:8080 │   + WhatsApp protocol client │
└─────────────────────┘              └──────────┬───────────────────┘
                                                │
                                     WebSocket (E2EE)
                                                │
                                     ┌──────────▼───────────────────┐
                                     │   WhatsApp Servers           │
                                     │   (Multi-Device Protocol)    │
                                     └──────────────────────────────┘
```

The backend maintains a persistent WebSocket connection to WhatsApp's servers via the
[`whatsapp-rust`](https://github.com/jlucaso1/whatsapp-rust) library. Incoming events
(messages, receipts, presence updates, group changes, etc.) are dispatched through an event
handler that populates an in-memory store. The frontend polls the REST API every 3 seconds
and renders a WhatsApp Web–style UI.

---

## Technology Stack

| Layer | Technology | Version |
|-------|-----------|---------|
| **Runtime** | Rust (Tokio) | Edition 2021 |
| **HTTP framework** | Axum | 0.7 |
| **WhatsApp protocol** | whatsapp-rust (git) | latest main |
| **Session persistence** | SQLite (via `whatsapp-rust`) | — |
| **Frontend framework** | Next.js (App Router) | 14.2.15 |
| **UI library** | React | 18.3.1 |
| **Styling** | Tailwind CSS | 3.4.14 |
| **QR rendering** | react-qr-code | 2.0.15 |
| **E2E testing** | Playwright | 1.48+ |
| **Serialization** | serde / serde_json | 1.x |
| **Base64** | base64 | 0.22 |
| **CORS** | tower-http | 0.5 |

---

## Project Structure

```
whatsapp-rust-web/
├── backend/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs           Entry point, router, tests          (316 lines)
│       ├── models.rs         Types, DTOs, constants, AppState    (210 lines)
│       ├── helpers.rs        Pure functions (no async/state)      (467 lines)
│       ├── store.rs          In-memory DataStore impl            (251 lines)
│       ├── handlers.rs       11 HTTP handler functions            (453 lines)
│       ├── sync.rs           Async contact/group resolution       (218 lines)
│       └── events.rs         WhatsApp event dispatcher            (289 lines)
│
├── frontend/
│   ├── package.json
│   ├── next.config.mjs       API proxy (/api/* → localhost:8080)
│   ├── tailwind.config.ts
│   ├── app/
│   │   ├── layout.tsx         Root layout
│   │   ├── page.tsx           Main Home component                 (761 lines)
│   │   ├── lib/
│   │   │   ├── types.ts       Shared TypeScript interfaces        (69 lines)
│   │   │   ├── helpers.ts     Pure utility functions               (166 lines)
│   │   │   └── normalizers.ts Defensive API normalizers            (87 lines)
│   │   └── components/
│   │       ├── Icons.tsx      6 SVG icon components                (57 lines)
│   │       └── MessageMedia.tsx  Rich media preview component      (92 lines)
│   ├── tests/
│   │   └── whatsapp.spec.ts  Playwright E2E tests
│   └── playwright.config.ts
│
├── run.sh                    One-command startup script
├── README.md
└── LICENSE
```

---

## Backend Architecture

### Module Map

```
main.rs
  ├── mod models     ← types, enums, AppState, DTOs
  ├── mod helpers    ← pure functions (no async)
  ├── mod store      ← DataStore impl block
  ├── mod handlers   ← Axum HTTP handler functions
  ├── mod sync       ← async contact/group resolution
  └── mod events     ← WhatsApp event dispatcher
```

All public types from `models.rs` are re-exported at the crate root via `pub use models::*`,
so tests and other modules can reference them directly.

**Dependency graph (modules import from):**

```
models   ← (standalone — no crate imports)
helpers  ← models
store    ← models, helpers
sync     ← models, helpers
handlers ← models, helpers, sync
events   ← models, helpers, sync
main     ← models, handlers, events
```

### Application State

`AppState` (defined in `models.rs`) is the single shared state object passed to all Axum
handlers and event callbacks:

```rust
pub struct AppState {
    pub qr_code:      Arc<RwLock<Option<String>>>,   // Current QR code for pairing
    pub is_connected:  Arc<AtomicBool>,               // WhatsApp connection status
    pub is_syncing:    Arc<AtomicBool>,               // Startup sync in progress
    pub client:        Arc<RwLock<Option<Arc<Client>>>>, // WhatsApp protocol client
    pub store:         Arc<RwLock<DataStore>>,         // In-memory chat/contact/media store
    pub db_path:       Arc<String>,                    // SQLite database path
}
```

All fields are `Arc`-wrapped for cheap cloning. Mutable state uses `RwLock` (from Tokio) or
`AtomicBool` for lock-free flags. `AppState` implements `Clone`.

### Data Models

**Core domain models** (all in `models.rs`):

| Struct | Purpose |
|--------|---------|
| `DataStore` | Top-level in-memory store: `HashMap<String, ChatRecord>` for chats, `HashMap<String, ContactSummary>` for contacts, `HashMap<String, MediaBlob>` for downloadable media |
| `ChatRecord` | Container holding a `ChatSummary` + `Vec<ChatMessage>` |
| `ChatSummary` | Sidebar-level metadata: JID, name, phone, preview, unread count, typing indicator, presence |
| `ChatMessage` | Single message: id, sender, text, timestamp, media attachment, mentions, receipt status |
| `ContactSummary` | Contact entry: JID, name, phone, status, avatar URL, business/registered flags |
| `MediaAttachment` | Serializable media metadata sent to the frontend (kind, MIME, dimensions, download path, preview) |
| `MediaBlob` | Server-side download parameters (direct path, media key, SHA256 hashes, file length, media type) — never serialized to the client |
| `MentionSummary` | Resolved mention: JID + display name |

**Constants:**

| Constant | Value | Purpose |
|----------|-------|---------|
| `MAX_MESSAGES_PER_CHAT` | 200 | Oldest messages are evicted when this limit is exceeded |
| `STARTUP_SYNC_TIMEOUT_SECS` | 45 | Maximum wait for WhatsApp's initial history sync |

**API DTOs** (request/response types):

| DTO | Direction | Description |
|-----|-----------|-------------|
| `QrResponse` | → client | QR code string + connection flag |
| `StatusResponse` | → client | Connection + sync flags |
| `BootstrapResponse` | → client | Full initial payload (QR, flags, chats, contacts) |
| `MessagesResponse` | → client | Messages for a single chat |
| `ChatsResponse` | → client | Sorted chat summaries |
| `ContactsResponse` | → client | Sorted contact list |
| `SendMessageRequest` | ← client | phone/jid, message text, mention JIDs |
| `TypingRequest` | ← client | Typing state string (`composing` / `recording` / `paused`) |
| `SendMessageResponse` | → client | Success flag + message ID |
| `LogoutResponse` | → client | Success flag + message |
| `ErrorResponse` | → client | Error description string |

### API Endpoints

All endpoints are prefixed with `/api` and proxied from the frontend via Next.js rewrites.

| Method | Path | Handler | Description | `whatsapp-rust` |
|--------|------|---------|-------------|:---------------:|
| `GET` | `/api/auth/qr` | `get_qr` | Returns current QR code + connection status | — |
| `GET` | `/api/auth/status` | `get_status` | Returns `is_connected` + `is_syncing` flags | — |
| `POST` | `/api/auth/logout` | `logout` | Disconnects session, deletes SQLite DB, resets store | ✅ `client.disconnect()` |
| `GET` | `/api/bootstrap` | `get_bootstrap` | Full initial payload for the UI | — |
| `GET` | `/api/chats` | `get_chats` | Sorted chat summaries | — |
| `GET` | `/api/contacts` | `get_contacts` | Sorted contact list | — |
| `GET` | `/api/chats/:jid/messages` | `get_chat_messages` | Messages for a specific chat (404 if unknown) | — |
| `POST` | `/api/chats/:jid/read` | `mark_chat_read` | Locally zeroes unread count | — |
| `POST` | `/api/chats/:jid/typing` | `update_typing` | Forwards typing state to WhatsApp | ✅ `client.chatstate()` |
| `POST` | `/api/messages/send` | `send_message` | Sends a text message (with optional mentions) | ✅ `client.send_message()` |
| `GET` | `/api/media/:chat_jid/:message_id` | `get_media` | Downloads + proxies media from WhatsApp servers | ✅ `client.download_from_params()` |

#### Send Message Flow

1. Validates `message` is non-empty.
2. Resolves target JID:
   - If `jid` is provided and is a group → use directly.
   - If `jid` is a LID (Linked Identity) → looks up phone from contacts store or `client.get_phone_number_from_lid()`.
   - If only `phone` is provided → constructs `phone@s.whatsapp.net`.
3. Constructs `wa::Message` (with `ExtendedTextMessage` + `ContextInfo` if mentions are present).
4. Calls `client.send_message()`.
5. On success, records the outgoing message in the store and returns the message ID.

### Event Pipeline

The `events.rs` module contains the central `handle_event()` dispatcher, called from the
bot's `on_event` closure in `main()`. It matches on ~20 `Event` variants:

```
Event::PairingQrCode     → stores QR code, sets is_connected=false
Event::Connected          → sets is_connected=true, spawns startup sync
Event::Disconnected       → sets is_connected=false
Event::Message            → handle_incoming_message() [see below]
Event::Receipt            → promotes receipt status (sent→delivered→read→played)
Event::ChatPresence       → updates typing indicator per chat
Event::Presence           → updates online/last-seen per contact
Event::JoinedGroup        → creates chat record
Event::PushNameUpdate     → renames contact
Event::ContactUpdated     → spawns contact profile refresh
Event::ContactNumberChanged → migrates chat/contact to new JID
Event::ContactSyncRequested → refreshes all known contacts
Event::PictureUpdate      → refreshes avatar or clears it
Event::UserAboutUpdate    → updates contact status text
Event::GroupUpdate        → refreshes group metadata
Event::ArchiveUpdate      → marks chat as archived
Event::MuteUpdate         → marks chat as muted
Event::MarkChatAsReadUpdate → zeroes unread count
Event::OfflineSyncCompleted → clears is_syncing flag
```

#### Incoming Message Processing (`handle_incoming_message`)

1. Resolves chat JID and sender JID (handles LID → phone mapping).
2. Extracts push name or falls back to stored display name.
3. Extracts text from the proto message (conversation, extended text, captions, etc.).
4. Extracts mention JIDs from `ContextInfo`.
5. Resolves mention display names.
6. Extracts media attachment + blob if present.
7. Upserts contact record.
8. Records message in the store (with media blob for later download).
9. Spawns background tasks for profile refresh and presence subscription.

### Media Pipeline

Media is handled in a two-phase flow:

1. **Ingest** (on message receive): `extract_media()` in `helpers.rs` inspects the proto
   message for image/video/document/audio/sticker fields. It builds a `MediaAttachment`
   (frontend-facing metadata) and a `MediaBlob` (server-side download parameters). The blob
   is keyed by `"{chat_jid}:{message_id}"` in the store.

2. **Download** (on frontend request): `GET /api/media/:chat_jid/:message_id` looks up the
   `MediaBlob`, calls `client.download_from_params()` with the stored crypto parameters,
   and streams the decrypted bytes back with the correct `Content-Type` and
   `Content-Disposition` headers.

**Document preview images**: For documents, the WhatsApp proto may include a JPEG thumbnail.
This is base64-encoded into a `data:image/jpeg;base64,...` URL and stored in
`MediaAttachment.preview_image_url` for inline rendering.

### Contact / Group Resolution

The `sync.rs` module provides async functions for resolving contact identity across
WhatsApp's dual JID system (phone-based `@s.whatsapp.net` and Linked Identity `@lid`):

| Function | Purpose |
|----------|---------|
| `resolve_phone_for_jid` | Maps a LID to a phone number via `client.get_phone_number_from_lid()`, or extracts phone from a standard JID |
| `resolved_display_name_for_store` | Gets the best current name from the store, falling back to phone/JID |
| `resolve_mention_summaries` | Builds `Vec<MentionSummary>` with resolved names for a list of mention JIDs |
| `refresh_contact_profile` | Full profile refresh: queries contact info, user info, profile picture, and propagates name/phone/avatar across all alias JIDs |
| `refresh_group_metadata` | Fetches group subject and participant list, spawns profile refreshes for each member |
| `refresh_all_known_contacts` | Iterates all known JIDs and refreshes each one |
| `wait_for_startup_sync` | Blocks until WhatsApp's initial sync completes (max 45s), then refreshes all contacts |
| `ensure_presence_subscription` | Subscribes to presence updates for non-group JIDs |

---

## Frontend Architecture

### Frontend Module Map

```
app/
├── page.tsx                Main Home component (state, effects, render)
├── lib/
│   ├── types.ts            Shared interfaces (ApiMessage, ApiChat, ApiContact, BootstrapResponse)
│   ├── helpers.ts          ~30 pure utility functions
│   └── normalizers.ts      Defensive normalizers for API responses
└── components/
    ├── Icons.tsx            6 SVG icon components
    └── MessageMedia.tsx     Rich media preview component
```

**Import dependency graph:**

```
types.ts          ← (standalone)
helpers.ts        ← types
normalizers.ts    ← types, helpers
Icons.tsx         ← (standalone)
MessageMedia.tsx  ← types, helpers
page.tsx          ← types, helpers, normalizers, Icons, MessageMedia
```

### Component Tree

```
<Home>                          (page.tsx — main component)
├── QR Code Screen              (when not connected)
│   └── <QRCode />              (from react-qr-code)
├── Sidebar
│   ├── Header (WhatsApp logo, search, new chat, contacts, logout)
│   ├── Search / New Chat Input
│   ├── Chat List               (filtered, sorted by timestamp)
│   │   └── Chat Row            (avatar, name, preview, unread badge, time)
│   └── Contact List            (when sidebar view = "contacts")
│       └── Contact Row         (avatar, name, phone)
├── Chat View
│   ├── Chat Header             (name, presence, avatar)
│   ├── Message List
│   │   ├── Day Separator
│   │   ├── Message Bubble
│   │   │   ├── <MessageMedia>  (image / video / audio / document card)
│   │   │   ├── Text with @mentions
│   │   │   ├── Timestamp
│   │   │   └── Receipt Icon    (✓ / ✓✓ / colored ✓✓)
│   │   └── ...
│   └── Message Input
│       ├── Mention Dropdown    (@ autocomplete)
│       └── Send Button
└── Empty State                 (when no chat selected)
```

### State Management

The `Home` component manages all state via React hooks:

| Hook | Type | Purpose |
|------|------|---------|
| `qrCode` | `useState<string \| null>` | Current QR code for pairing |
| `isConnected` | `useState<boolean>` | WhatsApp connection status |
| `isSyncing` | `useState<boolean>` | Startup sync in progress |
| `chats` | `useState<ApiChat[]>` | All chat summaries |
| `contacts` | `useState<ApiContact[]>` | All contacts |
| `selectedChat` | `useState<ApiChat \| null>` | Currently open chat |
| `messages` | `useState<ApiMessage[]>` | Messages for the selected chat |
| `draft` | `useState<string>` | Message input text |
| `search` | `useState<string>` | Sidebar search query |
| `sidebarView` | `useState<SidebarView>` | `"chats"` or `"contacts"` |
| `newChatPhone` | `useState<string>` | Phone number for new chat dialog |
| `showNewChat` | `useState<boolean>` | New chat dialog open |
| `mentionQuery` | `useState<string \| null>` | Active @mention autocomplete query |

Key `useMemo` computations:
- **filteredChats**: Chats filtered by search query and sorted by timestamp.
- **filteredContacts**: Contacts filtered by search query.
- **availableMentions**: Group members available for @mention (contacts matching mention query).

Key `useEffect` hooks:
- **Bootstrap**: Fetches `/api/bootstrap` on mount.
- **Polling**: Refreshes chats/contacts every 3 seconds when connected.
- **Message loading**: Fetches messages when `selectedChat` changes.

### Normalizers

The `normalizers.ts` module provides defensive functions that ensure API responses always
have safe defaults, preventing `undefined` / `null` access errors in the UI:

| Function | Input | Guarantees |
|----------|-------|------------|
| `normalizeMessage` | `Partial<ApiMessage>` | All fields populated with defaults; media sub-object fully normalized |
| `normalizeChat` | `Partial<ApiChat>` | Derives phone from JID if missing; defaults name to phone or "Unknown" |
| `normalizeContact` | `Partial<ApiContact>` | Derives phone from JID; defaults is_registered to true |
| `normalizeBootstrap` | `Partial<BootstrapResponse>` | Normalizes nested arrays of chats and contacts |

### API Polling Strategy

```
Mount → GET /api/bootstrap
         │
         ▼
    setInterval(3000ms)
         │
         ├── GET /api/bootstrap     (refreshes chats, contacts, connection status)
         │
         └── If selectedChat:
               GET /api/chats/:jid/messages  (refreshes message list)
```

All API calls go through the Next.js development proxy which rewrites `/api/*` to
`http://localhost:8080/api/*` (configured in `next.config.mjs`).

---

## Data Flow

### Incoming Message

```
WhatsApp Servers
    │ WebSocket (encrypted)
    ▼
whatsapp-rust client
    │ Event::Message
    ▼
events::handle_event()
    │
    ▼
events::handle_incoming_message()
    ├── sync::resolve_phone_for_jid()       ← LID → phone mapping
    ├── helpers::extract_text()             ← proto → plain text
    ├── helpers::extract_media()            ← proto → MediaAttachment + MediaBlob
    ├── sync::resolve_mention_summaries()   ← JID → display name
    ├── store.upsert_contact()              ← update contact record
    ├── store.record_message()              ← store message + media blob
    └── tokio::spawn(refresh_contact_profile)  ← async profile update
```

### Outgoing Message

```
User types message + clicks Send
    │
    ▼
Frontend: POST /api/messages/send { jid, phone, message, mentions }
    │
    ▼
handlers::send_message()
    ├── Resolve target JID (phone/JID/LID)
    ├── Build wa::Message (with optional ContextInfo for mentions)
    ├── client.send_message()
    ├── store.record_message()  ← optimistic local record
    └── Return { success, message_id }
    │
    ▼
Frontend updates on next poll cycle (3s)
```

### Media Download

```
Frontend: <img src="/api/media/{chat_jid}/{msg_id}" />
    │
    ▼
handlers::get_media()
    ├── store.media_for(chat_jid, msg_id)   ← lookup MediaBlob
    ├── client.download_from_params(...)      ← decrypt + download from WA servers
    └── Return bytes with Content-Type header
```

---

## Development Guide

### Prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| Rust | 1.85+ | Install via [rustup](https://rustup.rs) |
| Node.js | 18+ | LTS recommended |
| npm | 9+ | Comes with Node.js |
| libsqlite3-dev | — | Linux only: `sudo apt install libsqlite3-dev pkg-config` |

### Quick Start

```bash
# Start both backend (:8080) and frontend (:3030)
./run.sh

# Then open http://localhost:3030 and scan the QR code
```

### Manual Start

```bash
# Terminal 1 — Backend
cd backend
PORT=8080 cargo run --release

# Terminal 2 — Frontend
cd frontend
npm install
npm run dev -- -p 3030
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `8080` | Backend HTTP port |
| `RUST_LOG` | `info` | Log level (env_logger format) |

### Adding a New Backend Module

1. Create `backend/src/yourmodule.rs`.
2. Add `mod yourmodule;` in `main.rs`.
3. Import from other modules with `use crate::models::*;`, `use crate::helpers::*;`, etc.

### Adding a New Frontend Component

1. Create `frontend/app/components/YourComponent.tsx`.
2. Import types from `../lib/types` and helpers from `../lib/helpers`.
3. Import in `page.tsx` or other components.

---

## Testing

### Backend Unit Tests (12 tests)

```bash
cd backend && cargo test
```

Tests are located in `main.rs` under `#[cfg(test)] mod tests`. They use Axum's `tower::ServiceExt::oneshot()` to test HTTP handlers in isolation without starting a server or WhatsApp client.

| Test | Validates |
|------|-----------|
| `qr_returns_code_when_pairing` | QR endpoint returns stored code when not connected |
| `bootstrap_returns_empty_lists` | Bootstrap returns empty chats/contacts on fresh state |
| `status_reports_disconnected` | Status endpoint reflects disconnected state |
| `send_rejects_empty_target` | 400 when no phone/jid provided |
| `send_rejects_empty_message` | 400 when message is whitespace |
| `send_rejects_malformed_json` | 400 on invalid JSON body |
| `send_rejects_missing_fields` | 422 when required fields missing |
| `send_returns_503_when_not_connected` | 503 when WhatsApp not connected |
| `send_returns_503_when_client_missing` | 503 when client is None |
| `send_rejects_non_digit_phone` | 400 when phone contains non-digit characters |
| `send_accepts_jid_shape` | JID-format targets are accepted (returns 503 due to no client) |
| `messages_endpoint_returns_404_for_unknown_chat` | 404 for non-existent chat |

### Frontend E2E Tests

```bash
cd frontend
npx playwright install chromium
npx playwright test
```

Tests are located in `frontend/tests/whatsapp.spec.ts` and use Playwright with Chromium.

---

*Last updated: March 2026*
