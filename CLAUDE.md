# Lumik

Suite de aplicaciones para administrar fotografias de fotografos profesionales.
Permite convertir archivos RAW a DNG, organizar sesiones, etiquetar fotos para
culling, respaldar y entregar a clientes.

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

## Modelo de datos compartido

Tablas: fotografo, proyecto, fotografia, metadatos_fotografo.

Campos clave de fotografia:
- `dng_path`: relativo al mount point, nunca absoluto
- `device_name` / `device_uuid`: para detectar disponibilidad del disco
- `culled`: booleano, indica si vive en `_culled/`
- `workflow_status`: importada | editada | entregada
- Cache del DNG: stars, color_label, tags, capture_date, width, height, file_size_bytes

Ver `docs/schema.sql` para el esquema completo.

## Principios arquitectonicos globales

- **Paths relativos**: Los paths en BD son siempre relativos al mount point.
  El path completo se reconstruye en runtime via device_uuid.
- **DNG como fuente de verdad**: Los metadatos viven en el XMP del DNG.
  La BD es cache de consulta rapida.
- **Deteccion por UUID**: Dispositivos se identifican por UUID de filesystem,
  no por letra de unidad ni path de montaje.
- **Offline-first**: Cada componente debe funcionar sin conexion y sincronizar
  cuando haya red disponible.

## Estructura de carpetas en destino (discos externos)

```
/[mount_point]/lumik/[nombre-proyecto]/
  IMG_0001.dng
  IMG_0002.dng
  _culled/
    IMG_0047.dng
    IMG_0132.dng
```

## Vision a futuro

- Vinculacion de JPG editado (match por nombre o seleccion manual)
- Respaldo via API con cola offline-first
- Conversion WebP para entrega web optimizada
- Sincronizacion entre dispositivos via servidor
