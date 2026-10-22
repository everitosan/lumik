# Tasks

## 1. Tipos de progreso y de resultado

- [x] 1.1 En `src-tauri/src/import/progress.rs`, redefinir `ImportPhase` (`videos`, `copy`, `metadata`, `move`, `register`, `complete`) e `ImportProgress` con archivo y bytes (D7); agregar `ImportErrorCode`, `code` en `FailedFile`, `SkippedFile`, `skipped_files` y `fatal` en `ImportResult` (D6); cambiar `ImportLogEntry` a `{ kind, file_name?, code?, count? }`. Agregar una función pura `error_code_for(io::Error)` que mapea `StorageFull`/ENOSPC/112 a `no_space`, `NotFound`/`PermissionDenied` al leer a `source_unreadable`, etc. Verificar con tests unitarios del mapeo y del JSON serializado (`snake_case`) y `cargo test`.
- [x] 1.2 Espejar los tipos en `src/lib/types.ts`. Verificar con `pnpm exec tsc --noEmit` en `apps/desktop` (los errores de uso en `ImportPage`/`useImport` se resuelven en el grupo 4).

## 2. Pipeline: staging, copia con progreso, nombres

- [x] 2.1 `PipelineWorkspace::create(project_dir, session_id)` en `{proyecto}/.import-staging/<session_id>/{photos,videos}`, con limpieza previa de `.import-staging/` y un guard `Drop` que borra la sesión (y la carpeta raíz si queda vacía) (D1). Quitar `workspace_base_dir()`. Verificar con tests sobre un `tempdir`: restos previos se eliminan, el guard limpia ante un `Err` temprano.
- [x] 2.2 `copy_with_progress(src, dst, on_bytes)` por bloques de 4 MiB que borra el parcial si falla (D3). Verificar con un test que copia un archivo de varios bloques, cuenta los callbacks y comprueba que un destino no escribible no deja archivo.
- [x] 2.3 Función pura `free_name(dir, file_name, exists)` → `{stem}-001.{ext}`, `-002`… y `move_into(staging_file, dest_dir)` con `fs::rename` (respaldo copiar+borrar si `EXDEV`) que mueve también el `.xmp` hermano con el mismo stem (D5). Verificar con tests: sin choque, choque simple, choque con nombre de ráfaga, sidecar acompañando.
- [x] 2.4 Reescribir `pipeline_passthrough`, `pipeline_copy_videos` y `pipeline_move_to_dest` para acumular `imported`/`failed`/`skipped` por archivo en vez de cortar con `?`, cortando solo ante errores generales y marcando como `no_space` los pendientes tras un `no_space` (D6). Verificar con tests sobre `tempdir`: un origen inexistente en medio del lote queda como fallido y los demás se importan; un `.png` y un directorio quedan como `skipped`.
- [x] 2.5 Exiftool por archivo en `rename_and_embed_metadata` usando la sesión persistente, detectando el fallo por el resumen de stdout y obteniendo el nombre resultante (D4); en Android, convertir los errores de `write_xmp_sidecar`/`rename` en fallos por archivo. Verificar con un test de caracterización sobre `tests/fixtures/` que incluye un archivo truncado: los JPEG válidos se renombran con `%-03c` como hoy y el truncado queda como `metadata_failed`. *(Ajuste: cada archivo va en su propia subcarpeta del staging, así el nombre que deja exiftool es el único archivo de la carpeta y dos orígenes con el mismo nombre no se pisan; la ráfaga recibe `-001` al mover, con `free_name`, igual que hacía `%-03c`.)*

## 3. Caso de uso y comando

- [x] 3.1 Cambiar el puerto `ImportPipeline` (`application/ports.rs`) para que `process_photos` y `copy_videos` devuelvan `PipelineOutcome` y reciban el reporter; adaptar `StdImportPipeline` emitiendo `ImportProgress` con fase, archivo y bytes, con el límite de ~100 ms por archivo (D3, D7). Verificar con `cargo test` y un test del limitador de eventos con un reloj inyectado. *(Ajuste: un solo método `ImportPipeline::import` para videos y fotos, que siempre devuelve `PipelineOutcome` con `fatal` opcional: comparten el staging de la sesión y un error general debe conservar los videos ya copiados. Compila para `aarch64-linux-android`; para Windows no se pudo compilar aquí, falta el toolchain de mingw para `ring`.)*
- [x] 3.2 En `ImportPhotos::execute`: pasar `project_dir` al pipeline, sumar `bytes_total`, convertir el `Err` general en `fatal` conservando lo importado, registrar en BD y generar miniaturas solo de las fotos importadas, y propagar `failed`/`skipped` (D6). Actualizar los fakes y agregar tests: lote con un fallo de copia → `successful = n-1` y un `FailedFile`; fallo general tras copiar videos → `fatal` presente y `videos_copied` conservado.
- [x] 3.3 En `commands/import_cmd.rs`, devolver siempre `Ok(ImportResult)` durante la importación (reservar `Err` para precondiciones), y emitir las entradas de log estructuradas (`folder_moved`, `done`). Verificar con `cargo check` y `cargo test`.

## 4. Frontend

