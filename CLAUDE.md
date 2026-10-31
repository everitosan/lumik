# Lumik

Suite de aplicaciones para administrar fotografias de fotografos profesionales.
Permite organizar sesiones en proyectos, seleccionar, etiquetar, calificar y tagear fotos.

## Componentes de la suite

| App | Ubicacion | Stack | Estado |
|-----|-----------|-------|--------|
| Desktop (Linux, Windows, macOS) | `apps/desktop/` | Tauri v2 (Rust + React) | Beta (v0.2.0) |
| Servidor de respaldo | `apps/server/` | Rust + axum + Postgres + rclone | Beta |
| Portal web de respaldo | `apps/web/` | React 19 + Vite | Beta |
| Landing | `apps/landing/` | Astro | Publicada |
| UI kit (Storybook) | `apps/uikit/` | Storybook + Vite | Activo |
| App de tablet (Android; iPad después) | `apps/tablet/` | Expo / React Native + `lumik-core` por uniffi (`crates/lumik-mobile`) | En desarrollo: primera beta para Google Play; el iPad espera la cuenta de Apple |
| App iPad nativa | `apps/ios/` | Swift/SwiftUI | Descartada a favor de la app de tablet en React Native (`tablet-app-spike`) |

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
│   ├── tablet/       # App de tablet Android (Expo/RN + núcleo por uniffi)
│   └── ios/          # App iPad nativa (vacía, descartada a favor de la de tablet)
├── packages/
│   ├── ts-types/     # Tipos compartidos (TS)
│   └── ui/           # Componentes UI reutilizables
├── crates/
│   ├── lumik-core/   # Núcleo Rust compartido (dominio, casos de uso, import, SQLite)
│   └── lumik-mobile/ # Puente del núcleo para la tablet (uniffi)
├── openspec/         # Specs de comportamiento y changes (OpenSpec)
└── docs/             # Documentacion compartida
```

