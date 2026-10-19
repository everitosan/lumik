# Lumik

Suite de aplicaciones para que fotógrafos profesionales organicen, seleccionen, califiquen y etiqueten sus sesiones.

![Tauri](https://img.shields.io/badge/Tauri-24C8DB?logo=tauri&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?logo=typescript&logoColor=white)
![Astro](https://img.shields.io/badge/Astro-BC52EE?logo=astro&logoColor=white)
![Storybook](https://img.shields.io/badge/Storybook-FF4785?logo=storybook&logoColor=white)
![pnpm](https://img.shields.io/badge/pnpm-F69220?logo=pnpm&logoColor=white)

Monorepo pnpm con:

- **Desktop** (Tauri v2 · Linux, Windows, Android): importación, culling, calificación y respaldo.
- **Server de respaldo** (Rust + axum + Postgres + rclone): cuentas, catálogo y subida a la nube.
- **Portal web** (React + Vite): dashboard de proyectos y subida desde el navegador o el iPad.
- **Landing** (Astro) y **UI kit** (Storybook de `@lumik/ui`).

El comportamiento de cada capability está especificado en [`openspec/specs/`](openspec/specs/);
el plan y el estado están en [`docs/ROADMAP.md`](docs/ROADMAP.md) y el [CHANGELOG](CHANGELOG.md).

## Comandos

```bash
make install        # Instala dependencias del workspace (pnpm)
make clean          # Limpia artefactos de build (dist)

make dev-desktop    # App desktop en modo Tauri dev
make dev-landing    # Landing (Astro) en dev
make dev-uikit      # Storybook del UI kit
make db-up          # Postgres del server de respaldo (docker compose)
make dev-server     # Server de respaldo (Postgres + API en :8787)
make dev-web        # Server de respaldo + portal web (API :8787, web :5173)

make build-ui       # Compila @lumik/ui (dependencia de las apps)
make build-desktop  # Build de producción de la app desktop (Tauri)
make build-landing  # Build de producción de la landing
make build-uikit    # Build estático de Storybook

make release        # Libera la app desktop (tag → GH Actions). Override: make release VERSION=v0.2.0-beta
make deploy-landing # Build + sube la landing al server (rsync)

make help           # Muestra todos los comandos
```
