# Lumik Desktop

App de escritorio para importar y hacer culling de fotografías RAW.
Importa sesiones desde tarjetas o discos, escribe metadatos XMP al fotógrafo,
y permite aprobar y filtrar qué fotos vale la pena editar. También respalda la
carpeta `_exported/` de cada proyecto en la nube a través del server de respaldo
(`apps/server`), que comparte catálogo con el portal web (`apps/web`).

Comportamiento esperado por capability: `openspec/specs/` en la raíz del monorepo.

## Stack

- **Framework**: Tauri v2 (backend Rust, frontend React+TypeScript)
- **Bundler**: Vite
- **BD global**: SQLite en `~/.local/share/lumik/lumik.db` (fotógrafo, ajustes, keybindings, dispositivos conocidos, sesión y cola de respaldo)
- **BD proyectos**: SQLite por proyecto, en cada disco externo (`project.db`)
- **Metadatos EXIF/XMP**: `exiftool` (empaquetado en Windows y macOS; en Linux debe estar instalado)
- **UI**: `@lumik/ui` (paquete interno del monorepo en `packages/ui/`)

## Estructura de archivos relevantes

```
src/
  App.tsx                           # Raíz: secciones projects / settings / about (useState, sin router)
  lib/
    api.ts                          # Todas las llamadas invoke() a Tauri (una función por comando)
    hooks.ts                        # Custom hooks async para datos + useImport + useKeybindings
    types.ts                        # Tipos TypeScript que espeja los modelos Rust
    BackupContext.tsx               # Estado global de respaldos en segundo plano (progreso por archivo, cola, reintentar/descartar, modal de detalle)
    BackupSessionContext.tsx        # Sesión de la cuenta de respaldo compartida (Ajustes, aviso sin sesión, reanudación de la cola)
  pages/
    ImportPage.tsx                  # Wizard de importación: origen → ajustes de la importación → confirmar (solo desde un proyecto)
    SettingsPage.tsx                # Tabs: Importación, Atajos, Apariencia, Respaldo (`initialTab` para abrir una concreta)
    BackupAccountSection.tsx        # Tab Respaldo: URL del server, login/registro o email + cerrar sesión
    AboutPage.tsx
    Project/
      Projects/
        Projects.tsx                # Dashboard: proyectos por año, menú (editar, eliminar, abrir carpeta), iconos de proveedor
        EditProjectModal.tsx          # Editar nombre, fecha y descripción (+ renombrar archivos / reescribir descripción)
        DeleteProjectModal.tsx
      ProjectDetail/
        ProjectDetail.tsx           # Grid virtual de fotos (TanStack Virtualizer), agrupadas por día o en cuadrícula
        ProjectDetailHeader.tsx     # Barra superior: orden, filtros, modo de vista
        FilterPopovers.tsx          # Filtros por estrellas, tags y color
        ProjectDetailFooter.tsx     # Contador de fotos/aprobadas, filtros de aprobadas y papelera
        TrashPhotoModal.tsx         # Confirmación de mover foto a la papelera (borrado lógico a _trash/)
        BackupModal.tsx             # Modal de respaldo de _exported (proveedores, calidad, progreso, reintentar); lo monta BackupModalHost
      PhotoDetail/
        PhotoDetail.tsx             # Visor: atajos, aprobación, navegación
        PhotoViewer.tsx             # Canvas con zoom/pan/rotate
        PhotoViewerToolbar.tsx
        PhotoSidebar.tsx            # Panel info: histograma, EXIF, rating, color, tags, aprobación
  workers/
    histogram.worker.ts             # Web Worker: calcula histograma RGB de la imagen
  components/
    CreateProjectModal.tsx          # Modal para crear proyecto (nombre, fecha, dispositivo)
    DateField.tsx                   # Campo de fecha (flatpickr) compartido por los modales de proyecto
    Layout.tsx / Sidebar.tsx        # Shell de la app; Sidebar con secciones colapsables Respaldos (tarjetas de jobs; solo visible si hay jobs) y Almacenamiento
    BackupAccountForm.tsx           # Login / registro en el server de respaldo (lo usa Ajustes → Respaldo)
    BackupModalHost.tsx             # Único montaje de BackupModal, abierto con openBackupDetail desde el proyecto o la barra; sin sesión muestra el aviso
    BackupNoSessionModal.tsx        # Aviso al respaldar sin sesión, con "Ir a Ajustes" (pestaña Respaldo)
    BackupCloseGuard.tsx            # Confirmación al cerrar la ventana con respaldos en curso
```

