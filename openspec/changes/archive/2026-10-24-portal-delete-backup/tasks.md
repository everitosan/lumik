# Tasks

## 1. Servidor

- [x] 1.1 Agregar `Rclone::purge` (carpeta no encontrada = `false`, sin error) y `Rclone::rmdir`. Verificar con el test `purge_removes_dir_and_tolerates_missing` contra un remote `alias` local.
- [x] 1.2 Agregar `DELETE /v1/projects/{id}/backup`: dueño o 404, purga en cada remote antes de tocar el catálogo, borra las fotos del proyecto y limpia `backup_dir` y `workflow_status`. Verificar con el test de integración `delete_backup_removes_cloud_folder_and_catalog_photos` (borrar, borrar otra vez sin error, volver a subir, otro fotógrafo recibe 404).

## 2. Portal web

- [x] 2.1 Agregar `onDeleteBackup` a `ProjectCard` de `@lumik/ui` con el icono `cloud-off` y textos `es`/`en`. Verificar con `tsc` del paquete y la historia del Storybook.
- [x] 2.2 Agregar `backupApi.deleteBackup`, el modal `DeleteBackupModal` (confirmación con el nombre) y la opción en las tarjetas con fotos respaldadas del dashboard; recargar los proyectos al terminar. Verificar con `pnpm --filter @lumik/web build`.
- [x] 2.3 Documentar el flujo en `docs/useCases/wip-flujo-respaldar-exported.md`. Verificar que el bloque Mermaid parsea.

## 3. Verificación

- [x] 3.1 Prueba manual: borrar en Mega la carpeta de un proyecto respaldado, borrar su respaldo desde el portal y volver a respaldar desde el desktop; las fotos se suben de nuevo.
