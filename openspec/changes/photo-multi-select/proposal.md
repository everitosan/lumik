# Proposal

## Why

Hoy cada acción sobre fotos se hace de una en una, desde el menú de cada tarjeta. Mandar a la papelera las 40 fotos movidas de una sesión, o juntar en un álbum las 20 mejores, son 40 o 20 viajes por el mismo menú. Con la nueva sección "Papelera" (change `trash-section`) pasa lo mismo al restaurar o eliminar. Falta una forma de actuar sobre varias fotos a la vez.

## What Changes

- **Marcar fotos** en el grid del proyecto, en el desktop y en la tablet. Se dice "marcar" y no "seleccionar", porque en Lumik "seleccionar" es aprobar (`_culled/`).
  - Desktop: Ctrl/Cmd+clic marca o desmarca una foto, Shift+clic marca el rango desde la última marcada, Esc quita las marcas (un segundo Esc regresa, como hoy), y cada tarjeta muestra una casilla al pasar el ratón.
  - Tablet: un toque largo entra al modo marcar con esa foto marcada, cada toque marca o desmarca, y "Listo" sale. El menú de la tarjeta sigue en su botón ⋮.
- **Barra de acciones** mientras hay fotos marcadas: "N fotos marcadas", "Agregar a álbum" (solo en el desktop: la tablet no tiene álbumes), "Mover a la papelera" y quitar las marcas. Con fotos marcadas, un clic normal o un toque no abre el visor.
- **Mover varias a la papelera** con una sola confirmación que dice cuántas. Si alguna falla, las demás se mueven igual y se informa cuáles fallaron y por qué.
- **Agregar varias a un álbum**, con el mismo diálogo de hoy. Las que ya estaban en el álbum no se repiten.
- **En la sección "Papelera"** se marcan fotos sueltas y proyectos para restaurarlos o eliminarlos definitivamente en lote. Eliminar en lote pide una confirmación con los conteos. Restaurar en lote no abre el modal de colisión por cada elemento: los que chocan se quedan en la papelera y se informan, y se pueden restaurar uno por uno con otro nombre.
- Las marcas se quitan al cambiar los filtros, al cambiar de proyecto o de sección, y al terminar una acción.

### Fuera de alcance

- Aprobar, estrellas, color y tags en lote.
- Marcar en los resultados de búsqueda y dentro de un álbum.
- Marcar todo con un atajo (Ctrl+A).
- Arrastrar para marcar un rectángulo.

## Capabilities

### New Capabilities
- `photo-marking`: marcar varias fotos en el grid del proyecto (desktop y tablet), la barra de acciones, cuándo se quitan las marcas y cómo conviven con abrir el visor y con los atajos.

### Modified Capabilities
- `photo-trash`: mover varias fotos marcadas a la papelera, con una confirmación y reporte de las que fallen.
- `photo-albums`: agregar a un álbum las fotos marcadas de un proyecto.
- `trash-bin` (capability nueva del change `trash-section`, todavía sin archivar): marcar elementos en la sección "Papelera" para restaurarlos o eliminarlos definitivamente en lote. **`trash-section` debe archivarse antes que este cambio**, porque esta delta agrega requisitos a esa capability.

## Impact

- `crates/lumik-core`: nada nuevo en los casos de uso. Los lotes recorren `TrashPhoto`, `RestorePhoto` y `PurgePhoto`, que ya existen.
- `apps/desktop` (Rust): comandos de lote `trash_photos`, `restore_trash_items` y `purge_trash_items`, que siguen con los demás si uno falla y devuelven un reporte por elemento.
- `apps/desktop` (React): estado de marcas en `ProjectDetail` (Ctrl/Cmd/Shift+clic, Esc antes que `project.back`), barra de acciones, `AddToAlbumDialog` con varios elementos, modal de confirmación de lote; marcas y barra en `pages/Trash/TrashPage.tsx`; i18n `es`/`en`.
- `packages/ui`: `PhotoCard` y `ProjectCard` con estado "marcada" (casilla y borde) y su story en `apps/uikit`.
- `apps/tablet`: modo marcar en `ProjectScreen` (el toque largo deja de abrir el menú) y en `TrashScreen`, la barra sin "Agregar a álbum", y lotes en `session.ts` y `projectActions.ts` que reutilizan una sola copia de `project.db` por proyecto.
- `docs/useCases/`: flujo nuevo de marcar y actuar en lote; actualizar los de papelera y álbumes.
- Sin cambios en `apps/server` ni `apps/web`.
