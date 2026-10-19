# Proposal

## Why

Lumik ya tiene funcionalidad publicada (v0.1.3-beta) pero su comportamiento solo está descrito en documentos vivos (`docs/ROADMAP.md`, `docs/backup-plan.md`, `docs/useCases/`) que ya se desfasan del código (p.ej. *Search by Tag* figura como pendiente y está implementado). Fijar una línea base de specs permite que los trabajos pendientes (OAuth, refuerzo de cuentas, Albums, distribución Android, batch ops) se planifiquen como changes con deltas sobre un contrato conocido.

## What Changes

- Se documenta, **sin cambiar código**, el comportamiento actual verificado contra el código de `apps/desktop/`, `apps/server/` y `apps/web/`.
- Cada capability se añade como requisitos nuevos (`ADDED`); al archivar este change pasan a `openspec/specs/`.
- Quedan **fuera** de la línea base (cada uno será su propio change):
  - Conexión de proveedores por OAuth (Google Drive, Dropbox, OneDrive) — hoy responde `501`.
  - Verificación de email, reset de contraseña y revocación de sesiones desde la UI.
  - Albums, operaciones en lote desde el grid.
  - Proyectos archivados: el backend ya sabe archivar, pero no hay interfaz. Idea inicial, por definir: una sección aparte de "archivados", fuera del dashboard principal.
  - Pendientes de Android: nombre real del dispositivo, inconsistencia thumbnail/preview, distribución en Play Store.
  - App iOS/iPad nativa.

## Capabilities

### New Capabilities

Desktop (Tauri — Linux, Windows, Android):
- `project-management`: alta de proyectos en un dispositivo destino, estructura de carpetas por fecha, renombrado y orden del dashboard.
- `device-detection`: detección de discos por UUID, montaje y expulsión.
- `photo-import`: wizard y pipeline de importación (copia, metadatos XMP del fotógrafo, EXIF, miniaturas/previews, videos).
- `photo-culling`: aprobar (culled = seleccionada) o quitar la aprobación moviendo la foto entre `_media/` y `_culled/` con rollback.
- `photo-rating`: estrellas, etiquetas de color, tags y su reflejo en sidecars para iPad.
- `photo-rotation`: rotación en el visor y su persistencia en miniatura y metadatos.
- `photo-filtering`: filtros de la vista de detalle de proyecto (tags, rating, color, estado).
- `photo-viewer`: grid de miniaturas y visor de fotos (zoom, pan, histograma, panel de información, gestos táctiles).
- `app-settings`: perfil y metadatos del fotógrafo, ajustes de la app, atajos de teclado configurables e idioma.

Respaldo en la nube (server axum + desktop + web):
- `backup-accounts`: registro, login, sesión JWT con refresh rotativo y aislamiento por fotógrafo.
- `backup-providers`: conexión de proveedores de nube por credenciales, cifradas en reposo.
- `catalog-sync`: publicación y reconciliación del catálogo de proyectos/fotos entre desktop y server, incluida la reconciliación de `_exported`.
- `cloud-backup`: subida de `_exported` a los proveedores (fan-out, cola offline-first, verificación por hash, reubicación al renombrar).
- `web-portal`: dashboard web/iPad con proyectos del catálogo y subida desde el navegador.

### Modified Capabilities

(ninguna — no existen specs previas)

## Impact

- Solo se añaden archivos bajo `openspec/`. No hay cambios de código, APIs ni dependencias.
- Los documentos de `docs/` siguen existiendo; los changes futuros deberán mantenerse coherentes con estas specs.
