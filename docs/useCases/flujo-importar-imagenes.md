# Flujo: Importar Imágenes a un Proyecto

Lumik Desktop · Linux · Windows · Android

Wizard de 3 pasos en el frontend y un pipeline de 5 fases en Rust: copia a un workspace
temporal, metadatos y renombrado, movimiento a `_media/`, extracción EXIF e inserción en
`project.db`, y generación de miniaturas.

> **Splits por plataforma:** en los diagramas que se bifurcan, la rama izquierda es
> **Linux / Windows** (vía `exiftool`) y la derecha **Android** (Rust puro, sin `exiftool`).

## Vista general

```mermaid
flowchart LR
    W["Wizard<br/>3 pasos"]
    I["invoke<br/>'start_import'"]
    P["Particionar<br/>videos / fotos"]
    C["Workspace temp<br/>+ copia"]
    M["Metadatos<br/>+ renombrado"]
    D["Mover a _media/"]
    X["EXIF → DTOs"]
    B["create_photos_batch()"]
    T["Miniaturas<br/>.thumbs/"]
    R["ImportResult"]

    W --> I --> P --> C --> M --> D --> X --> B --> T --> R

    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef ipc fill:#c8e6c9,stroke:#388e3c,color:#10281a
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    class W ui
    class I,R ipc
    class P,C,M,D,X,T be
    class B db
```

## 1. Wizard de importación (frontend)

```mermaid
flowchart TD
    start(["Usuario en pantalla Importar<br/>o en Detalle de proyecto"])
    s1["Paso 1 — Seleccionar archivos<br/>• Clic en 'Browse' o arrastrar archivos (DropZone)<br/>• Validar extensiones: RAW, DNG, JPEG, video (MP4, MOV…)<br/>• Mostrar FileList con tamaño total acumulado"]
    s2["Paso 2 — Seleccionar destino<br/>(solo modo desktop completo)<br/>• Elegir dispositivo destino (useConnectedDevices)<br/>• Elegir proyecto existente o crear uno nuevo<br/>• Validar espacio libre ≥ tamaño total"]
    s3["Paso 3 — Confirmar e iniciar<br/>• Resumen: N fotos + M videos, dispositivo, proyecto<br/>• Usuario confirma → genera session_id único"]
    invoke["api.startImport()<br/>listen('import-progress') + invoke('start_import', ImportRequest)<br/>{ session_id, source_files[], project_id,<br/>device_uuid, mount_point, project_name }"]
    next(["continúa en §2"])

    n_uri["Android: DocumentsUI devuelve<br/>content://… URIs<br/>→ resolveContentUri()<br/>→ /storage/{UUID}/DCIM/…"]
    n_emb["Paso 2 se omite en modo embedded<br/>(desde ProjectDetail,<br/>el proyecto ya se conoce)"]

    start --> s1 --> s2 --> s3 --> invoke --> next
    s1 -.- n_uri
    s2 -.- n_emb

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef ipc fill:#c8e6c9,stroke:#388e3c,color:#10281a
    classDef note fill:#fff9c4,stroke:#f9a825,color:#3d3300
    class start,next term
    class s1,s2,s3 ui
    class invoke ipc
    class n_uri,n_emb note
```

## 2. Arranque del backend y particionado

Los videos salen del pipeline de inmediato: se copian tal cual, sin conversión, sin XMP y
sin registro en base de datos.

```mermaid
flowchart TD
    be_load["Cargar ProjectDatabase por project_id<br/>+ AppSettings (embed_metadata)<br/>+ datos del fotógrafo + descripción del proyecto"]
    part{"Particionar archivos<br/>videos vs fotos"}
    video_copy["pipeline_copy_videos()<br/>Copiar directamente a _video/<br/>Sin conversión · Sin XMP · Sin BD"]
    video_done(["videos_copied count<br/>→ ImportResult"])
    ws["PipelineWorkspace::create()<br/>Directorio temp lumik_{proyecto}_{ts}/files/<br/>Linux/Win: /tmp/ · Android: $HOME/cache/"]
    next(["continúa en §3"])

    be_load --> part
    part -->|Videos| video_copy --> video_done
    part -->|Fotos RAW/JPEG| ws --> next

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    class next term
    class be_load,video_copy,video_done,ws be
    class part dec
```

