# Flujo: Importar Imágenes a un Proyecto

Lumik Desktop · Linux · Windows · Android

Wizard de 3 pasos en el frontend (origen, ajustes, confirmar; solo desde un proyecto) y un pipeline en Rust que procesa
**archivo por archivo**: copia de videos y fotos a un staging dentro del proyecto
(`.import-staging/`, mismo disco que el destino), metadatos y renombrado, `rename` a `_video/` y `_media/` sin
sobrescribir, extracción EXIF e inserción en `project.db`, y generación de miniaturas. Un archivo que falla se
registra con su motivo y el resto sigue; el resultado se muestra siempre, y desde ahí se pueden reintentar
los fallidos (§7).

Un RAW y un JPG de la misma toma (mismo nombre de cámara sin extensión y misma fecha de captura al
segundo) se registran como **una sola foto, un par**, aunque vengan de carpetas o tarjetas distintas o
en imports separados (§3b). El import cuenta tomas, no archivos.

> **Splits por plataforma:** en los diagramas que se bifurcan, la rama izquierda es
> **Linux / Windows** (vía `exiftool`) y la derecha **Android** (Rust puro, sin `exiftool`).

## Vista general

```mermaid
flowchart LR
    W["Wizard<br/>3 pasos"]
    I["invoke<br/>'start_import'"]
    P["Particionar<br/>videos / fotos"]
    C["Staging en el proyecto<br/>+ copia por archivo"]
    M["Metadatos<br/>+ renombrado"]
    G["Agrupar en tomas<br/>(pares RAW+JPG, uniones)"]
    D["rename a _media/<br/>(nombre libre por toma)"]
    X["EXIF → DTOs"]
    B["create_photos_batch()"]
    T["Miniaturas<br/>.thumbs/"]
    R["ImportResult"]

    W --> I --> P --> C --> M --> G --> D --> X --> B --> T --> R

    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef ipc fill:#c8e6c9,stroke:#388e3c,color:#10281a
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    class W ui
    class I,R ipc
    class P,C,M,G,D,X,T be
    class B db
```

## 1. Wizard de importación (frontend)

```mermaid
flowchart TD
    start(["Usuario en Detalle de proyecto<br/>pulsa Importar (el proyecto es el destino)"])
    s1["Paso 1 — Origen: seleccionar archivos<br/>• Clic en 'Browse' o arrastrar archivos (DropZone)<br/>• Validar extensiones: RAW, DNG, JPEG, video (MP4, MOV…)<br/>• Mostrar FileList con tamaño total acumulado<br/>• Desglose por tomas (analyze_import_selection):<br/>pares RAW+JPG, solo RAW, solo JPG y cuántos se unirán"]
    s2["Paso 2 — Ajustes de esta importación<br/>• Precargados de AppSettings + metadatos del fotógrafo<br/>• Renombrar sí/no (ejemplo con el nombre del proyecto)<br/>• Firmar sí/no + autor, copyright, URL editables<br/>• Restablecer a la configuración global<br/>• No modifica la configuración global<br/>Bloque 'De este proyecto': entrega al aprobar<br/>(se guarda en el proyecto, con su aviso)"]
    s3["Paso 3 — Confirmar e iniciar<br/>• Resumen: N fotos (por toma) + M videos, proyecto, disco,<br/>espacio libre, renombrado, entrega y firma elegidos<br/>• Botón desactivado si no hay espacio suficiente<br/>• Usuario confirma → genera session_id único"]
    invoke["api.startImport()<br/>listen('import-progress') + invoke('start_import', ImportRequest)<br/>{ session_id, source_files[], project_id,<br/>device_uuid (ignorado en Rust), mount_point,<br/>project_name, settings? }"]
    next(["continúa en §2"])

    n_uri["Android: DocumentsUI devuelve<br/>content://… URIs<br/>→ resolveContentUri()<br/>→ /storage/{UUID}/DCIM/…"]
    n_set["settings = ImportSessionSettings<br/>{ rename_on_import, embed_metadata,<br/>artist, copyright, contact_url }"]

    start --> s1 --> s2 --> s3 --> invoke --> next
    s1 -.- n_uri
    s2 -.- n_set

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef ipc fill:#c8e6c9,stroke:#388e3c,color:#10281a
    classDef note fill:#fff9c4,stroke:#f9a825,color:#3d3300
    class start,next term
    class s1,s2,s3 ui
    class invoke ipc
    class n_uri,n_set note
```

