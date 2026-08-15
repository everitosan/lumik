# Lumik — monorepo task runner (pnpm workspace)

LANDING_REMOTE := contabo:/var/www/lumik.evesan.rocks/

.DEFAULT_GOAL := help

# ---------------------------------------------------------------------------
# Setup
# ---------------------------------------------------------------------------

.PHONY: install
install: ## Instala dependencias del workspace (pnpm)
	pnpm install

.PHONY: clean
clean: ## Limpia artefactos de build (dist)
	rm -rf apps/*/dist packages/*/dist

# ---------------------------------------------------------------------------
# Dev
# ---------------------------------------------------------------------------

.PHONY: dev-desktop
dev-desktop: ## Levanta la app desktop en modo Tauri dev
	pnpm --filter @lumik/desktop tauri dev

.PHONY: dev-landing
dev-landing: ## Levanta la landing (Astro) en dev
	pnpm --filter @lumik/landing dev

.PHONY: dev-uikit
dev-uikit: ## Levanta Storybook del UI kit
	pnpm --filter @lumik/uikit dev

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

.PHONY: build-ui
build-ui: ## Compila el paquete @lumik/ui (dependencia de las apps)
	pnpm --filter @lumik/ui build

.PHONY: build-landing
build-landing: build-ui ## Build de producción de la landing
	pnpm --filter @lumik/landing build

.PHONY: build-uikit
build-uikit: build-ui ## Build estático de Storybook
	pnpm --filter @lumik/uikit build

.PHONY: build-desktop
build-desktop: build-ui ## Build de producción de la app desktop (Tauri)
	pnpm --filter @lumik/desktop tauri build

# ---------------------------------------------------------------------------
# Release
# ---------------------------------------------------------------------------

.PHONY: release
release: ## Libera la app desktop: sugiere/pregunta versión y empuja el tag (dispara GH Actions). Override: make release VERSION=v0.2.0-beta
	@bash utils/scripts/release.sh $(VERSION)

# ---------------------------------------------------------------------------
# Deploy
# ---------------------------------------------------------------------------

.PHONY: deploy-landing
deploy-landing: build-landing ## Build + sube la landing al server (rsync)
	rsync -avz --delete apps/landing/dist/ $(LANDING_REMOTE)

# ---------------------------------------------------------------------------
# Help
# ---------------------------------------------------------------------------

.PHONY: help
help: ## Muestra esta ayuda
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| sort \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'