- [x] 4.1 `useImport()` con estado `idle | importing | done`, conservando el registro estructurado al terminar (D8). Verificar con `pnpm exec tsc --noEmit`.
- [x] 4.3 Agregar a `src/i18n/locales/es.json` y `en.json` las claves `import.progress.phases.*`, `import.log.*`, `import.errors.codes.*` e `import.summary.skipped*`, y quitar las que ya no se usen. Verificar con `jq empty` (o Python si no hay `jq`) y que no hay claves duplicadas. *(Verificado con Python: JSON válidos, sin duplicados, y todas las claves que usa `ImportPage` existen en los dos idiomas.)*

## 5. Documentación

- [x] 5.1 Actualizar `docs/useCases/flujo-importar-imagenes.md` con el staging en el proyecto, el flujo por archivo y las ramas de fallo por archivo y error general. Verificar que el diagrama Mermaid renderiza.
- [x] 5.2 Actualizar la sección "Pipeline de importación" de `apps/desktop/CLAUDE.md` (staging en `.import-staging/`, progreso por archivo y bytes, fallos por archivo, sin sobrescritura) y la carpeta `.import-staging/` en la estructura de disco. Verificar que coincide con el código.


## 7. Rendimiento (tras la prueba de 304 fotos en 11 min)

- [x] 7.1 Compilar las dependencias optimizadas en `[profile.dev]` (`[profile.dev.package."*"] opt-level = 3`) para que `tauri dev` sea representativo (D9). Verificar midiendo el decodificado y la reducción de un preview real de 20 MP: ~6.4 s en debug frente a ~86 ms en release. *(Medido: decode 4.9 s → 49 ms, resize 1.5 s → 36 ms.)*
- [x] 7.2 Reusar la rotación del EXIF batch al generar miniaturas (`ImageProcessor::cache_thumbnail(src, id, rotation: Option<i32>)`), en el import y al regenerar, en vez de una lectura de exiftool por foto serializada por el mutex de la sesión (D9). Verificar con un test de que la rotación conocida se respeta, y midiendo 32 CR2 reales. *(Medido: 2.15 s → 1.85 s. El lote de previews en una sola llamada de exiftool se descartó: empeoraba a 2.61 s, porque serializa en una sesión lo que antes eran 8 procesos en paralelo.)*

- [x] 7.3 En escritorio, solapar copia y firma: copia secuencial del origen al staging y 4 workers con su sesión de exiftool del pool (`exiftool.rs`) firmando con `exiftool -o` dentro del destino, con cola acotada (D10). Verificar con el test de caracterización (renombrado, origen intacto, truncado → `metadata_failed`, inexistente → `source_unreadable`, orden conservado, ráfaga `-001`, firma fallida sin doble conteo de bytes) y midiendo CR2 reales desde el disco mecánico. *(Primera versión con `exiftool -o` leyendo del origen en paralelo: la sesión real empeoró a 4 min 55 s porque el disco mecánico se desploma con 4 lectores. Versión solapada: 64 CR2 en 26.0 s frente a 24.7 s de solo copiar. Android conserva copiar → sidecar.)*

- [x] 7.4 Que no queden procesos de exiftool huérfanos: cerrar todas las sesiones en `RunEvent::Exit` (`exiftool::shutdown()`) y, en Linux, lanzarlas con `PR_SET_PDEATHSIG` desde un hilo dedicado de vida igual a la del proceso (D11). Verificar contando los `exiftool -stay_open` cuyo padre es systemd tras `cargo test` y tras reiniciar la app. *(Antes había 15 huérfanos acumulados desde las 12:38, uno por cada salida de la app o de `cargo test`; después de `cargo test`, 0. La rama de Windows no se pudo compilar aquí; en Windows y macOS solo cubre la salida normal.)*

## 8. Pruebas manuales pendientes (antes de archivar)

Quedan fuera de la rc-v0.1.5-beta y se hacen cuando haya con qué probarlas: un video de varios GB, Windows y Android. El change no se archiva hasta marcarlas.

- [ ] 8.1 (era 4.2) *(Código hecho y `tsc` limpio; falta la comprobación a mano.)* `ImportPage.tsx`: vista de progreso con fase, archivo actual (`42 de 400`), bytes del archivo y barra por bytes totales; vista de resultado para completo, parcial, `fatal` y error de precondición; lista de fallidos con motivo traducido y "Ver todos"; lista de ignorados. Verificar con `pnpm exec tsc --noEmit` y a mano con `pnpm tauri dev` que un `Err` simulado ya no vuelve a la confirmación.
- [ ] 8.2 (era 6.1) En Linux con `pnpm tauri dev` y un disco USB: importar una sesión real de RAW con firma activa y un video de varios GB. Comprobar que se ven el archivo y los bytes avanzando en todo momento, que no se crea nada en `/tmp`, que no queda `.import-staging/` al terminar, y anotar el tiempo frente a la versión actual (riesgo de D4/exiftool en USB).
- [ ] 8.3 (era 6.2) Forzar fallos: un archivo sin permiso de lectura en el origen, reimportar los mismos archivos con el renombrado desactivado (deben quedar con `-001`), y cerrar la app a mitad de una importación y volver a importar (los restos se limpian y no aparecen en el grid). Comprobar que el resultado muestra cada caso con su motivo en español y en inglés.
- [ ] 8.4 (era 6.3) Repetir 8.2 de forma reducida en Windows y en Android (sidecar `.xmp` con el nombre final, staging en el almacenamiento del proyecto), y correr `openspec validate import-feedback-and-errors --strict`.

## Workflow follow-up

- Archivar el change con `/opsx:archive` cuando las pruebas del grupo 8 estén hechas.
