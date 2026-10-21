# Tasks

## 1. Binding de DiskArbitration

- [x] 1.1 Agregar en `apps/desktop/src-tauri/Cargo.toml`, bajo `[target.'cfg(target_os = "macos")'.dependencies]`, `core-foundation` y `core-foundation-sys` con las versiones que ya resuelve el `Cargo.lock` (0.10 y 0.8). Verificar que `cargo check` en macOS pasa y que el `Cargo.lock` no agrega versiones nuevas de esos crates.
- [x] 1.2 Crear el módulo `#[cfg(target_os = "macos")]` con el FFI mínimo de DiskArbitration descrito en el design (D1), enlazado con `#[link(name = "DiskArbitration", kind = "framework")]`, y un wrapper seguro para crear una sesión y programarla en el run loop actual. Verificar con un test `#[cfg(target_os = "macos")]` que crea la sesión y obtiene un `DADisk` desde `/` sin fallar.

## 2. Watcher de macOS

- [x] 2.1 Agregar `watch_macos` a `device_watch.rs` y llamarlo desde `start` en un hilo propio (D2): registrar el callback de cambio de descripción filtrado por `kDADiskDescriptionVolumePathKey` y el de desaparición, con debounce de 300 ms antes de emitir `devices-changed`, y `warn!` si el registro falla. Actualizar el comentario del módulo (hoy dice que Android y "otros" no tienen listener). Verificar que `cargo clippy` en macOS pasa sin los warnings actuales de imports sin usar en `device_watch.rs`.
- [x] 2.2 Verificar a mano en macOS con `pnpm tauri dev`: con la app abierta, conectar el KINGSTON hace que el disco y sus proyectos aparezcan en menos de unos 2 s; expulsarlo desde Finder hace que desaparezcan en menos de unos 2 s; el log muestra el `info!` de arranque del watcher.

## 3. Expulsión en macOS

- [x] 3.1 Implementar la rama `#[cfg(target_os = "macos")]` de `os_eject` en `infrastructure/devices.rs` (D3): disco completo con `DADiskCopyWholeDisk`, `DADiskUnmount` con `kDADiskUnmountOptionWhole`, luego `DADiskEject`, esperando cada callback con `CFRunLoopRunInMode` y un límite de 30 s. Devolver `Ok` si el volumen ya no existe, `Err` con el texto del dissenter (o el código de estado traducido) si el desmontaje se rechaza, y `Ok` con `warn!` si solo falla la expulsión. Dejar la rama de fallback para el resto de plataformas sin cambios en Linux, Windows y Android. Verificar con `cargo test` que la traducción de códigos de estado a mensaje tiene tests unitarios para `kDAReturnBusy` y un código desconocido.
- [x] 3.2 Agregar al puerto `DeviceScanner` el método `devices_on_same_physical_disk(device_uuid)`, con una implementación por defecto que devuelve `[device_uuid]`, implementarlo en macOS comparando el BSD name del disco completo (D4), y hacer que `eject_device` marque como `ejecting` y cierre los proyectos de todos esos UUID. Actualizar la descripción del puerto en `docs/desktop-clean-architecture.md`. Verificar con un test del comando o del registry, usando un `DeviceScanner` falso que devuelve dos UUID hermanos, que se cierran los proyectos de ambos y que `ejecting` queda limpio al terminar, tanto en éxito como en error.
- [x] 3.3 Verificar a mano en macOS: expulsar el KINGSTON desde Lumik con proyectos abiertos lo desmonta (`mount` ya no muestra `/Volumes/KINGSTON`, `diskutil list external` ya no lo lista como montado) y no reaparece tras 60 s; con un archivo del disco abierto desde Terminal (`tail -f /Volumes/KINGSTON/x`), la expulsión falla, la UI muestra el error con el proceso si el sistema lo reporta, y el disco sigue listado; expulsar justo después de una importación funciona, lo que descarta handles abiertos de `exiftool -stay_open`.

## 4. `photo.device_uuid` obsoleto

- [x] 4.1 En `application/use_cases/import_photos.rs`, construir `CreatePhoto` con `device_uuid` vacío en lugar de `ImportParams.device_uuid` (D5), conservando `ImportParams.device_uuid` e `ImportRequest.device_uuid`. Actualizar la aserción `assert_eq!(cp.device_uuid, "devA")` del test existente para que espere `""`. Verificar que `cargo test` del crate desktop pasa completo. *(Ajuste: `device_uuid` se quitó de `ImportParams` y de `ImportRequest` en Rust porque quedaban sin leer; el frontend lo sigue enviando y serde lo ignora.)*
- [x] 4.2 Agregar en `db/project_schema.sql` el comentario de obsolescencia junto a `photo.device_uuid` y a `idx_photo_device` (sin cambiar el esquema ni las migraciones), y quitar `device_uuid` del `INSERT` descrito en `docs/useCases/flujo-importar-imagenes.md`, aclarando que el disco se resuelve en tiempo de ejecución. Verificar que no cambian `db/migrations.rs` ni la versión de esquema, y que el diagrama Mermaid se sigue renderizando. *(Ajuste: `device_uuid` sigue en el `INSERT` con valor `''` y la nota de obsolescencia, en lugar de quitarse, porque la columna se sigue escribiendo. Render del Mermaid no verificado localmente: no hay `mmdc`; los cambios son solo texto de etiquetas.)*
- [x] 4.3 Verificar a mano en macOS la portabilidad: copiar una carpeta de proyecto de prueba del KINGSTON a otro disco, en la misma ruta relativa bajo `lumik/` (por ejemplo `lumik/2026/10/10_sesion/`), conectar solo ese disco y comprobar que el proyecto aparece con sus fotos y su estado; importar fotos y comprobar con `sqlite3` sobre una copia del `project.db` que las filas nuevas tienen `device_uuid = ''`.

## 5. Documentación y cierre

- [x] 5.1 Actualizar `apps/desktop/CLAUDE.md`: mencionar el watcher de macOS (DiskArbitration) en la descripción de `device_watch.rs`, la expulsión en macOS y la nota de que `photo.device_uuid` está obsoleto y la identidad del proyecto es `project.id`. Verificar que la tabla de estado y la estructura de archivos siguen siendo coherentes con el código.
- [x] 5.2 Verificación integral en macOS y Linux: en macOS, conectar, ver los proyectos, importar, expulsar desde Lumik y reconectar; en Linux, repetir conectar y expulsar con el mismo disco para confirmar que nada cambió ahí y que los proyectos editados en Mac se abren bien. Verificar también `openspec validate macos-device-handling --strict`.

## Workflow follow-up

- Archivar el change con `/opsx:archive` cuando la verificación en macOS y Linux esté hecha.