El backend Rust sigue una arquitectura por capas (ver
`docs/desktop-clean-architecture.md`). Las dependencias apuntan hacia adentro:
`interface → application → domain`, y `infrastructure` implementa los puertos.

El núcleo vive en su propio crate, **`crates/lumik-core`**, compartido con la
futura app de tablet (`openspec/changes/tablet-app-spike`). No depende de Tauri,
de exiftool ni de `#[cfg(target_os)]`; `src-tauri` es el shell del desktop y lo
reexporta en `lib.rs` (`pub use lumik_core::{application, db, domain, import}`),
así que `crate::domain::…` sigue valiendo dentro del shell.

```
crates/lumik-core/src/              # núcleo compartido (desktop y tablet)
  domain/                           # núcleo puro (sin SQLite/exiftool/Tauri/fs), 100% testeable
    photo.rs                        #   VOs: Rotation, Stars, normalize_tags
    paths.rs                        #   cull_destination (_media↔_culled), destinos de _trash/ y _exported/, path_to_slash
    pairs.rs                        #   pares RAW+JPEG: ShotKey (nombre de cámara + fecha), group_shots
    delivery.rs                     #   DeliveryMode (nada / pares / todo JPG de cámara), delivery_for
    lock.rs                         #   candado del proyecto (.lumik-lock): ProjectLock, LockState
    search.rs                       #   SearchQuery: operadores de búsqueda y texto libre (ida y vuelta)
    device.rs                       #   DetectedDevice
    project.rs                      #   ProjectFolder (slug/date_parts/path), compare_dashboard
    error.rs                        #   DomainError

  application/                      # orquestación; define los puertos (traits)
    ports.rs                        #   DeviceScanner, MetadataTool, ImageProcessor, FinderTagWriter,
                                    #   FileStore, PhotoRepository, ImportPipeline, ProgressReporter, ProjectLockStore
    registry.rs                     #   ProjectRegistry (proyectos abiertos, ejecting, rotation-gen)
    project_session.rs              #   ProjectSessions: sesiones por proyecto y su candado
    index_queue.rs                  #   IndexQueue: proyectos por reindexar en la búsqueda (debounce)
    use_cases/                      #   cull_photo, trash_photo, rate_photo, rotate_photo, import_photos,
                                    #   eject_device, ensure_catalog_synced, sync_exported, change_delivery_mode
                                    #   (con tests de fakes)

  import/                           # pipeline genérico (staging, mover, pares, nombres libres), converter, xmp, progress
  db/                               # persistencia SQLite (GlobalDatabase / ProjectDatabase)
    mod.rs, models.rs, queries.rs (photo_from_row), migrations.rs, persistence.rs (PhotoRepository), *_schema.sql
    search_index.rs                 #   índice de búsqueda en lumik.db: escritura por proyecto y consulta

src-tauri/src/                      # shell del desktop
  main.rs                           # entry point → lumik_lib::run()
  lib.rs                            # composition root: reexporta el núcleo, arma infra, elige impl por plataforma, registra comandos

  infrastructure/                   # implementa los puertos (aquí vive el #[cfg] de plataforma)
    metadata.rs                     #   ExiftoolMetadata (desktop) / AndroidMetadata (rawler)
    imaging.rs                      #   DesktopImaging / AndroidImaging (thumbnails/previews)
    import_signing.rs               #   firma del import: pool de exiftool (desktop) / rawler + XMP (Android)
    lock.rs                         #   FsLockStore (escritura atómica de .lumik-lock), nombre del equipo
    app_data.rs                     #   ruta de lumik.db por plataforma
    devices.rs, finder_tags.rs, folder.rs, fs.rs, progress.rs, import_pipeline.rs
    backup_auth.rs, catalog.rs, backup_client.rs   # clientes HTTP del server de respaldo
    disk_arbitration.rs             #   FFI mínimo de DiskArbitration (solo macOS): watcher y expulsión

  commands.rs                       # AppState (+ write_session) + refresh_open_projects + declaración de submódulos
  commands/                         # comandos Tauri (adaptadores delgados) por dominio
    device.rs, project.rs, photo.rs, settings.rs, import_cmd.rs, backup.rs

  devices.rs                        # scan_mounted_devices() por plataforma (sysinfo / /proc); UUID en macOS
                                    #   vía diskutil (por punto de montaje o, si falla, por disco BSD); read_only
  device_watch.rs                   # watcher nativo de hotplug (Linux mountinfo, Windows WMI, macOS DiskArbitration)
  apple_tags.rs                     # sidecars AppleDouble (desktop-only)
  exiftool.rs / exif_android.rs     # herramientas EXIF por plataforma
  e2e_tests.rs                      # tests de punta a punta: núcleo + infraestructura real
  tests/fixtures/                   # JPEGs con EXIF conocido para tests de caracterización
```

