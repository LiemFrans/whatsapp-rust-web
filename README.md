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

- [Rust](https://www.rust-lang.org/tools/install) (1.70 or later)
- [Node.js](https://nodejs.org/) (18 or later) and npm

---

### Backend

The backend is a minimal HTTP server built with [Axum](https://github.com/tokio-rs/axum). It listens on port **3000** by default.

```bash
# From the repository root
cd backend
cargo run
```

The server will start and print:

```
Backend listening on http://0.0.0.0:3000
```

Visit <http://localhost:3000> to confirm it is running.

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
