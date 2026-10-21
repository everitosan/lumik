# Proposal

## Why

Cada foto guarda su ubicación (`photo.dng_path` y `photo.jpg_path`) como ruta relativa al punto de montaje del disco, por ejemplo `lumik/2026/10/10_sesion/_media/IMG_0001.dng`. Esa ruta incluye la ubicación del proyecto dentro de `lumik/`, así que si se copia la carpeta del proyecto a otra ruta, aunque sea dentro de `lumik/` (por ejemplo `lumik/10_sesion/`), el proyecto se descubre y muestra sus miniaturas, que cuelgan de la carpeta del proyecto, pero todo lo que abre el archivo original falla: previews, rotación, papelera, aprobar o descartar, mostrar en carpeta y reescritura de metadatos. Se comprobó a mano al verificar `macos-device-handling`. Además, cada vez que Lumik mueve la carpeta del proyecto por fecha tiene que reescribir las rutas de todas las fotos.

## What Changes

- **Rutas relativas a la carpeta del proyecto**: `dng_path` y `jpg_path` se guardan relativas a la carpeta donde está el `project.db` (por ejemplo `_media/IMG_0001.dng`, `_culled/…`, `_trash/…`, `_exported/…`) y se resuelven con la carpeta en la que se descubrió el proyecto. El proyecto se vuelve autocontenido: se puede copiar a cualquier ruta bajo `lumik/` de cualquier disco.
- **Migración automática al abrir**: al abrir un `project.db` con rutas al estilo anterior, se reescriben quitando todo lo que va antes de la primera subcarpeta del proyecto (`_media`, `_culled`, `_trash` o `_exported`). Esto también arregla los proyectos que ya se movieron y quedaron con rutas rotas. Es idempotente y no toca las rutas que ya están en el formato nuevo.
- **Mover la carpeta del proyecto ya no reescribe la base**: al reubicar el proyecto por fecha o nombre solo se mueve la carpeta; las rutas de las fotos siguen siendo válidas.
- **`jpg_path` en el catálogo**: el desktop publica `jpg_path` relativo a la carpeta del proyecto (`_exported/IMG_0001.jpg`), que es el formato que ya esperan las pruebas del server. El server lo guarda sin interpretarlo; los valores viejos se sobrescriben en la siguiente publicación del proyecto.
- **BREAKING (solo desktop)**: una versión anterior de Lumik no sabe resolver las rutas de un `project.db` migrado. Se acepta porque hoy hay un único usuario, que actualiza todos sus equipos a la vez.

### Fuera de alcance

- Cambios en `apps/server`, `apps/web` o `packages/ts-types`: el server guarda `jpg_path` como texto opaco y el portal no lo usa.
- Proyectos que viven fuera de `lumik/` o a más profundidad de la que recorre el descubrimiento.
- Resolver o advertir proyectos duplicados entre discos (sigue siendo la limitación conocida).
- Eliminar `photo.device_uuid` o cambiar el registro de dispositivos.

## Capabilities

### New Capabilities

_Ninguna._

### Modified Capabilities

- `project-management`: "Identidad portable del proyecto" pasa a garantizar que el proyecto funciona completo (fotos, previews, edición) desde cualquier ruta bajo `lumik/` de cualquier disco, no solo desde la misma ruta relativa. "Editar proyecto" deja de exigir que se actualicen las rutas de las fotos al mover la carpeta y exige solo que sigan abriéndose. Nuevos requisitos "Rutas de fotos relativas al proyecto" y "Conversión de rutas de proyectos existentes", que también repara los proyectos ya movidos.
- `photo-import`: "Registro de fotos y metadatos EXIF" registra la ruta relativa a la carpeta del proyecto en lugar de al punto de montaje.

## Impact

- `apps/desktop/src-tauri/src/db/migrations.rs` y `db/project_schema.sql`: nueva migración idempotente de rutas y comentario del esquema actualizado.
- `apps/desktop/src-tauri/src/db/mod.rs`: `ProjectDatabase` resuelve las rutas de las fotos con `project_dir`; `mount_point` queda solo para el disco.
- Casos de uso que hoy reciben `mount_point` para unir rutas de fotos: `import_photos`, `cull_photo`, `trash_photo` (papelera y restaurar), `rate_photo`, `rename_project_files` y `rewrite_project_description`.
- Comandos `commands/photo.rs` (preview, rotar, mostrar en carpeta, regenerar miniaturas, papelera, aprobar), `commands/import_cmd.rs`, `commands/project.rs` (`relocate_project_folder` deja de llamar a `update_photo_paths_prefix`) y `commands/backup.rs` (enlace de `_exported`).
- `db/queries.rs`: se elimina `update_photo_paths_prefix`, que queda sin uso.
- `docs/useCases/flujo-importar-imagenes.md`, `docs/desktop-clean-architecture.md` y `apps/desktop/CLAUDE.md`: reflejar el nuevo formato de ruta.
- Frontend: sin cambios funcionales. `ProjectDetail` ordena por `dng_path` y muestra su nombre base; el orden relativo dentro del proyecto no cambia.
