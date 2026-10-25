# Design

## Context

El servidor guarda por proyecto un ancla `backup_dir` (la carpeta `_exported/` en la nube) y por foto su `backup_status`. El modal de respaldo del desktop y el del portal consultan ese catálogo para saber qué falta subir; nunca revisan la nube. Ver proposal.md - Why.

## Goals / Non-Goals

**Goals:**
- Dejar un proyecto listo para respaldarse de nuevo desde cero, haya o no carpeta en la nube.

**Non-Goals:**
- Borrar fotos sueltas del respaldo.
- Reconciliar el catálogo con lo que hay en la nube (verificar sin borrar).
- Actualizar el estado local del desktop en el momento.

## Decisions

- **Solo en el portal.** En la app el menú del proyecto está a mano mientras se trabaja; en el portal la acción queda separada del flujo diario. Se descartó ofrecerla también en el desktop.
- **Confirmar escribiendo el nombre.** Un clic de más no basta para algo que borra la nube.
- **`rclone purge` por proveedor y luego `rmdir` del padre.** Se borra la carpeta `_exported/` del ancla y, si quedó vacía, la carpeta del proyecto. `purge` con "directorio no encontrado" (código 3) cuenta como ya borrada, que es justo el caso que motivó el cambio. En Mega los archivos van a su papelera (sin `hard_delete`).
- **Primero la nube, después el catálogo.** Si un proveedor falla se responde error sin tocar el catálogo, así el fotógrafo puede reintentar y el catálogo nunca dice "nada subido" con archivos todavía en la nube.
- **El proyecto se queda en el catálogo.** Se borran sus fotos, el ancla `backup_dir` (el siguiente respaldo la recalcula con el nombre y la fecha actuales) y `workflow_status`. Borrar la fila obligaría al desktop a republicar y perdería la portada.

## Risks / Trade-offs

- [El desktop sigue mostrando los proveedores del proyecto hasta el siguiente respaldo] → Su modal de respaldo consulta el catálogo, así que ofrece subir todo; `finalize_backup` corrige el estado al terminar.
- [El desktop puede volver a publicar `workflow_status` del proyecto antes de respaldar] → Solo afecta al badge del portal hasta el siguiente respaldo.
- [Purgar una carpeta grande tarda] → El servidor no tiene límite de tiempo en la ruta y el modal muestra "Borrando…" hasta la respuesta.