## 2. Arranque del backend, staging y videos

Fotos y videos comparten el staging de la sesión. Los videos se copian primero, sin
conversión, sin XMP y sin registro en base de datos.

```mermaid
flowchart TD
    be_load["Cargar ProjectDatabase por project_id<br/>+ AppSettings + metadatos del fotógrafo<br/>→ resolve_import_settings(globales, settings?)<br/>(sin settings = globales; con settings = los de la sesión)<br/>+ descripción del proyecto"]
    pre_err(["Err del comando (precondición)<br/>proyecto no abierto, sin fotógrafo<br/>→ la UI muestra 'No se pudo iniciar'"])
    part["Particionar archivos<br/>videos vs fotos"]
    ws{"PipelineWorkspace::create()<br/>borra .import-staging/ previa (import interrumpido)<br/>crea {proyecto}/.import-staging/{session}/photos|videos"}
    fatal(["fatal en ImportResult<br/>(no_space / write_failed)"])
    video_stage["pipeline_copy_videos()<br/>por archivo → videos/{nnnnn}/{nombre}<br/>copy_with_progress: bloques de 4 MiB → eventos de bytes"]
    video_q{"¿copia OK?"}
    video_fail["FailedFile { code, error }<br/>sin archivo parcial · sigue con el siguiente<br/>(tras no_space, los pendientes fallan igual)"]
    video_move["pipeline_move_to_dest(_video/)<br/>rename con free_name(): CLIP.mp4 → CLIP-001.mp4 si existe"]
    next(["continúa en §3 (fotos)"])

    be_load -->|error| pre_err
    be_load --> part --> ws
    ws -->|no se pudo crear| fatal
    ws --> video_stage --> video_q
    video_q -->|sí| video_move
    video_q -->|no| video_fail --> video_move
    video_move --> next

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    classDef err fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    class next term
    class be_load,part,video_stage,video_move be
    class ws,video_q dec
    class pre_err,fatal,video_fail err
```

## 3. Fotos: copia, metadatos y movimiento a destino

```mermaid
flowchart TD
    classify["Clasificar selección<br/>no es archivo → skipped(not_a_file)<br/>extensión no admitida → skipped(unsupported_format)"]
    plat_q{"Plataforma y<br/>¿hay algo que escribir?"}
    signed["Linux / Windows / macOS — pipeline_stage_signed() · fase copy_metadata<br/>copia y firma SOLAPADAS:<br/>• hilo del import: copia secuencial origen → photos/{nnnnn}/src/ (bytes exactos)<br/>• 4 workers (sesión -stay_open del pool): exiftool -o photos/{nnnnn}/ … src/{foto}<br/>• cola acotada (8): la copia espera si la firma se atrasa<br/>• Rename: {DateTimeOrig}__{proyecto}%-03c.{ext} (respaldo ModifyDate)<br/>• Embed: Artist, Copyright, ImageDescription@Año,<br/>CreatorURL, CreatorEmail, UsageTerms<br/>• Resultados en el orden de la selección"]
    passthrough["pipeline_passthrough() · fase copy<br/>(sin nada que escribir, o Android)<br/>por archivo → photos/{nnnnn}/{nombre}, formato original<br/>copy_with_progress: bloques de 4 MiB"]
    meta_android["Android · fase metadata<br/>rawler → extract_datetime → rename<br/>• Rename: {DateTimeOrig}__{proyecto}.{ext}<br/>• Sidecar .xmp: Artist, Copyright,<br/>ImageDescription@Año, CreatorURL, CreatorEmail<br/>(Rust puro, sin exiftool)"]
    copy_fail["FailedFile por archivo, el resto sigue<br/>exiftool falla → copia de prueba decide:<br/>source_unreadable · no_space · write_failed · metadata_failed"]

    move["Agrupar en tomas (§3b) y pipeline_move_shots(_media/)<br/>fs::rename (mismo disco; copia+borrado si EXDEV)<br/>suelto: free_name() → {stem}-001.{ext}, -002…<br/>par: free_stem() → el mismo nombre libre para RAW y JPG<br/>(libre en _media/, _culled/ y _exported/)<br/>unión: junto al archivo de la foto existente<br/>el sidecar .xmp viaja con el mismo stem"]
    cleanup["Drop de PipelineWorkspace<br/>borra .import-staging/{session} (y la raíz si queda vacía)<br/>también ante salidas tempranas"]
    next(["continúa en §4"])

    classify --> plat_q
    plat_q -->|Escritorio, firmar/renombrar| signed --> move
    plat_q -->|Nada que escribir| passthrough --> move
    plat_q -->|Android| passthrough
    passthrough -->|Android| meta_android --> move
    signed -.->|por archivo| copy_fail
    passthrough -.->|por archivo| copy_fail
    meta_android -.-> copy_fail
    move --> cleanup --> next

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef plat fill:#e1d5e7,stroke:#9673a6,color:#2a1a33
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    classDef err fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    class next term
    class classify,passthrough,move,cleanup be
    class signed,meta_android plat
    class plat_q dec
    class copy_fail err
```