**Nota:** `commands.rs` ya NO es un módulo-Dios: los comandos son adaptadores
delgados que construyen un caso de uso o llaman un puerto. La lógica frágil
(rollback de culling, debounce de rotación, normalización de tags, construcción
de rutas, migraciones) vive en `domain`/`application` con tests.

## Arquitectura de base de datos

**Dos tipos de BD coexisten:**

1. **BD Global** (`~/.local/share/lumik/lumik.db`):
   Fotógrafo activo, metadatos del fotógrafo, ajustes de la app (`AppSettings`),
   keybindings, registro de dispositivos conocidos (`device`), cola de respaldos
   pendientes (`backup_queue`) y la sesión del server de respaldo (claves
   `backup.*` en `settings`: URL, tokens, fotógrafo).

2. **BD de Proyecto** (`{mount}/lumik/{año}/{mes}/{día}_{nombre}/project.db`):
   Una por proyecto, vive en el disco externo. Contiene las tablas `project`,
   `photo` (paths relativos a la carpeta del proyecto) y `project_settings`.

**Identidad:** un proyecto se identifica por `project.id`, en cualquier disco y
plataforma; copiar o mover su carpeta a cualquier ruta bajo `lumik/`, en el mismo
disco o en otro, basta para que se liste y funcione desde ahí. El UUID del disco
es local de cada máquina (macOS y Linux ven UUIDs distintos para el mismo exFAT)
y solo sirve para localizar y expulsar.
`photo.device_uuid` está obsoleto: las fotos nuevas lo guardan vacío y nada lo
lee. Limitación: si el mismo `project.id` está en dos discos montados, solo se
lista uno.

**Discos de solo lectura:** `DetectedDevice.read_only` (`statvfs`/`ST_RDONLY` en
Linux y macOS, `GetVolumeInformationW` en Windows). Se registran y se listan con
la marca "Solo lectura", pero `refresh_open_projects` no abre sus `project.db`
(abrirlos escribe: WAL, migraciones) y `create_project` los rechaza. Caso típico:
NTFS en macOS, que el driver nativo monta como `ntfs://diskXsY/…` (tipo `lifs`).

**`AppState`** mantiene `global_db`, los puertos de infraestructura
(`device_scanner`, `metadata`, `image_processor`, `finder_tags`, `file_store`) y
un **`registry: Arc<ProjectRegistry>`** (`application/registry.rs`) que encapsula
el estado de sesión: proyectos abiertos (`open_projects`), dispositivos en
expulsión y el contador de generación del debounce de rotación. El mapa de
proyectos abiertos se puebla al arrancar y se refresca en cada
`scan_connected_devices`; si el disco se desmonta, sus proyectos desaparecen y el
comando devuelve error "device not mounted".

