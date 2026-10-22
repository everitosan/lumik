# Design

## Context

Ver proposal.md (Why). Estado actual relevante:

- `start_import` es un solo `invoke` bloqueante. El progreso sale por dos eventos Tauri: `import-progress` (`ImportProgress { current_index, total_files, current_file, phase }`, usado para 3 fases, no archivos) e `import-log` (texto libre en español, sin i18n).
- `ImportPhotos::execute` copia primero los videos (`pipeline_copy_videos`, sin eventos) y luego llama a `ImportPipeline::process_photos`, que encadena `pipeline_passthrough` → `pipeline_metadata` → `pipeline_move_to_dest`. Cada paso corta con `?`: un archivo con problema aborta todo y deja el workspace en `std::env::temp_dir()` (cache de la app en Android).
- `rename_and_embed_metadata` llama a exiftool una vez sobre la carpeta completa. La sesión persistente (`exiftool::run_text`, `-stay_open`) lee stdout hasta `{ready}`, así que no distingue qué archivo falló.
- `move_dng_files` copia y borra (pensado para cruzar filesystems) y `fs::copy` sobrescribe si el destino existe.
- En `ImportPage.tsx` la vista de progreso solo se pinta con `isImporting || result`: si el comando devuelve `Err`, se vuelve a la confirmación y el bloque `importError` y el registro no se ven.
- En Android, el sidecar es `foto.with_extension("xmp")`: viaja con la foto y debe seguir su nombre final.

## Goals / Non-Goals

**Goals:**
- Que el pipeline acumule fallos por archivo y devuelva un resultado parcial en lugar de cortar.
- Eventos de progreso estructurados (fase, archivo, bytes) y traducibles en el frontend.
- Un único sitio de preparación, en el mismo filesystem que `_media/` y `_video/`, para que el paso final sea `rename`.

**Non-Goals:**
- Paralelizar la copia: se mantiene un archivo a la vez.
- Cambiar el formato de `project.db` o el registro EXIF/miniaturas (salvo excluir los fallidos).

## Decisions

### D1. Staging en `{proyecto}/.import-staging/<session_id>/`

`PipelineWorkspace::create` recibe la carpeta del proyecto en lugar de usar `workspace_base_dir()`. Dentro: `photos/` y `videos/`. Al terminar (con éxito o no) se borra `<session_id>/`, y si `.import-staging/` queda vacía también se borra. Un guard con `Drop` cubre las salidas tempranas. Al iniciar un import se borra cualquier `.import-staging/` previa de ese proyecto: el wizard no permite dos importaciones a la vez en el mismo proyecto, así que todo lo que haya ahí es de una importación interrumpida.

Esto ocurre antes del auto-rename de la carpeta del proyecto (`relocate_project_folder`), así que el staging nunca viaja con la carpeta.

*Alternativas:* mantener `temp_dir()` y sumar una validación de su espacio (sigue el techo de tmpfs y la doble copia); staging en `{mount}/lumik/.staging/` (comparte filesystem igual, pero queda fuera del proyecto y es más difícil de asociar y limpiar).

### D2. Videos también por staging

Los videos se copian a `videos/` del staging y luego se hace `rename` a `_video/`. Así un video cortado a mitad nunca queda en `_video/`, y se aplica la misma resolución de nombres (D5). Sin metadatos, igual que hoy.

### D3. Copia por chunks con progreso

Una función `copy_with_progress(src, dst, on_bytes)` sustituye a `fs::copy` en la copia desde el origen: lee y escribe en bloques de 4 MiB y llama al callback tras cada bloque. Si falla, borra el archivo parcial de destino. La copia por `rename` (staging → destino) es instantánea y no reporta bytes.

El reporter limita la emisión de eventos: como máximo uno cada ~100 ms por archivo, más uno siempre al empezar y al terminar cada archivo, para no saturar el IPC con sesiones de miles de fotos.

### D4. Exiftool por archivo dentro de la sesión persistente