## 3b. Pares RAW+JPG y uniones

```mermaid
flowchart TD
    staged["Archivos en el staging<br/>(firmados y renombrados por archivo)"]
    dates["MetadataTool::capture_dates()<br/>exiftool -fast2 en lote (Android: rawler)"]
    key["Clave de toma = nombre de cámara del ORIGEN<br/>sin extensión (minúsculas) + fecha al segundo"]
    group{"group_shots():<br/>¿exactamente 1 RAW + 1 JPEG<br/>con la misma clave?"}
    pair["StagedShot::Pair<br/>→ una foto: dng_path = RAW,<br/>camera_jpg_path = JPG, source_stem"]
    single{"¿Completa una foto suelta del proyecto?<br/>(loose_photos: misma clave, tipo contrario)"}
    join["StagedShot::Join<br/>→ junto al archivo existente, con su nombre<br/>join_partner(): la foto pasa a ser par<br/>(conserva estrellas, color, tags, aprobación)"]
    alone["StagedShot::Single<br/>→ foto suelta"]
    approved{"¿La foto ya estaba<br/>aprobada?"}
    deliver["CullPhoto::deliver_joined()<br/>entrega el JPG según la opción del proyecto"]
    thumb["Miniatura y vista previa<br/>desde el JPG del par"]

    staged --> dates --> key --> group
    group -->|Sí| pair --> thumb
    group -->|No| single
    single -->|Sí| join --> approved
    approved -->|Sí| deliver --> thumb
    approved -->|No| thumb
    single -->|No| alone

    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    class staged,dates,key,pair,alone,deliver,thumb be
    class join db
    class group,single,approved dec
```

El reintento de fallidos por copia vuelve a `start_import`, así que un JPG que falló se une a su RAW
por la misma regla. `register_existing` (fallidos al registrar, ya en `_media/`) empareja por el
nombre final y no une con fotos existentes.

## 4. Extracción EXIF e inserción en base de datos