**Candado del proyecto** (`sessions: Arc<ProjectSessions>`,
`application/project_session.rs`): la vista abierta y cada comando que escribe
abren una sesión del proyecto; la primera escribe `.lumik-lock` en su carpeta y la
última lo quita. Todo comando que cambia archivos o la base empieza con
`state.write_session(&project_id)?` (falla con `AppError::ProjectLocked`, que viaja
como `project_locked:<dispositivo>`); `delete_project` y `edit_project` usan
`ensure_not_locked` porque cierran la base. Ver `docs/useCases/flujo-candado-proyecto.md`.

## Estructura de carpetas en disco externo

```
{mount_point}/lumik/{año}/{mes}/{día}_{nombre-proyecto}/
  project.db
  _media/
    IMG_0001.dng
    IMG_0002.dng
  _culled/
    IMG_0047.dng
  _trash/                    # papelera: fotos apartadas (borrado lógico), restaurables
    IMG_0003.dng
    _exported/               # exportados de fotos en la papelera
  _video/
    CLIP_001.mp4
  .thumbs/
    {photo_id}.jpg           # JPEG 480px, rotación física aplicada
  .previews/
    {photo_id}.jpg           # JPEG full-res extraído del DNG, Orientation=1
  .import-staging/           # solo durante un import: staging de la sesión (se borra al terminar)
```

## Reglas técnicas críticas

- **Paths siempre relativos al proyecto**: `dng_path` y `jpg_path` en BD son relativos
  a la carpeta del proyecto (la del `project.db`), p. ej. `_media/IMG_0001.dng`. El path
  absoluto se reconstruye en runtime con `ProjectDatabase::photo_file`
  (`{project_dir}/{dng_path}`). Mover la carpeta del proyecto no toca la BD. Las BDs
  con rutas viejas (relativas al mount point) se convierten al abrirlas, dejando antes
  una copia `.project.db.pre-relative-paths` junto al `project.db`.
- **`dng_path` refleja la ubicación real**: cuando una foto se cullea, `dng_path` se
  actualiza a `_culled/{filename}`. No hay campo separado "culled_path".
- **`_media/` y `_culled/` son hermanas**: ambas viven directamente bajo el directorio
  del proyecto. Al cullarse, el archivo pasa de `_media/` a `_culled/`; al descullear, vuelve a `_media/`.
- **`_video/`**: carpeta donde van los videos. Se copian sin conversión ni escritura de
  metadatos XMP (pasan por `.import-staging/` y llegan con `rename`).
- **exiftool es requerido en runtime**: thumbnails, previews, EXIF batch, orientación
  y metadata XMP dependen de él. Sin exiftool, la importación y el visor fallan.
  Siempre se lanza con `exiftool::command()`, nunca con `Command::new("exiftool")`:
  - **Windows**: `exiftool.exe` (lanzador) va en `externalBin` y `exiftool_files/`
    (Strawberry Perl + `exiftool.pl` + `lib/`) como recurso (`tauri.windows.conf.json`);
    el sistema lo encuentra por nombre junto al ejecutable.
  - **macOS**: `tauri.macos.conf.json` empaqueta `exiftool_files/exiftool.pl` como
    `Resources/exiftool/exiftool` junto con `lib/Image/` y `lib/File/RandomAccess.pm`,
    y se ejecuta con `/usr/bin/perl` (sin arquitectura: sirve en el binario universal).
    Una app abierta desde Finder recibe un PATH mínimo, así que el orden es: script
    del bundle → PATH → `/opt/homebrew/bin` → `/usr/local/bin`. Con `tauri dev` no
    hay bundle y se usa el del PATH. El log de arranque dice cuál se eligió.
  - **Linux**: el del PATH.
- **Orientación**: el DNG guarda `IFD0:Orientation`. Los thumbnails tienen la
  rotación física aplicada. Los previews tienen `Orientation=1` y el canvas
  aplica la rotación manualmente vía `PhotoViewer`.
