# Alelysee - Dioxus Fullstack App Makefile
# ===========================================

.PHONY: help setup build test clean dev deploy docker-build docker-push db-setup

# Variables
ENVIRONMENT ?= dev

# Default target
help: ## Show this help message
	@echo "Alelysee - Dioxus Fullstack App"
	@echo "==============================="
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

# Setup and Development
# =====================

setup: ## Install dependencies and setup development environment
	@echo "Setting up development environment..."
	@if ! command -v jq >/dev/null 2>&1; then echo "Installing jq..."; brew install jq; fi
	cargo install dioxus-cli --locked
	cargo install cargo-watch

install-deps: ## Install all Cargo dependencies
	cargo fetch

# Development Commands
# ===================

dev: ## Start development server (fullstack mode)
	cd packages/web && dx serve -p web --web --fullstack

dev-server: ## Start only the server component
	cd packages/web && dx serve -p server --server --fullstack

dev-client: ## Start only the client component
	cd packages/web && dx serve -p web --web

dev-desktop: ## Start desktop app in development
	cd packages/desktop && dx serve -p desktop --desktop

dev-mobile: ## Start mobile app in development
	cd packages/mobile && dx serve -p mobile --mobile

# Building
# ========

build: ## Build all packages in debug mode
	cargo build --workspace

build-release: ## Build all packages in release mode
	cargo build --workspace --release

build-web: ## Build web package
	cargo build --package web --release

build-server: ## Build server binary
	cargo build --package web --release --features server

build-desktop: ## Build desktop app
	cargo build --package desktop --release

build-mobile: ## Build mobile app
	cargo build --package mobile --release

# Testing
# =======

test: ## Run all tests
	cargo test --workspace

test-ci: ## Run tests for CI (server-focused, excludes desktop/mobile)
	cargo test --workspace --exclude desktop --exclude mobile

test-api: ## Run API-specific tests
	cargo test --package api

test-ui: ## Run UI-specific tests
	cargo test --package ui

test-web: ## Run web-specific tests
	cargo test --package web

test-integration: ## Run API integration tests
	cargo test --package api --test '*' --features server -- --test-threads=1

test-e2e: ## Run E2E browser tests
	cargo test --package e2e --test '*' -- --test-threads=1

test-all: test test-integration test-e2e ## Run all tests (unit + integration + E2E)

# Database
# ========

db-migrate: ## Run database migrations
	cargo run --package api --bin migrate

db-setup: ## Setup database (requires DATABASE_URL)
	@echo "Setting up database..."
	@if [ -z "$$DATABASE_URL" ]; then echo "DATABASE_URL not set"; exit 1; fi
	cargo run --package api --bin migrate


# Docker Commands
# ===============

docker-build: ## Build Docker image
	docker build -t alelysee:latest .

docker-run: ## Run Docker container locally
	docker run -p 8080:8080 --env-file .env alelysee:latest

docker-push: ## Push Docker image to registry (requires DOCKER_REGISTRY)
	@if [ -z "$$DOCKER_REGISTRY" ]; then echo "DOCKER_REGISTRY not set"; exit 1; fi
	docker tag alelysee:latest $$DOCKER_REGISTRY/$$IMAGE_REPOSITORY:latest
	docker push $$DOCKER_REGISTRY/$$IMAGE_REPOSITORY:latest

health-check: ## Run health checks
	@echo "Running health checks..."
	@if curl -f http://localhost:8080/api/health >/dev/null 2>&1; then echo "✅ Server health check passed"; else echo "❌ Server health check failed"; fi

logs: ## Show application logs (requires running container)
	docker logs $$(docker ps -q --filter ancestor=alelysee)

# Cleanup
# =======

clean: ## Clean build artifacts
	cargo clean
	rm -rf target/

clean-docker: ## Clean Docker images and containers
	docker system prune -f
	docker image rm alelysee:latest 2>/dev/null || true

clean-all: clean clean-docker ## Clean everything

# Utility Commands
# ================

fmt: ## Format code
	cargo fmt --all

fmt-check: ## Check code formatting
	cargo fmt --all -- --check

lint: ## Run clippy linter
	cargo clippy --all-targets --all-features -- -D warnings

lint-ci: ## Run clippy for CI (server-focused, excludes desktop/mobile)
	cargo clippy --workspace --all-targets --features server --exclude desktop --exclude mobile -- -D warnings

check: ## Check code without building
	cargo check --workspace

update: ## Update dependencies
	cargo update

audit: ## Audit dependencies for security vulnerabilities
	cargo audit

# Environment Management
# ======================

env-check: ## Check environment variables
	@echo "Checking environment variables..."
	@if [ ! -f .env ]; then echo "❌ .env file not found"; exit 1; fi
	@grep -v '^#' .env | grep -v '^$$' | while read line; do \
		key=$$(echo $$line | cut -d'=' -f1); \
		value=$$(echo $$line | cut -d'=' -f2-); \
		if [ -z "$$value" ]; then echo "❌ $$key is not set"; else echo "✅ $$key = $$value"; fi; \
	done

env-template: ## Generate .env template from env.example
	cp env.example .env
	@echo "✅ .env template created. Please fill in the required values."

# Information
# ===========

info: ## Show project information
	@echo "Alelysee - Dioxus Fullstack App"
	@echo "================================="
	@echo "Packages:"
	@cargo tree --workspace --depth 0 | grep -E "(api|web|desktop|mobile|ui)"
	@echo ""
	@echo "Environment:"
	@if [ -f .env ]; then echo "✅ .env file exists"; else echo "❌ .env file missing"; fi
	@echo "Rust version: $$(rustc --version)"
	@echo "Cargo version: $$(cargo --version)"
	@echo "DX version: $$(dx --version 2>/dev/null || echo 'Not installed')"
