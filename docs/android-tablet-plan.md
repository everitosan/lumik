# Plan: Lumik para Android (tablets)

Objetivo de viabilidad: compilar y correr Lumik en una tablet Android
sin romper el build de Linux/Windows existente.
Toda la separación usa `#[cfg(target_os = "android")]`.

---

## Alcance de esta fase

**En scope:** importar fotos RAW, culling básico (rating + mover a `_culled/`),
visor de fotos con zoom/pan. Orientado a tablets en campo.

**Fuera de scope permanente:** conversión a DNG (se eliminará también del desktop).  
**Fuera de scope por ahora:** metadatos XMP del fotógrafo, histograma en tiempo real,
keybindings, servidor de respaldo.

---

## Bloqueantes críticos (orden de ataque)

### BLOQUE 1 — Prueba de compilación base
**Estado:** ✅ Completo — app lanza, UI visible, proyectos creables en emulador

**Acciones:**
- [x] Instalar NDK 26.3 y Rust targets Android (`aarch64`, `armv7`, `i686`, `x86_64`)
- [x] Inicializar Android target con `tauri android init`
- [x] Restructurar a `lib.rs` + `main.rs` (requerido por Tauri mobile)
- [x] Eliminar `externalBin: dnglab` de `tauri.conf.json` (dnglab se elimina del proyecto)
- [x] Apuntar `JAVA_HOME` al JDK de asdf (`/home/evesan/.asdf/installs/java/openjdk-21`)
- [x] APK generado: `gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk` (14 MB)
- [x] Fix `get_default_db_path()` para Android: usa `HOME` env var con fallback a `/data/data/com.lumik.desktop/`
- [x] App lanza, UI visible, SQLite inicializa correctamente
- [x] Build Linux sigue funcionando sin cambios

---

### BLOQUE 2 — Detección de dispositivos / almacenamiento externo
**Estado:** ✅ Completo — SD card detectada, espacio disponible, proyectos creables

**Lo que se implementó:**
- `scan_mounted_devices()` en `devices.rs` tiene rama `#[cfg(target_os = "android")]`
  que lee `/proc/mounts` (accesible sin permisos especiales) y filtra volúmenes
  `vfat`/`exfat`/`fuse` bajo `/storage/` excluyendo `/storage/emulated` y `/storage/self`.
- El nombre del directorio en `/storage/<UUID>` es directamente el UUID del volumen,
  mismo formato que Linux — sin cambios al schema.
- Espacio disponible obtenido con `libc::statvfs` (dependencia solo para Android).
- Permisos en `AndroidManifest.xml`: `READ_EXTERNAL_STORAGE` (≤ API 32),
  `WRITE_EXTERNAL_STORAGE` (≤ API 32), `MANAGE_EXTERNAL_STORAGE` (API 30+).

**Quirk de desarrollo importante:**
`tauri android dev` reinstala la app en cada rebuild y **resetea el permiso
`MANAGE_EXTERNAL_STORAGE`** a `default` (denegado). Sin este permiso la app
puede leer el volumen pero no escribir → falla al crear `project.db`.

Después de cada rebuild, re-otorgar manualmente:
```bash
adb shell appops set com.lumik.desktop MANAGE_EXTERNAL_STORAGE allow
```

Para producción (APK firmado instalado permanentemente) el usuario otorga el
permiso una sola vez desde Ajustes → Apps → Lumik → Acceso especial → Archivos y medios.

---

### BLOQUE 3 — Reemplazar `exiftool`
**Estado:** pendiente

**Archivos:** `src-tauri/src/import/xmp.rs`, `commands.rs` (thumbnails, previews)  
`exiftool` es una dependencia de runtime que no puede instalarse en Android.

Se usa para tres cosas distintas:

| Uso | Solución Android |
|-----|-----------------|
| Leer EXIF (capture_date, orientation, etc.) | `kamadak-exif` crate |
| Escribir metadatos XMP del fotógrafo en import | omitir en v1 |
| Extraer preview JPEG embebido del RAW/DNG | `rawler` (ya en el proyecto) |

**Estrategia cfg:**
```toml
[target.'cfg(target_os = "android")'.dependencies]
kamadak-exif = "0.5"
```

```rust
#[cfg(not(target_os = "android"))]
fn extract_exif_metadata(path: &Path) -> Result<ExifData> { /* exiftool */ }

#[cfg(target_os = "android")]
fn extract_exif_metadata(path: &Path) -> Result<ExifData> { /* kamadak-exif */ }
```

---

### BLOQUE 4 — UI para tablet (touch)
**Estado:** pendiente — no requiere código Rust, es trabajo de frontend React

**Cambios mínimos para v1:**
- [ ] Desactivar el sistema de keybindings en Android
- [ ] Gestos touch en el visor de fotos (pinch-to-zoom, swipe entre fotos)
- [ ] Botones de culling con targets ≥ 44px
- [ ] Layout landscape con panel lateral colapsable

---

## Orden de ejecución

```
BLOQUE 1 ✅  →  BLOQUE 2 ✅  →  BLOQUE 3  →  BLOQUE 4
   build           devices       exiftool      UI touch
   base            detection     replace
```

---

## Configuración de entorno

```bash
# Rust targets Android (solo una vez)
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android

# Variables de entorno — agregar a ~/.zshrc
export ANDROID_HOME=/home/evesan/Android/Sdk
export NDK_HOME=/home/evesan/Android/Sdk/ndk/26.3.11579264
export JAVA_HOME=/home/evesan/.asdf/installs/java/openjdk-21
```

### Por qué estas rutas exactas

**`ANDROID_HOME`**: SDK en `~/Android/Sdk/` (instalado via Android Studio).

**`NDK_HOME`**: NDK r26 instalado vía `sdkmanager`. Tauri v2 requiere r26 mínimo.
La ruta incluye la versión exacta porque pueden coexistir varias en `~/Android/Sdk/ndk/`.

**`JAVA_HOME`**: el sistema tiene dos instalaciones Java:
- `/usr/lib/jvm/java-21-openjdk-amd64` → JRE del sistema (sin `javac`)
- `/home/evesan/.asdf/installs/java/openjdk-21` → JDK completo vía asdf ✅

Gradle requiere `[JAVA_COMPILER]`. Si `JAVA_HOME` apunta al JRE del sistema falla con:
```
Toolchain installation '...' does not provide the required capabilities: [JAVA_COMPILER]
```

### Comando de dev (emulador)

```bash
# Terminal 1 — levantar Vite primero
cd apps/desktop && pnpm dev

# Terminal 2 — compilar y desplegar
pnpm tauri android dev emulator-5554

# Después de cada rebuild: re-otorgar permiso de escritura
adb shell appops set com.lumik.desktop MANAGE_EXTERNAL_STORAGE allow
```

### Comando de build release

```bash
pnpm tauri android build --target aarch64
# APK en: src-tauri/gen/android/app/build/outputs/apk/universal/release/
```

---

## Criterio de éxito de esta fase

- [x] APK compila y se instala en Android
- [x] App lanza sin crash
- [x] SD card detectada con UUID y espacio disponible
- [x] Proyectos creables en el volumen externo
- [ ] Importar una foto RAW desde el almacenamiento (requiere BLOQUE 3)
- [ ] Ver y cullear la foto importada
- [ ] `tauri build` (Linux) sigue funcionando sin cambios ✅
