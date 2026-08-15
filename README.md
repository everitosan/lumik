# Lumik

Suite de aplicaciones para que fotógrafos profesionales organicen, seleccionen, califiquen y etiqueten sus sesiones.

![Tauri](https://img.shields.io/badge/Tauri-24C8DB?logo=tauri&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?logo=typescript&logoColor=white)
![Astro](https://img.shields.io/badge/Astro-BC52EE?logo=astro&logoColor=white)
![Storybook](https://img.shields.io/badge/Storybook-FF4785?logo=storybook&logoColor=white)
![pnpm](https://img.shields.io/badge/pnpm-F69220?logo=pnpm&logoColor=white)

Monorepo pnpm con la app desktop (Tauri), la landing (Astro) y el UI kit (Storybook).

## Comandos

```bash
make install        # Instala dependencias del workspace (pnpm)
make clean          # Limpia artefactos de build (dist)

make dev-desktop    # App desktop en modo Tauri dev
make dev-landing    # Landing (Astro) en dev
make dev-uikit      # Storybook del UI kit

make build-ui       # Compila @lumik/ui (dependencia de las apps)
make build-desktop  # Build de producción de la app desktop (Tauri)
make build-landing  # Build de producción de la landing
make build-uikit    # Build estático de Storybook

make release        # Libera la app desktop (tag → GH Actions). Override: make release VERSION=v0.2.0-beta
make deploy-landing # Build + sube la landing al server (rsync)

make help           # Muestra todos los comandos
```