`rename_and_embed_metadata` pasa a invocar `run_text` una vez por archivo (incluida la siembra de `DateTimeOriginal<ModifyDate`), con los mismos argumentos, sobre la ruta del archivo en lugar de la carpeta. Cada archivo vive en su propia subcarpeta del staging (`photos/{nnnnn}/`), así que el nombre resultante es el único archivo que queda en ella. Con `-stay_open` cada llamada no lanza un proceso, así que el coste es bajo. A cambio:

- el fallo de un archivo se detecta por el resumen de stdout (`0 image files updated` / `files weren't updated due to errors`) y ese archivo pasa a fallido con `metadata_failed`;
- el nombre resultante se obtiene de la propia llamada (con `-v0` o listando el staging antes y después), sin depender de releer la carpeta entera;
- `%-03c` sigue funcionando: exiftool incrementa el contador si el nombre ya existe en el staging;
- se puede informar el archivo actual también en esta fase (la spec solo exige la fase).

Si exiftool no arranca (no instalado, sesión muerta tras reintento), es un error general que marca como fallidas todas las fotos pendientes con `metadata_failed`; los videos ya copiados se conservan.

Android mantiene su bucle por archivo; solo cambia que un error de `write_xmp_sidecar` o del `rename` marca ese archivo como fallido en lugar de cortar.

*Alternativa:* una sola llamada con `-progress` y parseo de stderr: requiere leer stderr de la sesión `-stay_open` (hoy no se lee) y asociar errores a rutas de texto libre.

### D5. Resolución de nombres al mover

Al pasar cada archivo del staging a `_media/` o `_video/`, si el nombre existe se prueba `{stem}-001.{ext}`, `{stem}-002.{ext}`… hasta el primero libre (función pura testeable, con un predicado `exists`). El sidecar `.xmp` de Android se mueve con el mismo stem final. Un nombre que ya lleva sufijo de ráfaga (`…__Boda-001.cr2`) y choca recibe otro (`…__Boda-001-001.cr2`); se acepta por simplicidad.

El paso es `fs::rename`. Si falla con `EXDEV` (no debería, el staging está en el mismo disco), se recurre a copiar y borrar.

### D6. Fallos tipados y resultado parcial

`FailedFile` gana `code: ImportErrorCode` (`serde` en `snake_case`) y conserva `error` como detalle técnico:

| Código | Cuándo |
|--------|--------|
| `source_unreadable` | no se puede abrir o leer el origen |
| `no_space` | `ErrorKind::StorageFull`, o `raw_os_error` 28 (ENOSPC) o 112 (Windows) |
| `write_failed` | otro error al escribir o renombrar en el destino |
| `metadata_failed` | exiftool o sidecar fallaron para ese archivo |
| `register_failed` | falló el registro en la BD |
| `unknown` | cualquier otro |

`ImportResult` gana `skipped_files: Vec<SkippedFile { name, path, reason: unsupported_format \| not_a_file }>` y `fatal: Option<{ code, detail }>`.

El puerto `ImportPipeline` tiene un solo método `import(PipelineRequest)` para videos y fotos (comparten el staging de la sesión), que siempre devuelve `PipelineOutcome { photos, videos_copied, failed, skipped, bytes_total, fatal }`. Un error general (no se puede crear el staging, `_media/` o `_video/`) va en `fatal`, con lo ya importado. `ImportPhotos::execute` registra las fotos movidas y devuelve `ImportOutcome` sin `Result`. Así `start_import` responde `Ok(ImportResult)` en todo lo que ocurre durante la importación, y reserva `Err` para las precondiciones previas (proyecto no abierto, sin fotógrafo activo).

Con `no_space`, el pipeline no reintenta los archivos siguientes: los marca como fallidos con el mismo código, porque cada intento fallaría igual tras escribir parte del archivo.

### D7. Eventos estructurados

`ImportProgress` se redefine:

```
{ session_id, phase: videos|copy|metadata|move|register|complete,
  file_name, file_index, file_total,
  file_bytes, file_bytes_total, bytes_done, bytes_total }
```