```mermaid
flowchart TD
    plat_q{"Plataforma"}
    exif_desktop["Linux / Windows / macOS<br/>exiftool -csv batch sobre todos los archivos<br/>Extrae: Width, Height, DateTimeOriginal, Make, Model,<br/>ISO, FNumber, ExposureTime, FocalLength, LensModel,<br/>IFD0:Orientation → rotation°<br/>capture_date: DateTimeOriginal → CreateDate → ModifyDate<br/>Headers parseados dinámicamente — exiftool omite<br/>columnas vacías del batch → HashMap"]
    exif_android["Android<br/>exif_android::extract_exif_metadata_batch()<br/>RAW: crate rawler → RawMetadata.exif.orientation → rotation°<br/>JPEG: crate kamadak-exif → tag Orientation → rotation°<br/>(rawler no soporta JPEGs de cámaras de teléfono)<br/>→ HashMap"]
    merge_exif["HashMap (incl. rotation) disponible<br/>para todos los archivos"]
    dtos["Construir CreatePhoto DTOs<br/>dng_path relativo a la carpeta del proyecto (_media/…) ·<br/>original_format · capture_date · dimensiones ·<br/>ISO · apertura · velocidad · longitud focal · lente · rotation"]
    insert["project_db.create_photos_batch() — transacción SQLite única<br/>• Genera UUID por foto<br/>• INSERT INTO photo (id, project_id, dng_path,<br/>device_uuid = '' (obsoleto: las rutas se resuelven<br/>en runtime con la carpeta del proyecto), original_camera, original_format, capture_date, width, height,<br/>file_size_bytes, iso, aperture, shutter_speed, focal_length,<br/>lens_model, rotation)<br/>• Commit en éxito · Rollback en error, sin inserciones parciales"]
    next(["continúa en §5"])

    plat_q -->|Escritorio| exif_desktop --> merge_exif
    plat_q -->|Android| exif_android --> merge_exif
    merge_exif --> dtos --> insert --> next

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    classDef plat fill:#e1d5e7,stroke:#9673a6,color:#2a1a33
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    class next term
    class merge_exif,dtos be
    class insert db
    class exif_desktop,exif_android plat
    class plat_q dec
```

## 5. Miniaturas y resultado

Las miniaturas se generan en paralelo, con un máximo de 8 hilos; cada una emite progreso
(fase `register`, N de M) y una entrada `thumbnail` en el registro. Si falla la inserción en
la BD, todas las fotos del lote se informan como fallidas (`register_failed`).

```mermaid
flowchart TD
    plat_q{"Plataforma"}
    thumb_desktop["Linux / Windows — cache_thumbnail()<br/>RAW/JPEG: exiftool -b -PreviewImage<br/>(fallback -ThumbnailImage)<br/>TIFF: image::open() → si falla por JPEG strips,<br/>open_jpeg_tiff() via crate tiff 0.11 + zune-jpeg (Rust puro)<br/>→ image crate: resize 320×320, rotación EXIF aplicada<br/>→ encode JPEG → .thumbs/{id}.jpg"]
    thumb_android["Android — exif_android::cache_thumbnail()<br/>RAW: rawler::extract_thumbnail_pixels()<br/>+ resize + rotación física<br/>JPEG: image::open() + resize<br/>+ rotación via kamadak-exif<br/>→ encode JPEG → .thumbs/{id}.jpg"]
    merge_thumb["Miniaturas JPEG en {project}/.thumbs/{photo_id}.jpg<br/>para todas las fotos importadas"]
    result["Retornar Ok(ImportResult) al frontend, también si falló algo<br/>{ total_files, successful, failed, failed_files[] (code + error),<br/>skipped_files[], videos_copied, fatal? }"]
    show["Paso 3 muestra siempre el resultado (nunca vuelve a la confirmación)<br/>• Completo · parcial · fallido (fatal)<br/>• N fotos importadas + M videos copiados + tiempo<br/>• Fallidos con motivo traducido (3 + 'Ver todos') e ignorados<br/>• Registro de actividad conservado"]
    fin(["Fotos visibles en galería del proyecto<br/>Miniaturas en caché · rotation en BD"])

    plat_q -->|Linux/Win| thumb_desktop --> merge_thumb
    plat_q -->|Android| thumb_android --> merge_thumb
    merge_thumb --> result --> show --> fin

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef ipc fill:#c8e6c9,stroke:#388e3c,color:#10281a
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    classDef plat fill:#e1d5e7,stroke:#9673a6,color:#2a1a33
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    class fin term
    class show ui
    class result ipc
    class merge_thumb db
    class thumb_desktop,thumb_android plat
    class plat_q dec
```

## 6. Eventos de progreso

