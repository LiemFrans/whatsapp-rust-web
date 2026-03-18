# WhatsApp Web Clone

A full-stack WhatsApp Web clone built with **Rust (Axum)** for the backend and **Next.js + Tailwind CSS** for the frontend.

Uses [`whatsapp-rust`](https://github.com/jlucaso1/whatsapp-rust) for WhatsApp protocol handling, QR code pairing, and end-to-end encrypted messaging.

## Architecture

```
whatsapp-rust-web/
├── backend/               Rust (Axum) REST API
│   ├── Cargo.toml
│   └── src/main.rs        Server + unit tests
├── frontend/              Next.js 14 + Tailwind CSS
│   ├── app/               App Router pages
│   ├── tests/             Playwright E2E tests
│   └── playwright.config.ts
├── run.sh                 One-command startup script
└── Makefile               Build / test / clean targets
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
| `POST` | `/api/messages/send` | Send a message: `{ "phone": "15551234567", "message": "Hello!" }` |

## Running Tests

```bash
# Backend unit tests (10 tests)
make test-backend
# or: cd backend && cargo test

# Frontend E2E tests (4 tests, requires Chromium)
make test-e2e
# or: cd frontend && npx playwright install chromium && npx playwright test

# All tests
make test
```

## Tech Stack

- **Backend:** Rust, Axum, `whatsapp-rust` (WhatsApp Web protocol), SQLite (session persistence)
- **Frontend:** Next.js 14, React 18, Tailwind CSS 3, `react-qr-code`
- **Testing:** Rust native `#[test]` (backend), Playwright (frontend E2E)

## Disclaimer

This is an unofficial, open-source project. Using custom WhatsApp clients may violate Meta's Terms of Service and could result in account suspension. **Use at your own risk.**