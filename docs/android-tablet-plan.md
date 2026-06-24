# Lumik para Android (tablets) — Estado actual

Objetivo cumplido: Lumik corre en Android sin romper el build de Linux/Windows.
Toda la separación usa `#[cfg(target_os = "android")]`.

---

## Alcance de esta fase

**En scope:** importar fotos RAW y JPEG, culling básico (rating + mover a `_culled/`),
visor de fotos con zoom/pan. Orientado a tablets en campo.

**Fuera de scope permanente:** conversión a DNG.
**Fuera de scope por ahora:** metadatos XMP del fotógrafo, histograma en tiempo real,
servidor de respaldo.

---

## BLOQUE 1 — Compilación base
**Estado:** ✅ Completo

- NDK 26.3 + Rust targets Android (`aarch64`, `armv7`, `i686`, `x86_64`)
- `tauri android init`, restructurado a `lib.rs` + `main.rs`
- `externalBin: dnglab` eliminado de `tauri.conf.json`
- `get_default_db_path()` para Android: usa `HOME` env var con fallback a `/data/data/com.lumik.desktop/`
- App lanza, UI visible, SQLite inicializa correctamente
- Build Linux/Windows sigue funcionando sin cambios

---

## BLOQUE 2 — Detección de dispositivos / almacenamiento externo
**Estado:** ✅ Completo

`scan_mounted_devices()` en `devices.rs` — rama `#[cfg(target_os = "android")]`:
- Lee `/proc/mounts`, filtra `vfat`/`exfat`/`fuse` bajo `/storage/` excluyendo `/storage/emulated` y `/storage/self`
- UUID del volumen = nombre del directorio en `/storage/<UUID>` (mismo formato que Linux)
- Espacio disponible con `libc::statvfs`
- Permisos en `AndroidManifest.xml`: `READ_EXTERNAL_STORAGE`, `WRITE_EXTERNAL_STORAGE`, `MANAGE_EXTERNAL_STORAGE`

**Nombre de dispositivos:** heurística por formato de UUID (FAT32 serial `XXXX-XXXX` → "SD Card", UUID largo → "USB Drive"). Para el nombre real del fabricante se requiere un plugin Kotlin para `StorageManager.getDescription()`.

**Retrocompatibilidad cross-disk:** `ProjectDatabase` almacena `mount_point` derivado del punto de montaje donde se encontró el `project.db`, no del `device_uuid` en BD. Permite abrir proyectos copiados entre discos sin error "device not mounted".

**Quirk de desarrollo:**
`tauri android dev` reinstala la app en cada rebuild y **resetea `MANAGE_EXTERNAL_STORAGE`** a denegado. Sin este permiso la app no puede escribir en el volumen.

```bash
# Después de cada rebuild con tauri android dev:
adb shell appops set com.lumik.desktop MANAGE_EXTERNAL_STORAGE allow
```

En producción el usuario lo otorga una sola vez: Ajustes → Apps → Lumik → Acceso especial → Archivos y medios.

---

## BLOQUE 3 — Pipeline de importación sin exiftool
**Estado:** ✅ Completo — RAW y JPEG probados en emulador y Huawei MatePad DBR-W09

### Solución implementada

Módulo `src/exif_android.rs` con dos backends según el tipo de archivo:

| Operación | Archivos RAW (DNG, CR2, RAF…) | Archivos JPEG |
|-----------|-------------------------------|---------------|
| `read_exif_rotation` | `rawler` → `RawMetadata.exif.orientation` | `kamadak-exif` → tag Orientation |
| Thumbnail (320px) | `rawler::extract_thumbnail_pixels` + resize + rotación física | `image::open()` + resize + rotación física |
| Preview full-res | `rawler::extract_preview_pixels`; fallback a thumbnail si no hay preview embebido | Lectura directa del original (sin caché permanente en `.previews/`) |
| EXIF batch (fecha, ISO, apertura…) | `rawler::RawMetadata` por archivo | `kamadak-exif` por archivo |
| XMP del fotógrafo en import | **skip** — datos en SQLite únicamente | **skip** |

**Dependencias agregadas:**
- `rawler = "0.6"` — decoder de archivos RAW
- `kamadak-exif = "0.5"` — lectura de EXIF en JPEGs planos (rawler no los soporta como archivos RAW)

**Por qué rawler no sirve para JPEGs:** rawler identifica JPEGs como JFIF pero falla en `get_decoder()` si la cámara no es un fabricante RAW conocido (p.ej. "Motorola", "Samsung"). Resultado: thumbnails, previews y metadata vacíos. Fix: bypass completo con `is_jpeg()` antes de llamar a rawler.

### Archivos modificados

**Rust:**
- `src/exif_android.rs` — módulo Android con rawler + kamadak-exif
- `src/lib.rs` — `mod exif_android` con `#[cfg(target_os = "android")]`
- `src/commands.rs`:
  - `#[cfg]` en `read_exif_rotation`, `cache_thumbnail`, `ensure_preview_cached`, `extract_exif_metadata_batch`
  - `jpeg_preview_bytes_no_cache()` — para JPEGs no se genera caché en `.previews/`; se lee el original con temp file para strip de Orientation
  - `jpeg_exif_metadata_batch()` — nuevo comando `get_platform()` → `std::env::consts::OS`
