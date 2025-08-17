# Elgato Pedal Controller - Makefile
# A comprehensive build system for the Rust-based Elgato Stream Deck Pedal controller

# Variables
CARGO := cargo
BINARY_NAME := elgato-pedal-controller
TARGET_DIR := target
RELEASE_DIR := $(TARGET_DIR)/x86_64-unknown-linux-gnu/release
DEBUG_DIR := $(TARGET_DIR)/x86_64-unknown-linux-gnu/debug
DEST := $(HOME)/.local/bin

# Default target
.PHONY: all
all: build

# Build targets
.PHONY: build
build: ## Build the project in debug mode
	@echo "Building $(BINARY_NAME) (debug mode)..."
	$(CARGO) build

.PHONY: release
release: ## Build the project in release mode (optimized)
	@echo "Building $(BINARY_NAME) (release mode)..."
	$(CARGO) build --release

.PHONY: run
run: build ## Build and run the project
	@echo "Running $(BINARY_NAME)..."
	$(CARGO) run

.PHONY: run-release
run-release: release ## Build and run the project in release mode
	@echo "Running $(BINARY_NAME) (release mode)..."
	$(CARGO) run --release

# Development tools
.PHONY: check
check: ## Check the project for errors without building
	@echo "Checking project for errors..."
	$(CARGO) check

.PHONY: test
test: ## Run all tests
	@echo "Running tests..."
	$(CARGO) test

.PHONY: fmt
fmt: ## Format all source code
	@echo "Formatting source code..."
	$(CARGO) fmt

.PHONY: fmt-check
fmt-check: ## Check if code is properly formatted
	@echo "Checking code formatting..."
	$(CARGO) fmt --check

.PHONY: clippy
clippy: ## Run clippy linter
	@echo "Running clippy linter..."
	$(CARGO) clippy -- -D warnings

.PHONY: clippy-fix
clippy-fix: ## Run clippy with automatic fixes
	@echo "Running clippy with automatic fixes..."
	$(CARGO) clippy --fix --allow-dirty

# Documentation
.PHONY: doc
doc: ## Generate documentation
	@echo "Generating documentation..."
	$(CARGO) doc --no-deps

.PHONY: doc-open
doc-open: doc ## Generate and open documentation in browser
	@echo "Opening documentation..."
	$(CARGO) doc --no-deps --open

# Security and dependency management
.PHONY: audit
audit: ## Run security audit on dependencies
	@echo "Running security audit..."
	$(CARGO) audit

.PHONY: update
update: ## Update dependencies
	@echo "Updating dependencies..."
	$(CARGO) update

.PHONY: outdated
outdated: ## Check for outdated dependencies (requires cargo-outdated)
	@echo "Checking for outdated dependencies..."
	@if command -v cargo-outdated >/dev/null 2>&1; then \
		$(CARGO) outdated; \
	else \
		echo "cargo-outdated not installed. Run: cargo install cargo-outdated"; \
	fi

# Cleanup
.PHONY: clean
clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	$(CARGO) clean

.PHONY: clean-all
clean-all: clean ## Clean all artifacts including Cargo registry cache
	@echo "Cleaning all artifacts..."
	rm -rf ~/.cargo/registry/cache

# Installation
.PHONY: install
install: release ## Install the binary to ~/.local/bin
	@echo "Installing $(BINARY_NAME) to $(DEST)..."
	@mkdir -p "$(DEST)"
	@cp "$(RELEASE_DIR)/$(BINARY_NAME)" "$(DEST)/$(BINARY_NAME)"
	@chmod +x "$(DEST)/$(BINARY_NAME)"
	@echo "✅ Installed: $(DEST)/$(BINARY_NAME)"
	@$(MAKE) -s pathcheck
	@echo "Installing service by running `$(DEST)/$(BINARY_NAME) install`"
	@$(DEST)/$(BINARY_NAME) install || echo "⚠️ Failed to install service. Ensure you have systemd and the necessary permissions."


.PHONY: reinstall
reinstall: uninstall install ## Uninstall and reinstall the binary

.PHONY: uninstall
uninstall: ## Uninstall the binary and service from ~/.local/bin
	@echo "Uninstalling $(BINARY_NAME)..."
	@if [ -f "$(DEST)/$(BINARY_NAME)" ]; then \
		echo "Stopping and removing service..."; \
		$(DEST)/$(BINARY_NAME) uninstall || echo "⚠️ Failed to uninstall service (may not be installed)"; \
	fi
	@rm -f "$(DEST)/$(BINARY_NAME)" && echo "✅ Removed binary: $(DEST)/$(BINARY_NAME)" || true

.PHONY: pathcheck
pathcheck: ## Check if ~/.local/bin is on PATH
	@echo "$$PATH" | tr ':' '\n' | grep -qx "$(DEST)" || \
	  echo "⚠️  Note: $(DEST) is not on your PATH. Add this to your shell rc:\n  export PATH=\"$(DEST):$$PATH\""

# Development workflow
.PHONY: dev-setup
dev-setup: ## Setup development environment
	@echo "Setting up development environment..."
	@echo "Installing required tools..."
	@command -v cargo-audit >/dev/null 2>&1 || $(CARGO) install cargo-audit
	@command -v cargo-outdated >/dev/null 2>&1 || $(CARGO) install cargo-outdated
	@echo "Development environment ready!"

.PHONY: pre-commit
pre-commit: fmt clippy test ## Run pre-commit checks (format, lint, test)
	@echo "Pre-commit checks completed successfully!"

.PHONY: pre-release
pre-release: clean fmt-check clippy test audit release doc ## Run comprehensive pre-release validation
	@echo "Pre-release validation completed successfully!"
	@echo "Ready for release!"

# Quick commands
.PHONY: quick-check
quick-check: check fmt clippy ## Quick development checks
	@echo "Quick checks completed!"

# System requirements check
.PHONY: check-deps
check-deps: ## Check system dependencies
	@echo "Checking system dependencies..."
	@echo "Rust toolchain:"
	@rustc --version || echo "❌ Rust not installed"
	@cargo --version || echo "❌ Cargo not installed"
	@echo "Optional tools:"
	@command -v cargo-audit >/dev/null 2>&1 && echo "✅ cargo-audit installed" || echo "❌ cargo-audit not installed (run: cargo install cargo-audit)"
	@command -v cargo-outdated >/dev/null 2>&1 && echo "✅ cargo-outdated installed" || echo "❌ cargo-outdated not installed (run: cargo install cargo-outdated)"

# Help target
.PHONY: help
help: ## Show this help message
	@echo "$(BINARY_NAME) - Makefile targets:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Usage examples:"
	@echo "  make install        # Install binary to ~/.local/bin"
	@echo "  make build          # Build debug version"
	@echo "  make release        # Build release version"
	@echo "  make pre-commit     # Run all pre-commit checks"
	@echo "  make pre-release    # Run comprehensive validation for release"
	@echo ""
	@echo "After installation, use the binary directly:"
	@echo "  $(BINARY_NAME) install    # Install as systemd service"
	@echo "  $(BINARY_NAME) config     # Edit configuration"
	@echo "  $(BINARY_NAME) run        # Run the controller (or just run without args)"