El backend emite dos eventos, filtrados por `session_id` en `useImport()`:

- `import-progress` — `ImportProgress { phase, file_name, file_index, file_total, file_bytes,
  file_bytes_total, bytes_done, bytes_total }`. Granularidad **por archivo**; los bytes avanzan
  en `videos`, `copy` y `copy_metadata` (siempre por la copia secuencial; como máximo un evento cada ~100 ms por archivo, más uno al empezar y
  al terminar cada archivo). La barra de la UI sigue `bytes_done / bytes_total`.
- `import-log` — `ImportLogEntry { kind, file_name?, code?, count?, detail? }`, traducido en el
  frontend con `import.log.<kind>` e `import.errors.codes.<code>`.

```mermaid
flowchart LR
    p0["videos<br/>por archivo + bytes"] --> p1["copy_metadata (escritorio)<br/>o copy<br/>por archivo + bytes"] --> p2["metadata<br/>solo Android"] --> p3["move<br/>por archivo"] --> p4["register<br/>miniaturas N/M"] --> p5["complete"]

    classDef note fill:#fff9c4,stroke:#f9a825,color:#3d3300
    class p0,p1,p2,p3,p4,p5 note
```

Códigos de error (`ImportErrorCode`): `source_unreadable`, `no_space`, `write_failed`,
`metadata_failed`, `register_failed`, `unknown`.

## 7. Reintentar fallidos

Desde el resultado, "Reintentar (N)" procesa solo los fallidos, en el mismo proyecto y con los mismos
ajustes de la sesión. El resultado y el registro se acumulan: tras cada reintento muestran el total de la
sesión, y si algo vuelve a fallar el botón sigue disponible.

```mermaid
flowchart TD
    result(["Resultado con failed_files[]"])
    split{"Separar por code"}
    space{"¿Cabe en el disco?<br/>(suma de sizeBytes de esos orígenes<br/>vs espacio libre, releído al terminar)"}
    disabled(["Botón desactivado<br/>+ aviso del espacio que falta"])
    reg["1º · retry_import_register({ session_id nuevo, file_names })<br/>ImportPhotos::register_existing():<br/>• no está en _media/ → sigue como register_failed<br/>• dng_path ya en la BD (registered_dng_paths) → cuenta como importada<br/>• resto → EXIF batch + create_photos_batch + miniaturas<br/>sin copiar, sin borrar, sin sufijos -001"]
    imp["2º · start_import({ session_id nuevo,<br/>source_files = rutas de origen de esos fallidos,<br/>mismos settings }) — §2–§5"]
    fin["finish_import(): fecha de sesión inferida y renombrado de carpeta<br/>(también tras el reintento de registro)"]
    merge["mergeRetryResult(prev, reintentados, next)<br/>successful y videos_copied se suman · fallidos = no reintentados + los que volvieron a fallar<br/>ignorados y total_files del primer intento"]
    err(["Err de un comando (precondición)<br/>se conserva lo ya fusionado<br/>→ 'No se pudo reintentar'"])

    result --> split
    split -->|register_failed<br/>ya en _media/| reg
    split -->|resto: copia, firma, no_space…| space
    space -->|no| disabled
    space -->|sí| imp
    reg --> fin
    imp --> fin
    fin --> merge --> result
    reg -.->|Err| err
    imp -.->|Err| err

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    classDef err fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    class result term
    class merge ui
    class reg,imp,fin be
    class split,space dec
    class disabled,err err
```

## Archivos de código

