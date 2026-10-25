# Proposal

## Why

Cuando el fotógrafo borra a mano en la nube la carpeta de un proyecto, el catálogo del servidor sigue dando sus fotos por subidas y el respaldo no las vuelve a ofrecer, así que el proyecto queda sin poder respaldarse de nuevo. Hace falta una forma de borrar el respaldo de un proyecto que deje todo en orden, y debe estar en el portal web y no en la app, para no borrar respaldos por error mientras se trabaja.

## What Changes

- El dashboard del portal web ofrece "Borrar respaldo" en el menú de la tarjeta de los proyectos con fotos subidas, con confirmación escribiendo el nombre del proyecto.
- El servidor expone el borrado del respaldo de un proyecto: borra su carpeta en cada proveedor conectado (si ya no estaba, no es error) y sus fotos del catálogo, y limpia el estado fijado. Si un proveedor falla, el catálogo no se toca.
- La app de escritorio no ofrece esta acción.

## Capabilities

### New Capabilities

_Ninguna._

### Modified Capabilities

- `web-portal`: nuevo requisito para borrar el respaldo de un proyecto desde el dashboard.

## Impact

- Servidor (`apps/server`): nueva ruta `DELETE /v1/projects/{id}/backup`; rclone `purge` y `rmdir`; test de integración con un remote `alias` local.
- Portal web (`apps/web`): modal de confirmación, opción en la tarjeta, cliente `backupApi.deleteBackup`, textos `es`/`en`.
- `@lumik/ui`: `ProjectCard` acepta `onDeleteBackup` (opción del menú con el icono `cloud-off`).
- Desktop: sin cambios. Su estado local del proyecto se corrige en el siguiente respaldo.
