# Proposal

## Why

Al importar, el paso 3 solo muestra tres fases gruesas ("1 / 3 archivos" cuenta fases, no archivos) y los videos se copian sin ningún evento, así que una sesión grande o un video de varios GB parecen colgados. Cuando algo falla, el error se guarda pero la vista vuelve a la confirmación con el botón "Importar" y sin el registro, y un solo archivo con problema cancela la importación completa. Además, las fotos pasan por un workspace en la carpeta temporal del sistema (en Debian 13 es tmpfs, limitado por la RAM), que puede llenarse aunque el disco destino tenga espacio, y al moverlas a `_media/` se sobrescribe sin avisar un archivo con el mismo nombre.

## What Changes

- **Progreso por archivo y por bytes**: el import informa el archivo actual, su número dentro del total y los bytes copiados del archivo y de toda la importación, para fotos y videos. La barra avanza según los bytes totales y la fase se muestra como texto. La fase de metadatos se muestra sin porcentaje.
- **Fallos por archivo sin cancelar el lote**: si un archivo no se puede copiar, preparar o mover, se registra como fallido con su motivo y el import sigue con los demás. Solo un problema que impide importar cualquier archivo (p. ej. no se puede crear la carpeta del proyecto) termina el import, y aun así se informa qué quedó importado.
- **Resultado siempre visible**: al terminar, el paso 3 muestra el resultado (completo, parcial o fallido), con los motivos traducidos y el registro de actividad, en lugar de volver a la confirmación. Los archivos ignorados por formato también se informan.
- **Staging en el disco destino**: la preparación (renombrado y metadatos) ocurre en una carpeta oculta dentro del proyecto en el disco destino, no en la carpeta temporal del sistema. La validación de espacio del paso 3 pasa a cubrir todo lo que la importación necesita. Lo que quede de una importación interrumpida se limpia al iniciar la siguiente en ese proyecto.
- **Sin sobrescritura**: si el nombre final de una foto o video ya existe en `_media/` o `_video/`, se le agrega un sufijo `-001`, `-002`… y se importa igual.
- Los mensajes de progreso y de error pasan por i18n (es/en).

### Fuera de alcance

- Cancelar una importación en curso.
- Detectar duplicados reales (misma foto importada dos veces) por hash o por fecha y tamaño.
- Progreso por archivo dentro de la fase de metadatos.

## Capabilities

### New Capabilities

_Ninguna._

### Modified Capabilities

- `photo-import`: "Progreso del import" pasa a ser por archivo y por bytes, con resultado siempre visible y motivos traducidos. Nuevos requisitos: fallos por archivo sin cancelar el lote, preparación en el disco destino (espacio y limpieza de restos) y nombres sin sobrescritura. "Espacio suficiente antes de importar" y "Copia sin conversión" se ajustan a lo anterior.

## Impact

- `apps/desktop/src-tauri/src/import/pipeline.rs`: workspace en `{proyecto}/.import-staging/<sesión>/`, copia por chunks con callback de progreso, errores por archivo, `fs::rename` a `_media/` con resolución de nombres, copia de videos con progreso y fallos por archivo.
- `apps/desktop/src-tauri/src/import/progress.rs`: `ImportProgress` con fase, archivo y bytes; `FailedFile` con código de error.
- `apps/desktop/src-tauri/src/application/ports.rs`, `infrastructure/import_pipeline.rs`, `application/use_cases/import_photos.rs` y `commands/import_cmd.rs`: el puerto `ImportPipeline` devuelve resultados parciales y fallos en vez de cortar con `?`.
- Frontend: `src/lib/types.ts`, `useImport()` en `src/lib/hooks.ts` y `pages/ImportPage.tsx` (vista de progreso y de resultado), con textos en `src/i18n/locales/es.json` y `en.json`.
- Documentación: `docs/useCases/flujo-importar-imagenes.md` y la sección "Pipeline de importación" de `apps/desktop/CLAUDE.md`.
- Sin cambios en `apps/server`, `apps/web` ni en el formato de `project.db`.