- **Un fotógrafo activo**: la app crea automáticamente un fotógrafo con el username
  del sistema al primer arranque (`main.rs`).
- **Procesamiento de un archivo a la vez en la pipeline** para no saturar RAM
  (un RAW decodificado ≈ 150-200 MB).

## Pipeline de importación

El comando `start_import` (`commands/import_cmd.rs`) es un adaptador delgado que
resuelve los ajustes de la importación (`application/import_settings.rs`: los que
manda el wizard en `ImportRequest.settings` o, si no llegan, los globales, sin
modificarlos), particiona fotos/videos y delega en el caso de uso `ImportPhotos`
(`application/use_cases/import_photos.rs`), que orquesta el pipeline vía el puerto
`ImportPipeline` (`infrastructure/import_pipeline.rs`):

```
source files
  → PipelineWorkspace::create → {proyecto}/.import-staging/{sesión}/ (mismo disco que el destino;
                                  borra restos de un import interrumpido; Drop lo limpia)
  → pipeline_copy_videos → staging → pipeline_move_to_dest(_video/)
  → escritorio con algo que escribir: pipeline_stage_signed (fase copy_metadata): copia
      secuencial origen → staging y, solapado, 4 workers (sesiones del pool de exiftool) firman
      y renombran con exiftool -o dentro del staging. El origen se lee de a uno: con lectores
      concurrentes un disco mecánico o una tarjeta se desploman
    sin nada que escribir: pipeline_passthrough → staging (copia tal cual, fase copy)
    Android: pipeline_passthrough + pipeline_metadata (rawler + sidecar XMP, fase metadata)
    (un archivo por subcarpeta del staging, formato original, sin conversión)
  → stage_shots: agrupa en tomas (par RAW+JPEG por nombre de cámara + fecha de captura, o
      unión con una foto suelta del proyecto) → pipeline_move_shots(_media/): free_name() por
      archivo suelto, free_stem() por par (el mismo nombre libre para los dos archivos)
  → MetadataTool::extract_batch (UN proceso exiftool para todos, salida CSV)
  → PhotoRepository::create_batch (UNA transacción SQLite)
  → cache_thumbnails_parallel (ImageProcessor, paralelo por CPU cores, max 8)
```

**Fallos por archivo:** un archivo que no se puede copiar, preparar o mover se agrega a
`failed_files` con un `ImportErrorCode` (`source_unreadable`, `no_space`, `write_failed`,
`metadata_failed`, `register_failed`, `unknown`) y el resto sigue; tras `no_space` los
pendientes fallan sin intentarse. Los no admitidos van a `skipped_files`. Un error general (no
se puede crear el staging, `_media/` o `_video/`) va en `ImportResult.fatal` junto con lo ya
importado: `start_import` devuelve `Ok` en todo lo que ocurre durante la importación y `Err`
solo para precondiciones (proyecto no abierto, sin fotógrafo). Nunca se sobrescribe un archivo
de `_media/` o `_video/`.

El auto-rename de la carpeta por fecha (usa `relocate_project_folder`, acoplado al
registry) se queda en el adaptador, tras el caso de uso.

El progreso se emite como evento Tauri `"import-progress"` **por archivo**: fase (`videos`,
`copy`, `copy_metadata`, `metadata`, `move`, `register`, `complete`), archivo N de M y bytes del archivo y del
total (copia por bloques de 4 MiB con `copy_with_progress`, como máximo un evento cada ~100 ms).
El registro va por `"import-log"` como entradas estructuradas (`ImportLogEntry { kind, … }`) que
el frontend traduce (`import.log.*`, `import.errors.codes.*`). `useImport()` expone el estado
`idle | importing | done`, y el paso 3 muestra siempre el resultado (completo, parcial o fallido).

