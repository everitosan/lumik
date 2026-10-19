# Proposal

## Why

La card de foto del grid no ofrece ninguna acción propia: para ubicar el archivo en disco hay que abrir la carpeta del proyecto y buscarlo a mano, y no existe forma de sacar del proyecto una foto que no sirve. Lumik promete no borrar nunca un original, así que quitar fotos tiene que ser reversible y no destructivo.

## What Changes

- La card de foto gana un menú de 3 puntos con el mismo patrón que la card de proyecto (cada acción aparece solo si la pantalla la ofrece).
- Opción **Mostrar en carpeta**: abre el explorador de archivos del SO con el archivo de la foto seleccionado. Solo en escritorio; en Android la opción no aparece.
- Opción **Mover a la papelera**: borrado lógico. Tras una confirmación neutra en un modal, la foto se marca como en la papelera y su original, su JPG exportado y sus sidecars se mueven a `_trash/` dentro de la carpeta del proyecto. Ningún archivo se borra.
- Una foto en la papelera desaparece del grid, de los conteos y del respaldo. Si era la portada del proyecto, el proyecto queda sin portada.
- Nuevo filtro **Papelera** en la barra de filtros: muestra solo las fotos en la papelera. En esa vista el menú de la card ofrece **Restaurar**, que devuelve los archivos a su sitio (`_media/` o `_culled/` según la aprobación, y `_exported/`) y la foto vuelve al proyecto.

### Fuera de alcance (pendiente)

- Propagar la papelera al catálogo del servidor y al portal web / iPad. Hoy el servidor solo permite insertar o actualizar fotos del catálogo, y el desktop no republica un proyecto ya publicado, así que una foto en la papelera seguirá visible en el portal. Las copias ya respaldadas se conservan.
- **Vaciar papelera** desde la app (borrado permanente): el fotógrafo vacía `_trash/` a mano si necesita espacio.
- Mover a la papelera o restaurar varias fotos a la vez.

## Capabilities

### New Capabilities
- `photo-trash`: papelera de fotos del proyecto y su restauración — qué archivos se apartan en `_trash/`, cómo se revierte, confirmación y efectos sobre portada, conteos y respaldo.

### Modified Capabilities
- `photo-viewer`: el grid de miniaturas incorpora un menú de acciones por foto y la acción "Mostrar en carpeta".
- `photo-filtering`: nuevo filtro de papelera, combinable con los demás.

## Impact

- `packages/ui`: `PhotoCard` (menú ⋮ con handlers opcionales, como `ProjectCard`), textos i18n y story en `apps/uikit`.
- `apps/desktop` (React): `ProjectDetail` (handlers del menú, toggle "Papelera", modal de confirmación), `lib/api.ts`, i18n `es`/`en`.
- `apps/desktop` (Rust): nuevos comandos para revelar una foto, moverla a la papelera y restaurarla; caso de uso de papelera/restaurar con rollback; consulta de fotos en la papelera; `infrastructure/folder.rs` usa `reveal_item_in_dir` de `tauri-plugin-opener` (ya es dependencia).
- Sin cambios en `apps/server` ni `apps/web` en este cambio.
