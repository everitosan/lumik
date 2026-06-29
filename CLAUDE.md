# Lumik

Suite de aplicaciones para administrar fotografias de fotografos profesionales.
Permite organizar sesiones en proyectos, seleccionar, etiquetar, calificar y tagear fotos.

## Componentes de la suite

| App | Ubicacion | Stack | Estado |
|-----|-----------|-------|--------|
| Desktop Linux | `apps/desktop/` | Tauri (Rust + Web) | MVP en desarrollo |
| Servidor backups | `apps/server/` | Por definir | Planificado |
| Portal web clientes | `apps/web/` | Por definir | Planificado |
| App iPad | `apps/ios/` | Swift/SwiftUI | Planificado (requiere Mac) |

## Estructura del monorepo

```
lumik/
├── apps/
│   ├── desktop/      # App Linux - importacion y culling
│   ├── server/       # API de respaldo
│   ├── web/          # Portal de entrega a clientes
│   └── ios/          # App iPad para campo
├── packages/
│   ├── ts-types/ # Tipos compartidos (TS)
│   └── ui/           # Componentes UI reutilizables
└── docs/             # Documentacion compartida
```