**Reintentar fallidos:** desde el resultado, "Reintentar (N)" (`useImport().retry()`) procesa solo los
fallidos con los mismos ajustes, en serie y cada tanda con su `session_id`: los `register_failed` ya están
en `_media/` y van a `retry_import_register` (`ImportPhotos::register_existing`: solo EXIF + BD +
miniaturas, por nombre contra la carpeta actual, sin registrar dos veces los `dng_path` que ya están); el
resto vuelve a `start_import` con sus rutas de origen, si cabe en el disco. `mergeRetryResult()` acumula el
total de la sesión. Los dos comandos cierran con `finish_import()` (fecha inferida y renombrado de carpeta).

## Flujo de navegación (frontend)

```
App
├── Projects (dashboard por año, renderizado normal con .map())
│     ├── menú de proyecto → editar / eliminar / abrir carpeta
│     ├── icono de proveedor → carpeta del proyecto en la nube
│     └── click proyecto → ProjectDetail
│           ├── click foto → PhotoDetail
│           ├── botón respaldo → BackupModal (sin sesión: aviso → Settings/Respaldo)
│           └── botón importar → ImportPage (origen → ajustes → confirmar)
├── Settings (Importación · Atajos · Apariencia · Respaldo)
└── About
```

`App.tsx` maneja el routing con `useState`. No hay React Router.

## Visor de fotos (PhotoDetail / PhotoViewer)

- Renderiza en `<canvas>`, nunca en `<img>`. Esto evita el flash blanco entre fotos.
- Zoom/pan con refs (sin re-renders). Solo `displayScale` y `isDragging` son estado React.
- `useImperativeHandle` expone `zoomIn/zoomOut/fitToScreen/rotateLeft/rotateRight`
  para que el componente padre pueda invocarlos desde los atajos de teclado.
- El histograma se calcula en `histogram.worker.ts` (off-thread) y se pinta en canvas
  con `globalCompositeOperation = "screen"` para los tres canales RGB.
- Ediciones (stars, color_label, tags, culled) son **optimistas**: se reflejan en
  `photoOverrides` de `ProjectDetail` antes de que la BD confirme.

## Atajos de teclado

Los atajos se almacenan en la BD global y se cargan con `useKeybindings()`.
Defaults definidos en `hooks.ts:DEFAULT_KEYBINDINGS`. Se agrupan por contexto
(`photo_detail.*`, `project.*`). `matchesKey()` compara un `KeyboardEvent` contra
el valor almacenado (soporta `"Escape"`, `"ArrowLeft"`, `"Ctrl+c"`, etc.).
`"Ctrl+"` significa el modificador de la plataforma: Cmd en macOS y Ctrl en las demás
(`hasPlatformModifier()`); en macOS Ctrl no lo sustituye, porque Ctrl+clic es clic
secundario. Ajustes lo muestra como `⌘C` en macOS y `Ctrl+C` en las demás, y al
reasignar guarda `"Ctrl+<tecla>"` si estaba pulsado ese modificador.

## Flujos de producto ↔ comando ↔ caso de uso

Los flujos de `docs/useCases/` delegan en un caso de uso de
`application/use_cases/` (orquestación testeable con fakes). El resto de comandos
son de soporte/consulta (adaptadores directos a puertos/BD). El `invoke_handler`
de `lib.rs` está agrupado por flujo para reflejar esto.

| Flujo (`docs/useCases/`) | Comando (entry point)  | Caso de uso                  |
|--------------------------|------------------------|------------------------------|
| Importar Imágenes        | `start_import`, `retry_import_register` | `ImportPhotos` (`execute`, `register_existing`) |
| Crear Proyecto           | `create_project`       | — (lógica en el comando; `domain::ProjectFolder`) |
| Editar Proyecto          | `edit_project`         | `RenameProjectFiles` (con rollback), `RewriteProjectDescription` |
| Culling de imagen        | `save_photo_culled`    | `CullPhoto` (con rollback y entrega del JPG de cámara) |
| Entrega al aprobar       | `plan_project_delivery_change`, `set_project_delivery_mode` | `ChangeDeliveryMode` |
| Papelera / restaurar foto | `trash_photo`, `restore_photo` | `TrashPhoto` / `RestorePhoto` (con rollback) |
| Rating de estrellas      | `save_photo_rating`    | `RatePhoto`                  |
| Rotación de imagen       | `save_photo_rotation`  | `RotatePhoto`                |
| Sync catálogo (wip)      | `ensure_project_synced`| `EnsureCatalogSynced`        |
| Enlazar `_exported` ↔ fotos (wip) | `sync_exported`   | `SyncExported`               |
| Respaldar `_exported` (wip) | `prepare_backup` → `backup_exported_file` → `finalize_backup` | — (lógica en `commands/backup.rs`) |

