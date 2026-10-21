# Proposal

## Why

Un disco externo NTFS con dos particiones aparece en Finder, pero Lumik en macOS no lo lista; en Linux sí, y muestra sus proyectos. Hay dos causas. macOS monta NTFS con su driver nativo, que se reporta como `ntfs://disk4s4/Photos` (tipo `lifs`) en lugar de `/dev/disk4s4`, y `diskutil info <punto de montaje>` falla con "Could not find disk", así que el escaneo no obtiene el UUID y descarta el disco sin avisar. Además, ese driver solo lee (`WritableVolume = false`), y Lumik necesita escribir para abrir un proyecto: el `project.db` en modo WAL, las migraciones, las calificaciones, la importación y las miniaturas.

## What Changes

- **Detección en macOS de volúmenes del driver NTFS nativo**: el UUID se obtiene por el disco (`disk4s4`) cuando la consulta por punto de montaje falla, así que estos volúmenes aparecen en la lista con su UUID de volumen.
- **Discos de solo lectura**: el escaneo marca cada disco como de solo lectura cuando el sistema lo monta así, en cualquier plataforma de escritorio. Un disco de solo lectura se lista en la barra lateral con una marca y un aviso que explica que Lumik no puede abrir ni crear proyectos en él.
- **Sin proyectos en discos de solo lectura**: no se descubren ni abren sus proyectos (no se toca su `project.db`), y no se ofrecen como destino al crear un proyecto.
- **Expulsar** un disco de solo lectura sigue disponible igual que con cualquier otro disco.

### Fuera de alcance

- Abrir proyectos en modo de solo lectura (ver fotos y miniaturas sin editar).
- Escribir en NTFS desde macOS; requiere drivers de terceros, con los que el volumen se monta con escritura y Lumik lo trata como un disco normal.
- Android: sus volúmenes de `/storage/` no cambian.

## Capabilities

### New Capabilities

_Ninguna._

### Modified Capabilities

- `device-detection`: "Detección de dispositivos externos" agrega el escenario de un volumen NTFS montado por macOS. Nuevo requisito "Discos de solo lectura": marca, aviso, sin proyectos y expulsión disponible.
- `project-management`: "Crear proyecto en un dispositivo montado" exige además que el dispositivo admita escritura.

## Impact

- `apps/desktop/src-tauri/src/devices.rs`: `DetectedDevice` gana `read_only`; en macOS, respaldo de UUID por el nombre del disco y detección de solo lectura con `statvfs`; en Linux, `statvfs`; en Windows, `GetVolumeInformationW`.
- `apps/desktop/src-tauri/Cargo.toml`: `libc` también para macOS (ya está en el `Cargo.lock`).
- `apps/desktop/src-tauri/src/commands.rs` (`refresh_open_projects`) y `commands/project.rs` (`create_project`): omitir discos de solo lectura.
- Frontend: `DetectedDevice.read_only` en `src/lib/types.ts`; la barra lateral pasa `readOnly` a `DriveInfo`; `CreateProjectModal` excluye esos discos.
- `packages/ui`: `DriveInfo` gana `readOnly`, con marca y aviso traducidos en `es.json`/`en.json`, y su historia en `apps/uikit`.
- Sin cambios en `apps/server`, `apps/web` ni en el formato de `project.db`.