## 3. Copia, metadatos y movimiento a destino

```mermaid
flowchart TD
    passthrough["pipeline_passthrough() — todas las plataformas<br/>Archivos RAW/JPEG copiados en formato original,<br/>sin conversión"]
    ready["Archivos listos en workspace/files/"]

    plat_q{"Plataforma"}
    meta_desktop["Linux / Windows<br/>rename_and_embed_metadata()<br/>exiftool batch sobre workspace/files/<br/>• Pre-pass: si no hay DateTimeOriginal → DateTimeOriginal<br/>• Rename: {DateTimeOrig}__{proyecto}__%03c.{ext}<br/>• Embed: Artist, Copyright, ImageDescription@Año,<br/>CreatorURL, CreatorEmail, UsageTerms"]
    meta_android["Android<br/>rawler → extract_datetime → rename<br/>• Rename: {DateTimeOrig}__{proyecto}__{n:03}.{ext}<br/>• Sidecar .xmp: Artist, Copyright,<br/>ImageDescription@Año, CreatorURL, CreatorEmail<br/>(Rust puro, sin exiftool)"]

    merge_meta["Archivos renombrados en workspace<br/>Metadata embebida (Linux/Win) o en sidecar .xmp (Android)"]
    move["pipeline_move_to_dest()<br/>Mover a {project}/_media/ (+ sidecar .xmp si existe)<br/>Implementado como Copy + Delete — seguro cross-filesystem"]
    cleanup["workspace.cleanup()<br/>Eliminar directorio temp completo (fs::remove_dir_all)"]
    next(["continúa en §4"])

    passthrough --> ready --> plat_q
    plat_q -->|Linux/Win| meta_desktop --> merge_meta
    plat_q -->|Android| meta_android --> merge_meta
    merge_meta --> move --> cleanup --> next

    classDef term fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef plat fill:#e1d5e7,stroke:#9673a6,color:#2a1a33
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3100
    class next term
    class passthrough,ready,merge_meta,move,cleanup be
    class meta_desktop,meta_android plat
    class plat_q dec
```

## 4. Extracción EXIF e inserción en base de datos

```mermaid
flowchart TD
    plat_q{"Plataforma"}
    exif_desktop["Linux / Windows<br/>exiftool -csv batch sobre todos los archivos<br/>Extrae: Width, Height, DateTimeOriginal, Make, Model,<br/>ISO, FNumber, ExposureTime, FocalLength, LensModel,<br/>IFD0:Orientation → rotation°<br/>capture_date: DateTimeOriginal → CreateDate → ModifyDate<br/>Headers parseados dinámicamente — exiftool omite<br/>columnas vacías del batch → HashMap"]
    exif_android["Android<br/>exif_android::extract_exif_metadata_batch()<br/>RAW: crate rawler → RawMetadata.exif.orientation → rotation°<br/>JPEG: crate kamadak-exif → tag Orientation → rotation°<br/>(rawler no soporta JPEGs de cámaras de teléfono)<br/>→ HashMap"]
    merge_exif["HashMap (incl. rotation) disponible<br/>para todos los archivos"]
    dtos["Construir CreatePhoto DTOs<br/>dng_path · original_format · capture_date · dimensiones ·<br/>ISO · apertura · velocidad · longitud focal · lente · rotation"]
    insert["project_db.create_photos_batch() — transacción SQLite única<br/>• Genera UUID por foto<br/>• INSERT INTO photo (id, project_id, dng_path, device_uuid,<br/>original_camera, original_format, capture_date, width, height,<br/>file_size_bytes, iso, aperture, shutter_speed, focal_length,<br/>lens_model, rotation)<br/>• Commit en éxito · Rollback en error, sin inserciones parciales"]
    next(["continúa en §5"])

    plat_q -->|Linux/Win| exif_desktop --> merge_exif
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

Las miniaturas se generan en paralelo, con un máximo de 8 hilos.

```mermaid
flowchart TD
    plat_q{"Plataforma"}
    thumb_desktop["Linux / Windows — cache_thumbnail()<br/>RAW/JPEG: exiftool -b -PreviewImage<br/>(fallback -ThumbnailImage)<br/>TIFF: image::open() → si falla por JPEG strips,<br/>open_jpeg_tiff() via crate tiff 0.11 + zune-jpeg (Rust puro)<br/>→ image crate: resize 320×320, rotación EXIF aplicada<br/>→ encode JPEG → .thumbs/{id}.jpg"]
    thumb_android["Android — exif_android::cache_thumbnail()<br/>RAW: rawler::extract_thumbnail_pixels()<br/>+ resize + rotación física<br/>JPEG: image::open() + resize<br/>+ rotación via kamadak-exif<br/>→ encode JPEG → .thumbs/{id}.jpg"]
    merge_thumb["Miniaturas JPEG en {project}/.thumbs/{photo_id}.jpg<br/>para todas las fotos importadas"]
    result["Retornar ImportResult al frontend<br/>{ total_files, successful, failed, failed_files[], videos_copied }"]
    show["Mostrar resultado en UI<br/>• Barra de progreso (eventos 'import-progress')<br/>• Resumen: N fotos importadas + M videos copiados<br/>• Lista de errores si failed > 0 — hasta 3 archivos detallados"]
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

