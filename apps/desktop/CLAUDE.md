# Lumik Desktop

App de escritorio Linux para administrar fotografías de fotógrafos profesionales.
Convierte RAW a DNG, escribe metadatos XMP, y permite organizar/culling.

## Stack

- **Framework**: Tauri v2 (backend Rust, frontend React+TypeScript)
- **Bundler**: Vite
- **BD global**: SQLite en `~/.local/share/lumik/lumik.db` (fotógrafo, ajustes, keybindings, dispositivos conocidos)
- **BD proyectos**: SQLite por proyecto, en cada disco externo (`project.db`)
- **Conversión RAW**: `dnglab` (binario empaquetado en `src-tauri/binaries/`)
- **Metadatos EXIF/XMP**: `exiftool` (dependencia externa en runtime — debe estar instalado)
- **UI**: `@lumik/ui` (paquete interno del monorepo en `packages/ui/`)

## Estructura de archivos relevantes

```
src/
  App.tsx                           # Raíz: routing entre Dashboard / ProjectDetail / Settings
  lib/
    api.ts                          # Todas las llamadas invoke() a Tauri (una función por comando)
    hooks.ts                        # Custom hooks async para datos + useImport + useKeybindings
    types.ts                        # Tipos TypeScript que espeja los modelos Rust
  pages/
    Dashboard.tsx                   # Grid de proyectos agrupados por año (renderizado normal, sin virtual scroll)
    ImportPage.tsx                  # Wizard de importación (2-3 pasos)
    SettingsPage.tsx                # Metadatos XMP, toggles, keybindings editables
    ProjectDetail/
      ProjectDetail.tsx             # Grid virtual de fotos (TanStack Virtualizer), agrupadas por día
      PhotoDetailView.tsx           # Visor canvas con zoom/pan/rotate + panel info + culling
      ProjectDetailHeader.tsx       # Barra superior con sort, filtro culled
      ProjectDetailFooter.tsx       # Contador de fotos, botón importar
  workers/
    histogram.worker.ts             # Web Worker: calcula histograma RGB de la imagen
  components/
    CreateProjectModal.tsx          # Modal para crear proyecto (nombre, fecha, dispositivo)
    Layout.tsx / Sidebar.tsx        # Shell de la app

src-tauri/src/
  main.rs                           # Inicializa BD global, fotógrafo default, open_projects, Tauri builder
  commands.rs                       # Todos los #[tauri::command] + AppState + lógica de importación
  db/
    mod.rs                          # GlobalDatabase y ProjectDatabase (wrappers sobre rusqlite)
    models.rs                       # Structs Rust (Photographer, Project, Photo, etc.)
    queries.rs                      # SQL de consulta/inserción
    global_schema.sql               # Schema de la BD global
    project_schema.sql              # Schema de la BD por proyecto
  devices.rs                        # scan_mounted_devices() — detecta discos montados vía lsblk
  import/
    pipeline.rs                     # Orquestación del pipeline (copy → convert → metadata → move)
    converter.rs                    # Invoca dnglab para conversión RAW→DNG
    xmp.rs                          # Escritura de metadatos XMP con exiftool
    hasher.rs                       # SHA-256 de archivos
    progress.rs                     # Tipos ImportProgress / ImportResult / ImportPhase
```

## Arquitectura de base de datos

**Dos tipos de BD coexisten:**

1. **BD Global** (`~/.local/share/lumik/lumik.db`):
   Fotógrafo activo, metadatos del fotógrafo, ajustes de la app (`AppSettings`),
   keybindings, registro de dispositivos conocidos (`known_device`).

2. **BD de Proyecto** (`{mount}/{año}/{mes}/{día}_{nombre}/project.db`):
   Una por proyecto, vive en el disco externo. Contiene la tabla `fotografia`
   con paths relativos al mount point.

**`AppState`** mantiene:
- `global_db: Arc<GlobalDatabase>`
- `open_projects: Arc<Mutex<HashMap<project_id, Arc<ProjectDatabase>>>>` — se
  puebla al arrancar y se refresca en cada `scan_connected_devices`. Si el disco
  se desmonta, sus proyectos desaparecen del mapa. Si el proyecto no está en
  el mapa, el comando devuelve error "device not mounted".

## Estructura de carpetas en disco externo

```
{mount_point}/lumik/{año}/{mes}/{día}_{nombre-proyecto}/
  project.db
  _media/
    IMG_0001.dng
    IMG_0002.dng
  _culled/
    IMG_0047.dng
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

## Pipeline de importación (`start_import` en commands.rs)

```
source files
  → pipeline_copy_files → workspace temporal
  → pipeline_convert (dnglab, si convert_to_dng=true)  |  pipeline_passthrough
  → pipeline_metadata (exiftool escribe XMP del fotógrafo, si embed_metadata=true)
  → pipeline_move_to_dest → carpeta del proyecto en disco externo
  → extract_exif_metadata_batch (UN proceso exiftool para todos, salida CSV)
  → create_photos_batch (UNA transacción SQLite)
  → cache_thumbnails_parallel (exiftool + image crate, paralelo por CPU cores, max 8)
```

El progreso se emite como evento Tauri `"import-progress"` con 3 fases gruesas
(no por archivo). El frontend lo recibe en `useImport()` hook.

## Flujo de navegación (frontend)

```
App
├── Dashboard (lista proyectos, renderizado normal con .map())
│     └── click proyecto → ProjectDetail
│           ├── click foto → PhotoDetailView
│           └── botón importar → ImportPage (embedded, sin paso "destino")
└── Settings
```

`App.tsx` maneja el routing con `useState`. No hay React Router.

## Visor de fotos (PhotoDetailView / PhotoViewer)

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

## Comandos Tauri disponibles

Dispositivos: `scan_connected_devices`, `get_known_devices`
Proyectos: `get_projects_dashboard`, `get_project`, `create_project`, `archive_project`, `delete_project`
Fotos: `get_project_photos`, `get_project_thumbnails`, `get_photo_preview`, `save_photo_rotation`, `save_photo_rating`, `save_photo_culled`, `regenerate_project_thumbnails`
Fotógrafo: `get_active_photographer`, `ensure_default_photographer`
Metadata: `get_photographer_metadata`, `update_photographer_metadata`
Ajustes: `get_app_settings`, `update_app_settings`
Keybindings: `get_keybindings`, `update_keybinding`
Import: `start_import`

## Estado del MVP — qué está implementado

| Feature | Estado |
|---------|--------|
| Import RAW → DNG + metadatos → disco externo | ✓ Funcional |
| Grid virtual de fotos agrupadas por día | ✓ Funcional |
| Visor canvas (zoom / pan / rotate / histograma) | ✓ Funcional |
| Rating: estrellas, etiquetas de color, tags | ✓ Funcional |
| Culling: mover a `_culled/` y revertir | ✓ Funcional |
| Keybindings configurables en Settings | ✓ Funcional |
| Metadata XMP del fotógrafo en import | ✓ Funcional |
| Menú contextual de proyectos en Dashboard | ✗ TODO (stub con console.log) |
| Operaciones en lote desde el grid | ✗ No implementado |
| Respaldo/API/portal web/iOS | ✗ Planificado (fuera del MVP) |

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