**Al cambiar un flujo, actualizar su diagrama en `docs/useCases/` y su caso de uso.**

## Comandos Tauri disponibles

Plataforma: `get_platform`, `open_external_url`
Dispositivos: `scan_connected_devices`, `eject_device`, `get_known_devices`
Proyectos: `get_projects_dashboard`, `get_project`, `open_project_session`, `close_project_session`, `take_project_control`, `create_project`, `edit_project`, `set_project_archived`, `delete_project`, `open_project_folder`, `get_project_cover_thumbnail`, `set_project_cover_photo`, `get_project_settings`, `update_project_settings`, `get_project_delivery_mode`, `plan_project_delivery_change`, `set_project_delivery_mode`
Fotos: `get_project_photos`, `get_project_trashed_photos`, `trash_photo`, `restore_photo`, `reveal_photo`, `get_project_thumbnails`, `get_thumbnail`, `get_photo_preview`, `save_photo_rotation`, `save_photo_rating`, `save_photo_culled`, `regenerate_project_thumbnails`
Fotógrafo: `get_active_photographer`, `ensure_default_photographer`
Metadata: `get_photographer_metadata`, `update_photographer_metadata`
Ajustes: `get_app_settings`, `update_app_settings`, `get_install_name`, `set_install_name`, `get_dashboard_sort`, `update_dashboard_sort`, `get_dashboard_show_archived`, `set_dashboard_show_archived`, `get_sidebar_sections`, `set_sidebar_section_collapsed`
Búsqueda: `search_photos`, `get_search_thumbnail`, `get_search_index_count`, `parse_search_query`, `format_search_query`
Keybindings: `get_keybindings`, `update_keybinding`
Import: `start_import`, `analyze_import_selection`, `retry_import_register`
Respaldo — cuenta: `get_backup_server_url`, `set_backup_server_url`, `backup_register`, `backup_login`, `backup_logout`, `get_backup_session`
Respaldo — catálogo: `ensure_project_synced`, `sync_exported`, `sync_project_cover`
Respaldo — subida: `list_exported_backup`, `list_backup_providers`, `prepare_backup`, `backup_exported_file`, `finalize_backup`, `relocate_backup`, `get_backup_folder_link`
Respaldo — cola: `set_pending_backup`, `clear_pending_backup`, `get_pending_backups`

## Estado (v0.1.3-beta) — qué está implementado

| Feature | Estado |
|---------|--------|
| Import RAW/JPEG/TIFF + metadatos → disco externo (sin conversión) | ✓ Funcional |
| Grid virtual de fotos agrupadas por día | ✓ Funcional |
| Visor canvas (zoom / pan / rotate / histograma) | ✓ Funcional |
| Rating: estrellas, etiquetas de color, tags | ✓ Funcional |
| Culling: aprobar (mover a `_culled/`) y quitar aprobación | ✓ Funcional |
| Menú de la foto: mostrar en carpeta, mover a la papelera (`_trash/`) y restaurar | ✓ Funcional (mostrar en carpeta: no en Android) |
| Filtros (aprobadas, estrellas, tags, color) y modos de vista | ✓ Funcional |
| Keybindings configurables en Settings | ✓ Funcional |
| Metadata XMP del fotógrafo en import | ✓ Funcional |
| Menú de proyectos (renombrar, eliminar, abrir carpeta) | ✓ Funcional (abrir carpeta: no en Android) |
| Respaldo de `_exported` en la nube (cuenta, modal, cola offline) | ✓ Beta |
| Seguimiento de respaldos en la barra lateral (tarjetas, reintentar, descartar, confirmación al cerrar) | ✓ Beta (en Android la barra empieza colapsada) |
| Archivar proyectos | ✓ Funcional (menú de la tarjeta + botón "Archivados" en el dashboard) |
| Discos en macOS: detección inmediata (DiskArbitration) y expulsión del disco completo | ✓ Beta |
| Discos de solo lectura (p. ej. NTFS en macOS): se listan con marca y aviso, sin abrir ni crear proyectos | ✓ Beta |
| Paquete de macOS: `.dmg` universal (Intel + Apple Silicon, macOS 12+), firma ad hoc, exiftool incluido | ✓ Beta (sin notarizar) |
| Operaciones en lote desde el grid | ✗ No implementado |
| App iPad nativa | ✗ Planificado |

