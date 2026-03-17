# whatsapp-rust-web

A full-stack monorepo for a WhatsApp client, built with a **Rust/Axum** backend and a **React/Vite** frontend.

---

## Project Structure

```
whatsapp-rust-web/
├── backend/          # Rust web server (Axum)
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── frontend/         # React application (Vite + TypeScript)
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
│       └── ...
├── .gitignore
└── README.md
```

---

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) with **nightly** toolchain (`rustup toolchain install nightly`)
  - The backend's `rust-toolchain.toml` selects nightly automatically; `cargo run` picks it up without any extra flags.
- [Node.js](https://nodejs.org/) (18 or later) and npm

---

### Backend

The backend is a Rust HTTP server built with [Axum](https://github.com/tokio-rs/axum) that integrates the [whatsapp-rust](https://github.com/jlucaso1/whatsapp-rust) client library. It listens on port **3000** by default.

```bash
# From the repository root
cd backend
cargo run
```

On first run, a QR code is printed to the terminal — scan it with the WhatsApp mobile app to authenticate. The session is saved in `backend/whatsapp.db` for subsequent runs.

The server will also print:

```
Backend listening on http://0.0.0.0:3000
```

Visit <http://localhost:3000> to confirm it is running; the response includes the current WhatsApp connection status.

---

### Frontend

The frontend is a [React](https://react.dev/) application scaffolded with [Vite](https://vitejs.dev/). The development server runs on port **5173** by default.

```bash
# From the repository root
cd frontend
npm install        # first time only
npm run dev
```

Open <http://localhost:5173> in your browser.

---

### Running both simultaneously

Open two terminal windows (or use a tool such as [tmux](https://github.com/tmux/tmux)):

**Terminal 1 – Backend**

```bash
cd backend && cargo run
```

**Terminal 2 – Frontend**

```bash
cd frontend && npm run dev
```

---

## License

See [LICENSE](./LICENSE).
