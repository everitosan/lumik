# Tasks

## 1. Escaneo: UUID en macOS y `read_only`

- [ ] 1.1 Agregar en `devices.rs` una función pura que extrae el identificador BSD (`disk\d+s\d+`) de `Disk::name()`, y usarla en `get_device_uuid_macos` como respaldo cuando `diskutil info -plist <punto de montaje>` falla (D1). Verificar con tests unitarios: `ntfs://disk4s4/Photos` → `disk4s4`, `/dev/disk2s1` → `disk2s1`, `map auto_home` → `None`.
- [ ] 1.2 Agregar `read_only: bool` a `DetectedDevice` y calcularlo con `statvfs` y `ST_RDONLY` en Linux y macOS (agregando `libc` a las dependencias de macOS en `Cargo.toml`), con `GetVolumeInformationW` y `FILE_READ_ONLY_VOLUME` en Windows, y `false` en Android (D2). Ajustar los `DetectedDevice` de los tests de `eject_device`. Verificar que `cargo test` pasa, que `cargo check` pasa en macOS, y con `cargo test test_scan_devices -- --nocapture` con el disco NTFS conectado, que `Photos` y `Games` aparecen con su `VolumeUUID` y `read_only = true`.

## 2. Descubrimiento y creación

- [ ] 2.1 En `commands.rs::refresh_open_projects`, seguir registrando el dispositivo en la BD global, pero no llamar a `discover_projects_on_device` si `read_only` (D3). Verificar con un test sobre la lógica extraída o, si no se puede sin `AppState`, comprobando en el log de `pnpm tauri dev` con el disco NTFS que no se abre ningún `project.db` suyo y que no aparecen archivos `-wal`/`-shm` nuevos en sus proyectos.
- [ ] 2.2 En `create_project`, fallar con "El dispositivo está montado en solo lectura" antes de crear carpetas si el dispositivo es `read_only`. Actualizar `docs/useCases/flujo-crear-proyecto.md` con esa rama. Verificar con `cargo check` y a mano que el comando rechaza el UUID del disco NTFS.

## 3. UI

- [ ] 3.1 Agregar `readOnly?: boolean` a `DriveInfo` en `packages/ui`, con la etiqueta "Solo lectura" y el aviso en `title`, y sus textos en `src/i18n/locales/es.json` y `en.json` bajo `components.driveInfo` (D4). Agregar la variante a `apps/uikit/src/stories/DriveInfo.stories.tsx`. Verificar con `jq empty` sobre los dos JSON y que Storybook muestra la variante.
- [ ] 3.2 Agregar `read_only: boolean` a `DetectedDevice` en `apps/desktop/src/lib/types.ts`, pasar `readOnly={device.read_only}` desde `Sidebar.tsx` y excluir esos dispositivos en `CreateProjectModal`. Verificar con `pnpm exec tsc --noEmit` en `apps/desktop`.
- [ ] 3.3 Actualizar `apps/desktop/CLAUDE.md` (escaneo de macOS con respaldo por identificador BSD, y discos de solo lectura listados sin proyectos). Verificar que la sección de dispositivos sigue siendo coherente con el código.

## 4. Verificación integral

- [ ] 4.1 Verificar a mano en macOS con el disco NTFS conectado y `pnpm tauri dev`: `Photos` y `Games` aparecen en la barra lateral con la marca "Solo lectura" y el aviso; sus proyectos no aparecen en el dashboard; el modal de crear proyecto no los ofrece; el KINGSTON sigue funcionando igual; expulsar el disco NTFS desde Lumik lo desmonta (`mount` ya no lo muestra). Si la expulsión no lo desmonta, crear el `DADisk` desde el identificador BSD de 1.1 y repetir.
- [ ] 4.2 Verificar en Linux (Xorg) que el mismo disco se lista sin la marca y sus proyectos se abren, y correr `openspec validate read-only-disks --strict`.

## Workflow follow-up

- Archivar el change con `/opsx:archive` cuando la verificación en macOS y Linux esté hecha.