`bytes_total` es la suma de los tamaños de origen de fotos y videos; solo las fases `videos` y `copy` suman bytes. En `metadata`, `move` y `register` la barra queda llena en la parte de copia y el texto indica la fase.

`import-log` deja de llevar texto libre: `ImportLogEntry { session_id, kind, file_name?, code?, count?, detail? }` con `kind` ∈ `file_copied`, `file_failed`, `file_skipped`, `video_copied`, `photos_registered`, `thumbnails`, `thumbnail`, `folder_moved`, `done`. El frontend lo traduce con claves `import.log.<kind>` y los códigos con `import.errors.codes.<code>`. El detalle técnico va a `log::warn!` y al campo `error` de `FailedFile`, que la UI muestra solo como texto secundario.

### D8. Frontend: estados explícitos

`useImport()` expone un estado `idle | importing | done`. `done` contiene `result` (que puede traer `fatal`) o `error` (solo precondiciones). `renderConfirmStep` muestra la vista de progreso/resultado siempre que el estado no sea `idle`, así un `Err` ya no devuelve a la confirmación. Desde `done` el pie muestra "Aceptar" (vuelve al proyecto). El registro se conserva al terminar.

La lista de fallidos muestra los 3 primeros y un botón "Ver todos" que la expande en el mismo panel (hoy solo dice "N más").

### D9. Rendimiento de miniaturas

En una prueba con `tauri dev` (304 CR2, 7.2 GB), las miniaturas tomaron ~7 de los 11 minutos. La causa fue el build debug: decodificar y reducir un preview de 20 MP con `image` tarda ~6.4 s sin optimizar frente a ~86 ms optimizado. Las dependencias se compilan con `opt-level = 3` también en `[profile.dev]`; el release no cambia.

Además, `cache_thumbnail` recibe la rotación ya leída por el EXIF batch (o por la reconciliación al regenerar) y solo la lee del archivo si no se conoce, lo que evita una llamada de exiftool por foto que el mutex de la sesión serializaba entre hilos.

*Descartado:* extraer todos los previews de una tanda en una sola llamada (`-j -b -PreviewImage`). Medido en 32 CR2 reales, empeoró el tiempo (2.61 s frente a 2.15 s): una sesión hace en serie lo que antes eran 8 procesos en paralelo, y el preview viaja como base64 por un pipe de texto.

### D10. Copiar y firmar en una pasada, en paralelo (escritorio)

Tras D9, la prueba real (304 CR2) tardó 4 min 35 s, con ~2 min 26 s en metadatos. Medido: exiftool consume ~0.5 s de CPU por CR2 (Perl, un hilo) aunque escriba en otro archivo, y la copia al USB es casi gratis (~200 MB/s). El límite era el CPU de un solo exiftool, no la E/S.

En escritorio, cuando hay algo que renombrar o firmar, la copia y la firma se **solapan** (fase `copy_metadata`): el hilo del import copia del origen al staging (`slot/src/`) **de a un archivo y de corrido**, y en cuanto una foto está copiada, uno de 4 workers la firma con `exiftool -o slot/ … slot/src/{foto}` dentro del destino y borra la copia sin firmar. Cada worker usa su sesión `-stay_open` de un pool (`exiftool::run_text_in`; se cierran al terminar la fase). Una cola acotada (`SIGN_QUEUE` = 8) hace esperar a la copia si la firma se atrasa, para no ocupar en el destino mucho más que el tamaño del import. Sin nada que escribir se copia tal cual (`copy`), y Android mantiene copiar → sidecar (`metadata`).

Por qué así, medido en este hardware (CR2 de ~24 MB, origen un disco mecánico, destino un USB 3):
- Exiftool escribiendo en paralelo con el origen ya en el USB: 32 CR2 en 16.2 s con 1 hilo, 6.1 s con 4 y 4.5 s con 8.
- Pero con `exiftool -o` leyendo **directamente del origen** en paralelo (primera versión de D10), el disco mecánico se desplomó por los saltos entre 4 lectores: 17.7 s con 4 frente a 6.1 s de la copia secuencial de los mismos 32. En la sesión real el import empeoró de 4 min 35 s a 4 min 55 s.
- Con copia secuencial y firma solapada, 64 CR2 tardan 26.0 s frente a 24.7 s de copiar sin firmar: la firma queda casi del todo detrás de la copia.

