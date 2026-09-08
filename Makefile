CARGO := cargo

BOLD  := \033[1m
CYAN  := \033[36m
GREEN := \033[32m
RESET := \033[0m

.DEFAULT_GOAL := help

# ── Build ────────────────────────────────────────────────────────────────────

.PHONY: build release

build: ## Debug build
	$(CARGO) build -p combe

release: ## Release build
	$(CARGO) build --release -p combe

# ── Quality ──────────────────────────────────────────────────────────────────

.PHONY: check audit test lint fmt

check: audit ## Full quality gate — dependency audit, format, lint, test
	@printf '\n$(BOLD)[2/4] Checking format$(RESET)\n'
	$(CARGO) fmt --all -- --check
	@printf '\n$(BOLD)[3/4] Running clippy$(RESET)\n'
	$(CARGO) clippy --workspace --all-targets -- -D warnings
	@printf '\n$(BOLD)[4/4] Running tests$(RESET)\n'
	$(CARGO) test --workspace
	@printf '\n$(GREEN)  ✓ All checks passed$(RESET)\n\n'

audit: ## Check locked Rust dependencies for known vulnerabilities
	@printf '\n$(BOLD)[1/4] Auditing dependencies$(RESET)\n'
	$(CARGO) audit --file Cargo.lock

test: ## Run workspace tests
	$(CARGO) test --workspace

lint: ## Run clippy
	$(CARGO) clippy --workspace --all-targets -- -D warnings

fmt: ## Format code
	$(CARGO) fmt --all

# ── App ──────────────────────────────────────────────────────────────────────

APP := /Applications/Combe.app
APP_BIN := $(APP)/Contents/MacOS/Combe
BINDIR ?= $(shell \
	if [ -w /opt/homebrew/bin ]; then echo /opt/homebrew/bin; \
	elif [ -w /usr/local/bin ]; then echo /usr/local/bin; \
	else echo $(HOME)/.local/bin; fi)

.PHONY: app run install uninstall

run: app ## Build and run build/Combe.app in the foreground
	build/Combe.app/Contents/MacOS/Combe

app: ## Build build/Combe.app
	bash scripts/package_app.sh

install: app ## Install Combe.app and link combe onto PATH
	rm -rf $(APP)
	cp -R build/Combe.app $(APP)
	-/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f $(APP) >/dev/null 2>&1
	mkdir -p $(BINDIR)
	ln -sf $(APP_BIN) $(BINDIR)/combe
	@echo $(APP)
	@echo $(BINDIR)/combe

uninstall: ## Remove Combe.app and the combe symlink
	rm -rf $(APP)
	@if [ -L "$(BINDIR)/combe" ]; then \
		target=$$(readlink "$(BINDIR)/combe"); \
		if [ "$$target" = "$(APP_BIN)" ]; then rm -f "$(BINDIR)/combe"; fi; \
	fi
	@echo removed $(APP)

# ── Maintenance ──────────────────────────────────────────────────────────────

.PHONY: clean

clean: ## Remove build artifacts
	$(CARGO) clean

# ── Help ─────────────────────────────────────────────────────────────────────

.PHONY: help

help: ## Show available targets
	@awk 'BEGIN {FS = ":.*## "; printf "\n$(BOLD)Combe$(RESET) — worktree-aware terminal\n"} \
		/^# ── / {n = $$0; gsub(/(^# ── | ─+$$)/, "", n); printf "\n$(BOLD)%s$(RESET)\n", n} \
		/^[a-zA-Z_-]+:.*## / {printf "  $(CYAN)make %-12s$(RESET) %s\n", $$1, $$2} \
		END {printf "\n"}' $(MAKEFILE_LIST)
