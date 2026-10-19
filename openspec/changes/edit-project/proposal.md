# Proposal

## Why

Hoy solo se puede cambiar el nombre de un proyecto: el modal "Editar proyecto" tiene un único campo, y la descripción y la fecha de sesión quedan fijas desde la creación (o desde la primera importación, que infiere la fecha). Además, al renombrar solo cambian la carpeta y la base de datos. Las fotos importadas con renombrado conservan el nombre viejo en el archivo (`…__Boda.cr3`), y la descripción que llevan dentro (`{descripción}@{año}`) no puede corregirse.

## What Changes

- El modal "Editar proyecto" permite editar **nombre** (obligatorio), **fecha de sesión** (obligatoria) y **descripción**, con las mismas reglas que al crear el proyecto.
- Al guardar se actualiza siempre la base del proyecto y, si cambian el nombre o la fecha, la carpeta pasa a `lumik/{año}/{mes}/{día}_{slug}`.
- La fecha se precarga con la fecha de sesión o, si el proyecto aún no tiene, con la fecha de su carpeta (la de creación). Si se guarda sin cambiarla, el proyecto sigue sin fecha de sesión y la primera importación todavía puede inferirla. Una fecha distinta se toma como intencional y ya no se infiere.
- Casilla **"Renombrar también los archivos"** (aparece si cambió el nombre y hay archivos con el nombre viejo): renombra las fotos de `_media/` y `_culled/` importadas con renombrado, junto con sus sidecars, usando el nombre saneado nuevo. Si un renombrado falla, se deshacen los ya hechos.
- Casilla **"Actualizar los metadatos de las fotos ya importadas"** (aparece si cambió la descripción o el año y hay fotos): reescribe `{descripción}@{año}` en las fotos existentes (dentro del archivo en Linux/Windows, en el sidecar `.xmp` en Android). Informa del progreso y de las fotos que fallaron.
- Las dos casillas empiezan desmarcadas. `_exported/` no se modifica nunca, y si cambia el nombre el modal avisa de que los editores externos pueden perder la referencia a los archivos renombrados.
- Tras guardar se mueve la carpeta del respaldo en la nube (como hoy al renombrar) y se resincroniza el catálogo del servidor.
- **BREAKING (interno):** el comando `rename_project` se sustituye por `edit_project`. Su único consumidor es el dashboard del desktop.

## Capabilities

### New Capabilities
<!-- Ninguna -->

### Modified Capabilities
- `project-management`: "Renombrar proyecto" se sustituye por "Editar proyecto" (nombre, fecha y descripción). Se añaden los requisitos de renombrar archivos y de actualizar los metadatos de las fotos existentes, ambos opcionales.
- `photo-import`: "Fecha de sesión inferida" aclara que una fecha fijada al editar el proyecto no se sustituye.

## Impact

- Backend desktop: `commands/project.rs` (`edit_project` reemplaza a `rename_project`), un caso de uso nuevo para renombrar archivos con rollback y otro para reescribir la descripción (`application/use_cases/`), reglas puras en `domain/project.rs` (fecha efectiva, nombre de archivo renombrado), el saneado del nombre movido de `import/pipeline.rs` a `domain` para compartirlo, un método nuevo en `MetadataTool` (desktop: exiftool; Android: sidecar `.xmp`), `db/queries.rs` (actualizar descripción y fecha, renombrar `dng_path`).
- Frontend desktop: `EditProjectModal` sustituye a `RenameProjectModal`, `Projects.tsx`, `lib/api.ts`, `lib/types.ts`, i18n `project.edit.*`.
- Documentación: `apps/desktop/CLAUDE.md` (comandos), `docs/useCases/` (diagrama nuevo de edición de proyecto).
- Fuera de alcance: renombrar archivos de `_exported/`, que `sync_exported` recuerde los nombres anteriores, y renombrar los archivos importados sin renombrado (`IMG_4821.CR3`).