| Fase | Archivos |
|------|----------|
| Wizard (§1) | `src/pages/ImportPage.tsx` — wizard 3 pasos (origen, ajustes de la importación, confirmar), DropZone, FileList, `handleBrowse()`, `resolveContentUri()` (Android); auto-muestra importación si el proyecto está vacío |
| IPC (§1, §7) | `src/lib/api.ts` — `startImport()`, `retryImportRegister()`, `listen('import-progress')` · `src/lib/hooks.ts` — `useImport()`, `session_id` |
| Arranque (§2) | `src-tauri/src/commands/import_cmd.rs` — `start_import()`, particionado, `image_description = {desc}@{year}` · `src-tauri/src/application/import_settings.rs` — `resolve_import_settings()` (ajustes de la sesión contra los globales) |
| Staging (§2–3) | `src-tauri/src/import/pipeline.rs` — `PipelineWorkspace::create(project_dir, session_id)` en `{proyecto}/.import-staging/`, `Drop` que limpia, `copy_with_progress()`, `pipeline_copy_videos()`, `pipeline_passthrough()` · `src-tauri/src/infrastructure/import_pipeline.rs` — `StdImportPipeline::import()`, eventos y `EmitThrottle` |
| Metadatos (§3) | `src-tauri/src/import/pipeline.rs` — escritorio: `exiftool_metadata_args()`, `pipeline_stage_signed()`, `sign_one()` · Android: `pipeline_metadata()` · `src-tauri/src/exiftool.rs` — pool `run_text_in()` / `close_pool()` · `src-tauri/src/import/xmp.rs` — `write_xmp_sidecar()` (Rust puro) |
| Movimiento (§3) | `src-tauri/src/import/pipeline.rs` — `pipeline_move_shots()`, `move_pair()`, `move_into()`, `free_name()` y `free_stem()` (sufijo `-001`…), `fs::rename` con respaldo copia+borrado, `.xmp` excluidos de la lista EXIF |
| Pares (§3b) | `src-tauri/src/domain/pairs.rs` — `ShotKey`, `FileKind`, `group_shots()` · `src-tauri/src/infrastructure/import_pipeline.rs` — `stage_shots()` · `src-tauri/src/application/use_cases/import_photos.rs` — `existing_shots()`, `join()`, `import_breakdown()` · `src-tauri/src/commands/import_cmd.rs` — `analyze_import_selection()` |
| EXIF (§4) | `src-tauri/src/commands.rs` — `extract_exif_metadata_batch()`, `exiftool -csv`, headers dinámicos, `-IFD0:Orientation` → `rotation°` · `src-tauri/src/exif_android.rs` — RAW con `rawler`, JPEG con `kamadak-exif` |
| Inserción (§4) | `src-tauri/src/commands.rs` — `create_photos_batch()`, `CreatePhoto` incl. `rotation` · `src-tauri/src/db/queries.rs` · `db/project_schema.sql`; migración automática `ALTER TABLE ADD COLUMN rotation` |
| Miniaturas (§5) | `src-tauri/src/commands.rs` — `cache_thumbnail()`, `cache_thumbnails_parallel()` · `src-tauri/src/exif_android.rs` — `rawler::extract_thumbnail_pixels()`, `image::open()` + rotación `kamadak-exif` |
| Progreso (§6) | `src-tauri/src/import/progress.rs` — `ImportPhase`, `ImportProgress`, `ImportErrorCode`, `error_code_for()`, `FailedFile`, `SkippedFile`, `FatalError`, `ImportLogEntry`, `ImportResult` (serializados en eventos Tauri) |
| Caso de uso (§2–5) | `src-tauri/src/application/use_cases/import_photos.rs` — `ImportPhotos::execute()` (siempre devuelve `ImportOutcome`), registro y miniaturas solo de las fotos movidas |
| Resultado (§5) | `src/lib/hooks.ts` — `useImport()` con estado `idle / importing / done` · `src/pages/ImportPage.tsx` — progreso por bytes y archivo, resultado completo/parcial/fallido, fallidos e ignorados, registro traducido |
| Reintento (§7) | `src/pages/ImportPage.tsx` — `retryPlan` (registro vs copia, espacio), `handleRetry()` · `src/lib/hooks.ts` — `useImport().retry()`, `mergeRetryResult()` · `src-tauri/src/commands/import_cmd.rs` — `retry_import_register()`, `finish_import()` (compartido con `start_import`) · `src-tauri/src/application/use_cases/import_photos.rs` — `ImportPhotos::register_existing()` · `src-tauri/src/db/queries.rs` — `get_registered_dng_paths()` |
