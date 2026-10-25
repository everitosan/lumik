# Lumik

Suite de aplicaciones para administrar fotografias de fotografos profesionales.
Permite organizar sesiones en proyectos, seleccionar, etiquetar, calificar y tagear fotos.

## Componentes de la suite

| App | Ubicacion | Stack | Estado |
|-----|-----------|-------|--------|
| Desktop (Linux, Windows, Android) | `apps/desktop/` | Tauri v2 (Rust + React) | Beta (v0.1.6) |
| Servidor de respaldo | `apps/server/` | Rust + axum + Postgres + rclone | Beta |
| Portal web de respaldo | `apps/web/` | React 19 + Vite | Beta |
| Landing | `apps/landing/` | Astro | Publicada |
| UI kit (Storybook) | `apps/uikit/` | Storybook + Vite | Activo |
| App iPad | `apps/ios/` | Swift/SwiftUI | Planificado (requiere Mac); hoy el iPad usa el portal web |

La especificación de comportamiento de cada capability vive en `openspec/specs/`.

## Estructura del monorepo

```
lumik/
├── apps/
│   ├── desktop/      # App Tauri - importacion, culling y respaldo
│   ├── server/       # API de respaldo (axum + Postgres + rclone)
│   ├── web/          # Portal web de respaldo (tambien para iPad)
│   ├── landing/      # Sitio publico (Astro)
│   ├── uikit/        # Storybook de @lumik/ui
│   └── ios/          # App iPad nativa (vacia, planificada)
├── packages/
│   ├── ts-types/     # Tipos compartidos (TS)
│   └── ui/           # Componentes UI reutilizables
├── crates/
│   └── lumik-core/   # Núcleo Rust compartido (dominio, casos de uso, import, SQLite)
├── openspec/         # Specs de comportamiento y changes (OpenSpec)
└── docs/             # Documentacion compartida
```

