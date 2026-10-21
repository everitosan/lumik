# Design

## Context

Ver `proposal.md` (Why). Estado actual relevante:

- `devices.rs::scan_mounted_devices` (escritorio) recorre `sysinfo::Disks`, descarta los de sistema con `should_skip_disk` y descarta en silencio cualquier disco sin UUID (`None => continue`).
- En macOS, `get_device_uuid_macos` ejecuta `diskutil info -plist <punto de montaje>` y lee `VolumeUUID`. Con el disco NTFS de prueba (`disk4`, particiones `Games` y `Photos`):
  - `mount` muestra `ntfs://disk4s4/Photos on /Volumes/Photos (lifs, local, read-only, …)`.
  - `diskutil info /Volumes/Photos` → "Could not find disk", exit 1.
  - `diskutil info -plist disk4s4` → `VolumeUUID A458E660-…`, `FilesystemType ntfs`, `WritableVolume false`.
  - `statvfs("/Volumes/Photos").f_flag & ST_RDONLY` → 1.
- `sysinfo` 0.31 no expone si un disco es de solo lectura, y en macOS `Disk::name()` es el nombre del volumen (`Photos`), no el dispositivo de origen.
- `commands.rs::refresh_open_projects` registra cada dispositivo en la BD global y llama a `discover_projects_on_device`, que abre cada `project.db` y le corre las migraciones, que escriben.
- `create_project` busca el dispositivo por UUID en `scan()`; `CreateProjectModal` ofrece todos los `DetectedDevice`.
- La barra lateral pinta cada disco con `DriveInfo` (`@lumik/ui`), que traduce sus textos internamente (`components.driveInfo.*`).
- `libc` solo es dependencia directa en Linux y Android.

## Goals / Non-Goals

**Goals:**
- Que un disco que el sistema monta deje de desaparecer en silencio por no resolver su UUID.
- Un único dato, `DetectedDevice.read_only`, que deciden el escaneo y del que dependen el descubrimiento, la creación y la UI.

**Non-Goals:**
- Abrir `project.db` en modo inmutable ni mostrar proyectos de discos de solo lectura.
- Cambiar el escaneo de Android.

## Decisions

### D1. UUID en macOS: por punto de montaje y, si falla, por nombre del disco

`get_device_uuid_macos` intenta primero `diskutil info -plist <punto de montaje>`, como hoy. Si falla, lee el dispositivo de origen del montaje (`f_mntfromname` con `statfs`, porque en macOS `Disk::name()` de `sysinfo` es el nombre del volumen), extrae el primer segmento con forma `disk\d+s\d+` (de `ntfs://disk4s4/Photos` o de `/dev/disk4s4`) y repite `diskutil info -plist disk4s4`. El parseo del identificador se separa en una función pura con tests.

- *Alternativa: obtener el UUID con DiskArbitration (`DADiskCreateFromBSDName` + descripción).* Evita el subproceso, pero el escaneo de macOS ya usa `diskutil` y el FFI de DA hoy vive en el watcher y en la expulsión. Se descarta para no ampliar el FFI en este change.

### D2. `read_only` con `statvfs` en Unix y `GetVolumeInformationW` en Windows

`DetectedDevice` gana `read_only: bool`:
- **Linux y macOS**: `statvfs(punto de montaje)` y `f_flag & ST_RDONLY`. Se agrega `libc` como dependencia directa de macOS (ya está en el `Cargo.lock`). Si `statvfs` falla, `false`.
- **Windows**: `GetVolumeInformationW` y el flag `FILE_READ_ONLY_VOLUME` (`windows-sys` ya incluye `Win32_Storage_FileSystem`). Si falla, `false`.
- **Android**: siempre `false`; no se toca su escaneo.

Ante la duda se elige `false`: el comportamiento queda como hoy, y el error de escritura de SQLite sigue avisando.

- *Alternativa: `WritableVolume` de `diskutil`.* Solo existe en macOS y obliga a otra consulta por disco. Se descarta.
- *Alternativa: probar a escribir un archivo temporal.* Escribiría en discos que sí admiten escritura solo por escanearlos. Se descarta.

### D3. Descubrimiento y creación omiten discos de solo lectura

- `refresh_open_projects` sigue registrando el dispositivo en la BD global (es la BD local, no el disco), pero no llama a `discover_projects_on_device` si `read_only`. `retain_mounted` no cambia: el disco sigue montado.
- `create_project` falla con "El dispositivo está montado en solo lectura" antes de crear carpetas si el dispositivo encontrado es `read_only`.
- La expulsión no cambia: `eject_device` usa el punto de montaje, igual que con los demás discos.

### D4. UI: marca y aviso en `DriveInfo`, filtro en `CreateProjectModal`

- `DriveInfo` gana `readOnly?: boolean`. Si está activo, muestra una etiqueta "Solo lectura" junto al nombre, con un `title` que contiene el aviso ("Lumik no puede abrir ni crear proyectos en este disco porque el sistema lo montó en solo lectura."). Los textos van en `packages/ui/src/i18n/locales/{es,en}.json` bajo `components.driveInfo`, siguiendo la regla de no pasar textos por props. Se agrega la variante a `apps/uikit/src/stories/DriveInfo.stories.tsx`.
- `Sidebar.tsx` pasa `readOnly={device.read_only}`.
- `CreateProjectModal` filtra los dispositivos con `read_only` antes de convertirlos en `Drive`. Si no queda ninguno, muestra el estado vacío que ya tiene.
- `DetectedDevice` en `src/lib/types.ts` gana `read_only: boolean`.

## Risks / Trade-offs

- **[DiskArbitration no resuelve volúmenes `lifs` por ruta al expulsar]** → La expulsión en macOS usa `DADiskCreateFromVolumePath`. Si falla con estos volúmenes, hoy devolvería "ya desmontado" (`Ok`) sin expulsar. Se verifica a mano (tarea 4.1); si falla, se crea el `DADisk` desde el identificador BSD de D1, que ya queda disponible.
- **[Un volumen que pasa a escritura mientras está montado]** → Raro (requiere remontar). El siguiente escaneo lo ve con escritura y descubre sus proyectos.
- **[Falsos negativos de `statvfs`]** → Se tratan como disco con escritura, que es el comportamiento actual.
