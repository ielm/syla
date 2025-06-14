# Syla Platform Workspace Makefile
# Orchestrates building and deployment of all services

# Platform and service discovery
PLATFORMS := syla
SYLA_SERVICES := platforms/syla/core/api-gateway platforms/syla/core/execution-service platforms/syla/tools/cli

# All services across all platforms
ALL_SERVICES := $(SYLA_SERVICES)

# Docker configuration
DOCKER_REGISTRY ?= syla
VERSION ?= $(shell git describe --always --dirty 2>/dev/null || echo "dev")

# Detect changed services using git
CHANGED_SERVICES := $(shell ./scripts/detect-changes.sh 2>/dev/null || echo $(ALL_SERVICES))

.PHONY: help
help:
	@echo "Syla Platform Build System"
	@echo ""
	@echo "Available targets:"
	@echo "  all            - Build all services and CLI"
	@echo "  build-changed  - Build only changed services (default)"
	@echo "  proto-deps     - Setup proto dependencies"
	@echo "  test           - Run all tests"
	@echo "  clean          - Clean all build artifacts"
	@echo "  dev-up         - Start development environment"
	@echo "  dev-down       - Stop development environment"
	@echo "  dev-restart    - Restart changed services"
	@echo "  dev-watch      - Watch for changes and auto-rebuild"
	@echo ""
	@echo "Service-specific targets:"
	@echo "  <service>-build       - Build specific service"
	@echo "  <service>-test        - Test specific service"
	@echo "  <service>-clean       - Clean specific service"
	@echo ""
	@echo "Current changed services: $(CHANGED_SERVICES)"

# Default target - build only changed services
.PHONY: build-changed
build-changed: proto-deps $(CHANGED_SERVICES)

# Build all services
.PHONY: all
all: proto-deps $(ALL_SERVICES) cli-build

# Setup proto dependencies
.PHONY: proto-deps
proto-deps:
	@echo "Setting up proto dependencies..."
	@./scripts/setup-proto-deps.sh

# Pattern rule for building services
.PHONY: $(ALL_SERVICES)
$(ALL_SERVICES): proto-deps
	@echo "Building $@..."
	@if [ -f "$@/Makefile" ]; then \
		$(MAKE) -C $@ build; \
	elif [ -f "$@/Cargo.toml" ]; then \
		cd $@ && cargo build --release; \
	else \
		echo "No build system found for $@"; \
	fi

# Service-specific targets
%-build: proto-deps
	@service=$$(echo $(ALL_SERVICES) | tr ' ' '\n' | grep -E "$*$$" | head -1); \
	if [ -n "$$service" ]; then \
		echo "Building $$service..."; \
		if [ -f "$$service/Makefile" ]; then \
			$(MAKE) -C $$service build; \
		elif [ -f "$$service/Cargo.toml" ]; then \
			cd $$service && cargo build --release; \
		fi; \
	else \
		echo "Service not found: $*"; \
		exit 1; \
	fi

%-test:
	@service=$$(echo $(ALL_SERVICES) | tr ' ' '\n' | grep -E "$*$$" | head -1); \
	if [ -n "$$service" ]; then \
		echo "Testing $$service..."; \
		if [ -f "$$service/Makefile" ]; then \
			$(MAKE) -C $$service test; \
		elif [ -f "$$service/Cargo.toml" ]; then \
			cd $$service && cargo test; \
		fi; \
	else \
		echo "Service not found: $*"; \
		exit 1; \
	fi

%-clean:
	@service=$$(echo $(ALL_SERVICES) | tr ' ' '\n' | grep -E "$*$$" | head -1); \
	if [ -n "$$service" ]; then \
		echo "Cleaning $$service..."; \
		if [ -f "$$service/Makefile" ]; then \
			$(MAKE) -C $$service clean; \
		elif [ -f "$$service/Cargo.toml" ]; then \
			cd $$service && cargo clean; \
		fi; \
	else \
		echo "Service not found: $*"; \
		exit 1; \
	fi

# CLI specific targets
.PHONY: cli-build
cli-build:
	@echo "Building Syla CLI..."
	@cd cli && cargo build --release
	@ln -sf cli/target/release/syla syla

# Test all services
.PHONY: test
test:
	@for service in $(ALL_SERVICES); do \
		echo "Testing $$service..."; \
		if [ -f "$$service/Makefile" ]; then \
			$(MAKE) -C $$service test || exit 1; \
		elif [ -f "$$service/Cargo.toml" ]; then \
			cd $$service && cargo test || exit 1; \
		fi; \
	done

# Development environment management
.PHONY: dev-up
dev-up:
	@echo "Starting development environment..."
	@docker compose up -d
	@echo "Starting services..."
	@./syla dev up --detach

.PHONY: dev-down
dev-down:
	@echo "Stopping development environment..."
	@./syla dev down
	@docker compose down

.PHONY: dev-restart
dev-restart: build-changed
	@echo "Restarting changed services..."
	@for service in $(CHANGED_SERVICES); do \
		service_name=$$(basename $$service); \
		./syla dev restart $$service_name || true; \
	done

# Watch for changes and auto-rebuild
.PHONY: dev-watch
dev-watch:
	@echo "Watching for changes (press Ctrl+C to stop)..."
	@while true; do \
		changed=$$(./scripts/detect-changes.sh); \
		if [ -n "$$changed" ]; then \
			echo "Detected changes in: $$changed"; \
			for service in $$changed; do \
				if $(MAKE) $$service; then \
					service_name=$$(basename $$service); \
					./syla dev restart $$service_name || true; \
				fi; \
			done; \
		fi; \
		sleep 2; \
	done

# Clean all services
.PHONY: clean
clean:
	@for service in $(ALL_SERVICES); do \
		echo "Cleaning $$service..."; \
		if [ -f "$$service/Makefile" ]; then \
			$(MAKE) -C $$service clean || true; \
		elif [ -f "$$service/Cargo.toml" ]; then \
			cd $$service && cargo clean || true; \
		fi; \
	done
	@rm -f syla

# Setup all services
.PHONY: setup
setup: proto-deps
	@echo "Setting up all services..."
	@for service in $(ALL_SERVICES); do \
		echo "Setting up $$service..."; \
		./scripts/setup-service.sh $$service || exit 1; \
	done

# Initialize the workspace
.PHONY: init
init:
	@echo "Initializing Syla workspace..."
	@./syla init