## Convenciones de código

- Estilos: objetos `React.CSSProperties` inline definidos como constantes en el
  mismo archivo. No hay CSS modules ni Tailwind — todo usa variables CSS de `@lumik/ui`.
- Variables CSS del design system: `--lumik-primary`, `--lumik-on-surface`,
  `--lumik-surface-container-low`, `--lumik-font-mono`, etc.
- Cada comando Tauri tiene su función wrapper en `src/lib/api.ts` y su hook
  en `src/lib/hooks.ts`. Los hooks siguen el patrón `AsyncState<T>` + `refetch`.
- Usar `pnpm` para instalar dependencias, nunca `npm` ni `npx`.

## Organización de componentes frontend

Al maquetar una página, aplicar esta regla:

- **Componente exclusivo de una página** → definirlo en el mismo archivo de la
  página (o en un archivo dentro de su carpeta, p.ej. `pages/ProjectDetail/`).
  No moverlo a ningún paquete compartido.
- **Componente usado en dos o más páginas** → moverlo a `packages/ui/src/components/`
  para ser consumido vía `@lumik/ui`. No duplicarlo en cada página.

El criterio es el uso real, no el potencial: un componente no se mueve hasta que
efectivamente se necesite en una segunda página.

## Internacionalización (i18n)

La app soporta español (por defecto) e inglés.

**Estructura:**
- Traducciones en `apps/desktop/src/i18n/locales/` (es.json, en.json)
- Configuración en `apps/desktop/src/i18n/config.ts`
- Extiende traducciones de `@lumik/ui` (componentes compartidos) con propias

**Organización de claves:**
- `import.*` - Flujo de importación
- `project.*` - Gestión de proyectos
- `settings.*` - Configuración (photographer, language, keybindings, placeholders, errors)
- `navigation.*` - Navegación principal
- `dashboard.*` - Vista de proyectos
- `projectDetail.*` - Detalle de proyecto (noDate, photos, culled, filters, viewMode, errors, loading)
- `photo.*` - Detalles de fotos (colors, sidebar, viewer, detail)
- `about.*` - Página about
- `components.*` - Traducciones de componentes (createProjectModal)

**Implementación:**
- Usar `const { t, i18n } = useTranslation()` en cada componente
- Llamar `t('key.path')` para todos los strings de UI
- **Nunca pasar props de texto** a componentes - dejar que traduzcan internamente
- Agregar `i18n.language` a dependencias de useMemo/useEffect si la lógica depende del idioma

**Interpolación:**
- Variables: `"upTo": "Hasta {{star}} estrellas"` → `t('key', { star: 5 })`
- Plurales: `"photo_one"`, `"photo_other"` → `t('key', { count })`

**Validación:**
- Usar `jq empty` para validar JSON
- No permitir claves duplicadas
- Consolidated en secciones únicas

**No se traducen:**
- Library names y URLs externas
- Keyboard symbols estándar (Esc, Space, Backspace)
- Términos técnicos de fotografía (ISO, F-Stop, SS, EV)
- Formatos ISO de fecha