El backend emite `app.emit('import-progress', progress)`. La granularidad es **por fase, no
por archivo**; el frontend filtra por `session_id`.

```mermaid
flowchart LR
    p0["0/3<br/>Copiando archivos"] --> p1["1/3<br/>Agregando metadatos"] --> p2["2/3<br/>Registrando en BD"] --> p3["3/3<br/>Completo"]

    classDef note fill:#fff9c4,stroke:#f9a825,color:#3d3300
    class p0,p1,p2,p3 note
```

`ImportPhase` admite `Reading`, `Converting`, `Writing`, `Saving`, `Complete` y `Failed`.

## Archivos de código

| Fase | Archivos |
|------|----------|
| Wizard (§1) | `src/pages/ImportPage.tsx` — wizard 3 pasos, DropZone, FileList, `handleBrowse()`, `resolveContentUri()` (Android); auto-muestra importación si el proyecto está vacío |
| IPC (§1) | `src/lib/api.ts` — `startImport()`, `listen('import-progress')` · `src/lib/hooks.ts` — `useImport()`, `session_id` |
| Arranque (§2) | `src-tauri/src/commands.rs` — `start_import()`, `AppSettings(embed_metadata)`, particionado, `image_description = {desc}@{year}` |
| Workspace (§2–3) | `src-tauri/src/import/pipeline.rs` — `PipelineWorkspace::create()` (solo `files/`), `pipeline_passthrough()`; temp dir `/tmp` (Linux/Win) o `$HOME/cache` (Android) |
| Metadatos (§3) | `src-tauri/src/import/pipeline.rs` — `pipeline_metadata()`, split Linux vs Android, `rename_and_embed_metadata()` · `src-tauri/src/import/xmp.rs` — `write_xmp_sidecar()` (Rust puro) |
| Movimiento (§3) | `src-tauri/src/import/pipeline.rs` — `pipeline_move_to_dest()`, `move_dng_files()`, Copy + Delete cross-FS, `.xmp` excluidos de la lista EXIF, `workspace.cleanup()` |
| EXIF (§4) | `src-tauri/src/commands.rs` — `extract_exif_metadata_batch()`, `exiftool -csv`, headers dinámicos, `-IFD0:Orientation` → `rotation°` · `src-tauri/src/exif_android.rs` — RAW con `rawler`, JPEG con `kamadak-exif` |
| Inserción (§4) | `src-tauri/src/commands.rs` — `create_photos_batch()`, `CreatePhoto` incl. `rotation` · `src-tauri/src/db/queries.rs` · `db/project_schema.sql`; migración automática `ALTER TABLE ADD COLUMN rotation` |
| Miniaturas (§5) | `src-tauri/src/commands.rs` — `cache_thumbnail()`, `cache_thumbnails_parallel()` · `src-tauri/src/exif_android.rs` — `rawler::extract_thumbnail_pixels()`, `image::open()` + rotación `kamadak-exif` |
| Progreso (§6) | `src-tauri/src/import/progress.rs` — `ImportPhase`, `ImportProgress`, `ImportResult`, `FailedFile`, `ImportLogEntry` (serializados en eventos Tauri) |
| Resultado (§5) | `src/lib/hooks.ts` — `useImport()` sobre el evento `import-progress` · `src/pages/ImportPage.tsx` — barra de progreso, resumen, lista de errores |