- El respaldo de fecha de los TIFF (antes, una llamada que escribía `DateTimeOriginal<ModifyDate` en el staging) va en la misma llamada: `-DateTimeOriginal<ModifyDate -DateTimeOriginal<DateTimeOriginal` y lo mismo para `FileName`, porque en exiftool gana la última asignación válida.
- El progreso de bytes sale de la copia secuencial, exacto. "Copiada" se informa al terminar la copia; si la firma falla después, el archivo pasa a fallido sin volver a contar sus bytes.
- Un fallo al copiar trae su código de E/S (`source_unreadable`, `no_space`…; tras un `no_space`, los pendientes fallan sin intentarse). Si falla la firma, una copia de prueba dentro del destino distingue `no_space` de `metadata_failed`. El slot queda vacío.
- Los resultados se devuelven en el orden de la selección, así la ráfaga sigue recibiendo `-001` en el mismo archivo.

*Descartado:* `exiftool -o` leyendo del origen en paralelo (ver arriba), y más de 4 workers de firma (con la firma ya detrás de la copia no aportan).

### D11. Sesiones de exiftool sin huérfanos

Exiftool con `-stay_open True -@ -` **no termina al cerrarse su stdin** (comprobado: sigue vivo 5 s después). La sesión principal vive en una `static` que nunca se destruye, así que cada salida de la app o de `cargo test` dejaba un Perl dormido (~20 MB); se habían acumulado 15. Esto ya pasaba antes de este change; el pool de D10 lo habría multiplicado si no se cerrara al final de la fase.

- `exiftool::shutdown()` cierra la sesión principal y el pool en `RunEvent::Exit` (con `try_lock`, para no bloquear la salida).
- En Linux, cada sesión se lanza con `PR_SET_PDEATHSIG = SIGTERM`, para que el kernel la termine si Lumik muere por cualquier causa (Ctrl+C, crash, fin de los tests). Como esa señal se dispara cuando termina el **hilo** que lanzó el proceso, y las sesiones se crean desde workers y hilos de tokio que terminan, se lanzan desde un hilo `exiftool-spawner` que vive mientras viva el proceso.
- Windows y macOS: solo la salida normal. Un Job object (Windows) cubriría también los crashes; queda fuera de alcance.

## Risks / Trade-offs

- [Con firma activa, el archivo se escribe ya firmado sobre el disco destino (USB)] → Resuelto con D10: una sola escritura por archivo y exiftool en paralelo. Medido en la sesión real: de 11 min a 4 min 35 s (D9), y D10 elimina la mayor parte de los metadatos restantes.
- [Con un origen muy rápido (NVMe) la copia deja de ser el límite y manda la firma: ~0.5 s de CPU por RAW entre 4 workers] → Aun así, más rápido que antes. `SIGN_WORKERS` es una constante si hiciera falta subirlo.
- [Detectar el fallo por el texto de resumen de exiftool depende de su formato de salida] → Fijarlo con un test de caracterización sobre los fixtures, uno con un archivo dañado.
- [Si la app se cierra a mitad, queda `.import-staging/` en el disco del proyecto hasta la siguiente importación] → Es una carpeta oculta que nadie lista. La limpieza al iniciar el import la elimina.
- [Sufijo en vez de rechazo: importar dos veces la misma tarjeta duplica fotos] → Aceptado; la detección de duplicados queda fuera de alcance.
- [Cambia el contrato de los eventos `import-progress` / `import-log` y de `ImportResult`] → Solo los consume `useImport()` en la misma app; se cambian juntos.

## Migration Plan

Sin migración de datos. Las importaciones previas no dejaron staging en el proyecto. Los workspaces viejos en la carpeta temporal del sistema no se tocan (el sistema operativo los limpia).
