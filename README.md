# WhatsApp Web UI – Personal & Business Mode

A full-stack WhatsApp Web interface with **Personal Mode** (1-on-1 messaging) and **Business Mode** (multi-agent customer support) built with Rust + React.

## Architecture

| Layer | Stack |
|-------|-------|
| **Frontend** | React 19 · TypeScript 5.7 · Vite 6 · Tailwind CSS 3.4 · Zustand 5 |
| **Backend** | Rust (Axum 0.7) · SQLx 0.8 · JWT · Argon2 · WebSockets |
| **Database** | PostgreSQL 16 |
| **WhatsApp** | whatsapp-rust v0.5 *(stubbed – see note below)* |

## Features

### Personal Mode
- QR code pairing & multi-session management
- Real-time messaging via WebSocket
- Chat list with search, unread badges, media previews
- Message bubbles with status ticks, reply, star, and media types

### Business Mode
- Unassigned chat queue with round-robin or manual assignment
- Agent dashboard with "My Chats" view
- Ticket system (create, assign, escalate, status workflow)
- Quick replies with `/shortcut` trigger
- Analytics panel (messages today, queue size, per-agent breakdown)
- Role-based access (Admin / Agent / User)

## Project Structure

```
whatsapp-rust-web/
├── frontend/              # React + TypeScript + Vite
│   ├── src/
│   │   ├── components/    # ChatList, ChatBubble, MessageInput, Sidebar, QRCodeModal
│   │   ├── pages/         # Login, ModeSelect, PersonalMode, BusinessMode
│   │   ├── store/         # Zustand stores (auth, chat, business)
│   │   ├── hooks/         # useAuth, useWebSocket
│   │   ├── services/      # Axios API client, WebSocket service
│   │   └── types/         # Shared TypeScript types
│   └── ...
├── backend/               # Rust Axum server
│   └── src/
│       ├── api/           # REST routes (auth, chats, business, whatsapp)
│       ├── auth/          # JWT tokens, Argon2 passwords, middleware extractors
│       ├── models/        # SQLx models (user, session, chat, message, ticket, etc.)
│       ├── services/      # Business logic (assignment, ticket, escalation, audit)
│       ├── websocket/     # Broadcast hub with per-user & per-role channels
│       └── whatsapp/      # WhatsApp session manager & event handler
├── database/
│   └── migrations/        # PostgreSQL schema (001_initial_schema.sql)
├── docker-compose.yml     # PostgreSQL + backend + frontend
└── .env.example
```

## Quick Start

### Prerequisites
- **Rust** ≥ 1.75 (`rustup update stable`)
- **Node.js** ≥ 18 + npm
- **Docker** & Docker Compose (for PostgreSQL)

### 1. Clone & configure

```bash
cp .env.example .env
# Edit .env: set DATABASE_URL, JWT_SECRET, etc.
```

### 2. Start PostgreSQL

```bash
docker compose up -d postgres
```

### 3. Run the backend

```bash
cd backend
cargo run
# Server starts on http://localhost:3001
```

The backend auto-runs migrations and seeds an admin user (`admin` / `admin123`).

### 4. Run the frontend

```bash
cd frontend
npm install
npm run dev
# Frontend starts on http://localhost:5173
```

### 5. Login

Open `http://localhost:5173`, log in with `admin` / `admin123`, and select Personal or Business mode.

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/auth/login` | Authenticate, receive JWT |
| POST | `/api/auth/register` | Create user (admin-only for agent/admin roles) |
| POST | `/api/auth/refresh` | Rotate refresh token |
| GET | `/api/auth/me` | Current user info |
| GET | `/api/whatsapp/sessions` | List WhatsApp sessions |
| POST | `/api/whatsapp/sessions` | Connect a new session |
| DELETE | `/api/whatsapp/sessions/:id` | Disconnect session |
| GET | `/api/chats` | List chats (filterable) |
| GET | `/api/chats/:id/messages` | Paginated messages |
| POST | `/api/chats/:id/messages` | Send a message |
| GET | `/api/business/queue` | Unassigned chat queue |
| POST | `/api/business/take` | Agent takes a chat |
| POST | `/api/business/transfer` | Transfer chat to another agent |
| GET | `/api/business/tickets` | List tickets |
| POST | `/api/business/tickets` | Create ticket |
| GET | `/api/business/analytics` | Dashboard analytics |
| GET | `/api/business/quick-replies` | Quick reply templates |
| WS | `/ws?token=<JWT>` | Real-time events |

## WhatsApp Integration Note

> **whatsapp-rust v0.5** is temporarily commented out in `Cargo.toml` due to a
> native library conflict: `whatsapp-rust-sqlite-storage` needs `libsqlite3-sys
> v0.35` while `sqlx-sqlite` (pulled in by feature unification) needs `v0.28/0.30`.
>
> The backend runs with a **simulated WhatsApp manager** that emits a mock QR code
> and auto-connects after 5 seconds. All other features (auth, chat CRUD, business
> logic, WebSocket broadcast) work fully.
>
> Once the upstream conflict is resolved, uncomment the dependency and replace the
> stub manager in `src/whatsapp/manager.rs` with the real `Bot::run()` integration.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | – | PostgreSQL connection string |
| `JWT_SECRET` | – | Secret for signing JWTs |
| `PORT` | `3001` | Backend server port |
| `CORS_ORIGINS` | `http://localhost:5173` | Allowed CORS origins |
| `SESSION_DB_DIR` | `./data/sessions` | Directory for WhatsApp session DBs |
| `SYNC_DAYS_LIMIT` | `30` | Max days for history sync |

## License

MIT
