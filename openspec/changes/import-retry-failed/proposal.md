# Proposal

## Why

Desde la v0.1.5 el resultado del import lista las fotos que fallaron y su motivo, pero lo único que se puede hacer es "Aceptar". Para recuperarlas hay que volver a abrir el wizard, buscar en el origen justo esos archivos y repetir los ajustes. En una sesión real fallaron 3 de ~40 fotos por metadatos y, al reimportarlas a mano, entraron sin problema: el fallo era pasajero y un reintento desde el resultado lo habría resuelto.

## What Changes

- **Botón "Reintentar"** en el resultado del import cuando hay archivos fallidos que se pueden reintentar. Reintenta solo esos, en el mismo proyecto y con los mismos ajustes de la importación (renombrado, firma).
- **Según el motivo del fallo**:
  - si falló al copiar, preparar o firmar (`source_unreadable`, `write_failed`, `metadata_failed`, `unknown`), se vuelve a importar desde el archivo de origen;
  - si faltó espacio (`no_space`), se reintenta igual, pero solo si el disco tiene ahora espacio para esos archivos;
  - si falló el registro en el proyecto (`register_failed`), los archivos ya están en `_media/`: solo se repite el registro (EXIF, BD y miniaturas), sin volver a copiarlos ni borrarlos.
- **Resultado acumulado**: tras cada reintento, el resultado muestra el total de la sesión (fotos importadas, videos copiados, fallidos que quedan) y el registro sigue creciendo. Si algo vuelve a fallar, se puede reintentar de nuevo.
- Textos nuevos en i18n (es/en).

### Fuera de alcance

- Reintentar lo que quedó sin intentar tras un error general (`fatal`): hoy esos archivos no aparecen como fallidos.
- Elegir qué fallidos reintentar: se reintentan todos los reintentables.
- Investigar por qué la firma falla de forma pasajera (`sign_staged`); el reintento no lo arregla, solo lo hace recuperable.

## Capabilities

### New Capabilities

_Ninguna._

### Modified Capabilities

- `photo-import`: nuevo requisito "Reintentar archivos fallidos" (botón en el resultado, reintento por motivo, solo registrar para `register_failed`, resultado acumulado). Se agrega como requisito propio para no depender del change `import-feedback-and-errors`, que aún no se archiva.

## Impact

- `apps/desktop/src-tauri/src/application/use_cases/import_photos.rs`: el registro (EXIF + `create_batch` + miniaturas) se expone para usarlo solo, sobre archivos ya en `_media/`, omitiendo los que ya estén registrados.
- `apps/desktop/src-tauri/src/commands/import_cmd.rs`: nuevo comando para reintentar el registro, que comparte con `start_import` la fecha de sesión inferida y el renombrado de la carpeta; registro en `lib.rs` y wrapper en `src/lib/api.ts`.
- `apps/desktop/src-tauri/src/db/`: consulta de qué `dng_path` ya están registrados.
- Frontend: `useImport()` en `src/lib/hooks.ts` (reintento con resultado y registro acumulados), `pages/ImportPage.tsx` (botón, revalidación de espacio) y `src/i18n/locales/{es,en}.json`.
- Documentación: `docs/useCases/flujo-importar-imagenes.md` y la sección "Pipeline de importación" de `apps/desktop/CLAUDE.md`.
- Sin cambios en `apps/server`, `apps/web` ni en el formato de `project.db`.
