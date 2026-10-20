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
- **Metadatos EXIF/XMP**: `exiftool` (dependencia externa en runtime — debe estar instalado)
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

```
src-tauri/src/
  main.rs                           # entry point → lumik_lib::run()
  lib.rs                            # composition root: arma infra, elige impl por plataforma, registra comandos

  domain/                           # núcleo puro (sin SQLite/exiftool/Tauri/fs), 100% testeable
    photo.rs                        #   VOs: Rotation, Stars, normalize_tags
    paths.rs                        #   cull_destination (_media↔_culled), destinos de _trash/, path_to_slash
    project.rs                      #   ProjectFolder (slug/date_parts/path), compare_dashboard
    error.rs                        #   DomainError

  application/                      # orquestación; define los puertos (traits)
    ports.rs                        #   DeviceScanner, MetadataTool, ImageProcessor, FinderTagWriter,
                                    #   FileStore, PhotoRepository, ImportPipeline, ProgressReporter
    registry.rs                     #   ProjectRegistry (proyectos abiertos, ejecting, rotation-gen)
    use_cases/                      #   cull_photo, trash_photo, rate_photo, rotate_photo, import_photos,
                                    #   ensure_catalog_synced, sync_exported (con tests de fakes)

  infrastructure/                   # implementa los puertos (aquí vive el #[cfg] de plataforma)
    metadata.rs                     #   ExiftoolMetadata (desktop) / AndroidMetadata (rawler)
    imaging.rs                      #   DesktopImaging / AndroidImaging (thumbnails/previews)
    devices.rs, finder_tags.rs, folder.rs, fs.rs, progress.rs, import_pipeline.rs, persistence.rs
    backup_auth.rs, catalog.rs, backup_client.rs   # clientes HTTP del server de respaldo

  commands.rs                       # AppState + refresh_open_projects + declaración de submódulos
  commands/                         # comandos Tauri (adaptadores delgados) por dominio
    device.rs, project.rs, photo.rs, settings.rs, import_cmd.rs, backup.rs

  db/                               # persistencia SQLite (GlobalDatabase / ProjectDatabase)
    mod.rs, models.rs, queries.rs (photo_from_row), migrations.rs, *_schema.sql
  devices.rs                        # scan_mounted_devices() por plataforma (sysinfo / /proc)
  device_watch.rs                   # watcher nativo de hotplug
  apple_tags.rs                     # sidecars AppleDouble (desktop-only)
  exiftool.rs / exif_android.rs     # herramientas EXIF por plataforma
  import/                           # pipeline (passthrough → metadata → move), converter, xmp, progress
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
   `photo` (paths relativos al mount point) y `project_settings`.

**`AppState`** mantiene `global_db`, los puertos de infraestructura
(`device_scanner`, `metadata`, `image_processor`, `finder_tags`, `file_store`) y
un **`registry: Arc<ProjectRegistry>`** (`application/registry.rs`) que encapsula
el estado de sesión: proyectos abiertos (`open_projects`), dispositivos en
expulsión y el contador de generación del debounce de rotación. El mapa de
proyectos abiertos se puebla al arrancar y se refresca en cada
`scan_connected_devices`; si el disco se desmonta, sus proyectos desaparecen y el
comando devuelve error "device not mounted".

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
```

## Reglas técnicas críticas

- **Paths siempre relativos**: `dng_path` en BD es relativo al mount point. El path
  absoluto se reconstruye en runtime con `{mount_point}/{dng_path}`.
- **`dng_path` refleja la ubicación real**: cuando una foto se cullea, `dng_path` se
  actualiza a `_culled/{filename}`. No hay campo separado "culled_path".
- **`_media/` y `_culled/` son hermanas**: ambas viven directamente bajo el directorio
  del proyecto. Al cullarse, el archivo pasa de `_media/` a `_culled/`; al descullear, vuelve a `_media/`.
- **`_video/`**: carpeta donde van los videos. Se copian directamente sin conversión ni
  escritura de metadatos XMP.
- **exiftool es requerido en runtime**: thumbnails, previews, EXIF batch, orientación
  y metadata XMP dependen de él. Sin exiftool, la importación y el visor fallan.
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
  → pipeline_passthrough → workspace temporal (copia en formato original, sin conversión)
  → pipeline_metadata (exiftool renombra y embebe XMP del fotógrafo según los ajustes resueltos)
  → pipeline_move_to_dest → carpeta del proyecto en disco externo
  → MetadataTool::extract_batch (UN proceso exiftool para todos, salida CSV)
  → PhotoRepository::create_batch (UNA transacción SQLite)
  → cache_thumbnails_parallel (ImageProcessor, paralelo por CPU cores, max 8)
```

El auto-rename de la carpeta por fecha (usa `relocate_project_folder`, acoplado al
registry) se queda en el adaptador, tras el caso de uso.

El progreso se emite como evento Tauri `"import-progress"` con 3 fases gruesas
(no por archivo). El frontend lo recibe en `useImport()` hook.

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

## Flujos de producto ↔ comando ↔ caso de uso

Los flujos de `docs/useCases/` delegan en un caso de uso de
`application/use_cases/` (orquestación testeable con fakes). El resto de comandos
son de soporte/consulta (adaptadores directos a puertos/BD). El `invoke_handler`
de `lib.rs` está agrupado por flujo para reflejar esto.

| Flujo (`docs/useCases/`) | Comando (entry point)  | Caso de uso                  |
|--------------------------|------------------------|------------------------------|
| Importar Imágenes        | `start_import`         | `ImportPhotos`               |
| Crear Proyecto           | `create_project`       | — (lógica en el comando; `domain::ProjectFolder`) |
| Editar Proyecto          | `edit_project`         | `RenameProjectFiles` (con rollback), `RewriteProjectDescription` |
| Culling de imagen        | `save_photo_culled`    | `CullPhoto` (con rollback)   |
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
Proyectos: `get_projects_dashboard`, `get_project`, `create_project`, `edit_project`, `set_project_archived`, `delete_project`, `open_project_folder`, `get_project_cover_thumbnail`, `set_project_cover_photo`, `get_project_settings`, `update_project_settings`
Fotos: `get_project_photos`, `get_project_trashed_photos`, `trash_photo`, `restore_photo`, `reveal_photo`, `get_project_thumbnails`, `get_thumbnail`, `get_photo_preview`, `save_photo_rotation`, `save_photo_rating`, `save_photo_culled`, `regenerate_project_thumbnails`
Fotógrafo: `get_active_photographer`, `ensure_default_photographer`
Metadata: `get_photographer_metadata`, `update_photographer_metadata`
Ajustes: `get_app_settings`, `update_app_settings`, `get_dashboard_sort`, `update_dashboard_sort`, `get_dashboard_show_archived`, `set_dashboard_show_archived`, `get_sidebar_sections`, `set_sidebar_section_collapsed`
Keybindings: `get_keybindings`, `update_keybinding`
Import: `start_import`
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
