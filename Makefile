# ──────────────────────────────────────────────────────────────────────────────
# Makefile — WhatsApp Web Clone
# ──────────────────────────────────────────────────────────────────────────────
.PHONY: all install dev backend frontend test test-backend test-e2e clean

BACKEND_PORT ?= 8080

# ── Default: install deps then start dev ──────────────────────────────────────
all: install dev

# ── Install all dependencies ──────────────────────────────────────────────────
install:
	@echo "📦 Installing frontend dependencies…"
	cd frontend && npm install --no-audit --no-fund
	@echo "📦 Checking Rust backend compiles…"
	cd backend && cargo check

# ── Start both servers (parallel) ─────────────────────────────────────────────
dev:
	@echo "🚀 Starting backend (port $(BACKEND_PORT)) + frontend (port 3000)…"
	@$(MAKE) -j2 backend frontend

backend:
	cd backend && PORT=$(BACKEND_PORT) cargo run

frontend:
	cd frontend && npm run dev

# ── Tests ─────────────────────────────────────────────────────────────────────
test: test-backend test-e2e

test-backend:
	@echo "🧪 Running Rust unit tests…"
	cd backend && cargo test

test-e2e:
	@echo "🧪 Installing Playwright browsers…"
	cd frontend && npx playwright install --with-deps chromium
	@echo "🧪 Running Playwright E2E tests…"
	cd frontend && npx playwright test

# ── Cleanup ───────────────────────────────────────────────────────────────────
clean:
	cd backend && cargo clean
	rm -rf frontend/node_modules frontend/.next
	rm -rf frontend/test-results frontend/playwright-report