- `src/import/pipeline.rs` — `pipeline_metadata` y `pipeline_convert` son no-op en Android; `workspace_base_dir()` usa `$HOME/cache`
- `src/import/xmp.rs` — funciones gateadas con `#[cfg(not(target_os = "android"))]`
- `src/db/mod.rs` — `PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL` en project DBs (reduce escrituras en SD card de ~1200ms a ~50ms)

**Frontend:**
- `src/lib/api.ts` — `getPlatform()` + tipo `Platform`
- `src/lib/hooks.ts` — `usePlatform()` hook
- `src/pages/ImportPage.tsx` — `resolveContentUri()` convierte content URIs Android a file paths; `handleBrowse` con `[platform]` en deps
- `src-tauri/capabilities/default.json` — scope `fs` con `/storage/**`

### Bugs resueltos durante desarrollo

| Bug | Fix |
|-----|-----|
| `std::env::temp_dir()` devuelve `/tmp` (no existe en Android) | `workspace_base_dir()` usa `$HOME/cache` |
| `tauri-plugin-fs` no puede hacer `stat()` en `/storage/**` | Scope explícito en `capabilities/default.json` |
| `useCallback([], [])` capturaba `platform=null` (stale closure) | Dep array incluye `platform` |
| Content URIs (`content://...`) no son file paths | `resolveContentUri()` los convierte a `/storage/<uuid>/path` |
| rawler no procesa JPEGs de cámaras de teléfono | Bypass con `is_jpeg()` + kamadak-exif + image crate |
| `extract_exif_metadata_batch` en Linux/Windows con índices CSV fijos | Headers CSV parseados dinámicamente (exiftool omite columnas vacías del batch) |
| Preview de RAW sin preview embebido falla silencioso | Fallback a `extract_thumbnail_pixels` con logging de warning |
| Rotación EXIF no se guardaba en BD para JPEGs | `jpeg_exif_metadata()` via kamadak-exif incluye `rotation` |

---

## BLOQUE 4 — UI para tablet (touch)
**Estado:** ✅ Completo

- `Layout.tsx` — sidebar colapsado por default en Android/iOS; overlay absoluto en mobile; backdrop para cerrar con tap fuera
- `Sidebar.tsx` — transición CSS `width/padding 0.2s ease`; botón ✕ en header mobile
- `PhotoViewer.tsx`:
  - Pinch-to-zoom (2 dedos, centrado en punto medio del gesto)
  - Swipe horizontal (>60px, <400ms, ratio H/V >1.5) navega prev/next
  - RAF delay antes de `onRotationChange` → canvas rota visualmente antes del invoke()
- `PhotoSidebar.tsx` — `CullInput`, `StarInput`, `ColorInput` con targets ≥ 44px en mobile
- `PhotoDetail.tsx` — indicador de guardado: amarillo (`--lumik-secondary`) mientras guarda, verde al terminar, rojo en error

---

## Criterio de éxito

| Feature | Estado |
|---------|--------|
| APK compila, se instala en emulador y Huawei MatePad DBR-W09 | ✅ |
| App lanza sin crash | ✅ |
| SD card detectada con UUID y espacio disponible | ✅ |
| Proyectos creables en volumen externo | ✅ |
| Importar fotos RAW (CR2, RAF) | ✅ |
| Importar fotos JPEG | ✅ |
| Thumbnails generados al importar | ✅ |
| Metadata (fecha, cámara, ISO, apertura…) guardada en BD | ✅ |
| Rotación EXIF respetada en thumbnails y BD | ✅ |
| Preview en PhotoDetail | ✅ |
| Proyectos creados en Linux abribles en Android | ✅ |
| Proyectos copiados a otro disco abren sin error | ✅ |
| Culling (mover a `_culled/` y revertir) | ✅ |
| Rating (estrellas, color labels) | ✅ |
| Build Linux/Windows sin cambios | ✅ |

---

## Pendientes / issues conocidos

- **Inconsistencia thumbnail ↔ preview en Android para JPEGs con Orientation ≠ 1:** el thumbnail aplica rotación física al generar; el preview sirve el original con Orientation EXIF intacto → WebView puede auto-rotar mientras el canvas también aplica rotación de BD → doble rotación visual. Requiere investigación.
- **Nombre real del dispositivo:** actualmente heurística por UUID. Requiere plugin Kotlin para `StorageManager.getDescription()`.
- **APK debug universal muy grande (~1.4GB):** incluye 4 arquitecturas + debug symbols. Para distribución usar `--target aarch64` (tablet física) o release build.

---

## Configuración de entorno

```bash
# Rust targets (solo una vez)
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android

# Variables — agregar a ~/.zshrc
export ANDROID_HOME=/home/evesan/Android/Sdk
export NDK_HOME=/home/evesan/Android/Sdk/ndk/26.3.11579264
export JAVA_HOME=/home/evesan/.asdf/installs/java/openjdk-21
```

**`JAVA_HOME`** debe apuntar al JDK de asdf, no al JRE del sistema (`/usr/lib/jvm/java-21-openjdk-amd64` no tiene `javac` → Gradle falla con "does not provide the required capabilities: [JAVA_COMPILER]").

### Comandos

```bash
# Dev en emulador
cd apps/desktop && pnpm dev          # Terminal 1 — Vite
pnpm tauri android dev emulator-5554 # Terminal 2

# Re-otorgar permiso después de cada rebuild
adb shell appops set com.lumik.desktop MANAGE_EXTERNAL_STORAGE allow

# Build release para tablet física (aarch64)
pnpm tauri android build --target aarch64
# APK: src-tauri/gen/android/app/build/outputs/apk/arm64-v8a/release/
```
