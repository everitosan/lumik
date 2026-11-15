# Proposal

## Why

Lumik promete no borrar nunca un original sin que el fotógrafo lo pida. Las fotos ya cumplen: van a la papelera del proyecto. Pero "Eliminar proyecto" borra la carpeta completa para siempre en un solo paso, tanto en el desktop como en la tablet. Es justo la operación con más que perder. Además, la papelera de fotos solo se ve proyecto por proyecto, y nada de lo que se manda ahí se puede borrar desde la app para recuperar espacio.

## What Changes

- **Borrar en dos pasos**, para fotos y proyectos: primero se manda a la papelera (reversible) y desde ahí se elimina definitivamente. Lumik nunca vacía la papelera por su cuenta.
- **BREAKING**: "Eliminar proyecto" pasa a ser "Mover a la papelera". La carpeta del proyecto se mueve en el mismo disco a `lumik/_trash/`, bajo una carpeta con la fecha y hora en que se mandó y con su ruta bajo `lumik/`, para que no choque con otra. El proyecto sale del dashboard y de la búsqueda.
- **Nueva sección "Papelera"** en el menú izquierdo del desktop y en el rail de la tablet, justo antes de la configuración. Solo aparece cuando hay algo en la papelera de los discos conectados. Muestra los proyectos y las fotos sueltas en la papelera, agrupados por disco conectado. Cada elemento ofrece "Restaurar" y "Eliminar definitivamente", y cada disco, "Vaciar papelera de {disco}". Las fotos de un proyecto en la papelera no se listan aparte: viajan con él.
- **BREAKING**: se quita el filtro "Papelera" de la barra de filtros del proyecto. Para ver y restaurar fotos en la papelera se usa la nueva sección.
- **Restaurar sin sobrescribir**: si el destino ya está ocupado, un modal avisa de la colisión y pide otro nombre. Para un proyecto se pide otro nombre de proyecto; para una foto, otro nombre de archivo.
- **Eliminar definitivamente no deja huella**: una foto se borra del disco (original, sidecars, exportado, miniatura y preview) y de los registros (su fila y sus datos en `project.db`, y el índice de búsqueda). Un proyecto se borra con su carpeta completa y sale del índice.
- **Solo el disco local**: el estado de la papelera vive en cada disco, sin copia en la base global. Lo que hay en un disco desconectado no se ve. Eliminar definitivamente no toca el respaldo en la nube, y los avisos lo dicen: se borra desde el portal web.
- **La tablet Android sigue las mismas reglas**: hoy borra la carpeta del proyecto directamente, y al buscar proyectos entraría a `lumik/_trash/`.

### Fuera de alcance

- Papelera o "Deshacer" para álbumes. Un álbum no tiene archivos; sus elementos que apuntan a algo en la papelera o eliminado siguen marcados como hoy.
- Ver o vaciar la papelera de discos desconectados.
- Borrar el respaldo en la nube desde el desktop o la tablet.
- Mandar a la papelera, restaurar o eliminar varios elementos a la vez, salvo "Vaciar papelera".
- Vaciado automático por antigüedad.

## Capabilities

### New Capabilities
- `trash-bin`: la sección "Papelera" de la app: qué lista y cómo lo agrupa por disco, restaurar (con el modal de colisión), eliminar definitivamente y vaciar la papelera de un disco, con sus confirmaciones y avisos sobre el respaldo.

### Modified Capabilities
- `project-management`: "Eliminar proyecto" se reemplaza por mandar el proyecto a la papelera (`lumik/_trash/{fecha}/{ruta}`). Un proyecto en la papelera no se lista en el dashboard.
- `photo-trash`: restaurar se hace desde la sección "Papelera" y no desde un filtro del proyecto. Una colisión al restaurar pide otro nombre en vez de solo fallar. Se agrega el borrado definitivo de una foto en la papelera.
- `photo-filtering`: se quita el filtro "Papelera" y su combinación con los demás filtros.
- `photo-search`: los proyectos en la papelera salen del índice y vuelven al restaurarse; lo eliminado definitivamente no queda en él.

## Impact

- `crates/lumik-core`: casos de uso para mandar un proyecto a la papelera y restaurarlo (sobre el `rename` que ya usa `relocate_project_folder`), listar la papelera de un disco (`lumik/_trash/` y las fotos con `deleted = 1` de cada proyecto), eliminar definitivamente fotos y proyectos, y restaurar con otro nombre. Las reglas de rutas van en `domain/paths.rs`. `discover_projects_on_device` ya se salta las carpetas que empiezan con `_`.
- `apps/desktop` (Rust): `delete_project` deja de borrar y pasa a mandar a la papelera; comandos nuevos para listar, restaurar, eliminar definitivamente y vaciar. Se respeta el candado del proyecto (`ensure_not_locked`) y se libera la sesión.
- `apps/desktop` (React): entrada "Papelera" en `Sidebar.tsx` y página nueva; el modal de borrar proyecto cambia de texto; sale el toggle "Papelera" de `ProjectDetail`; i18n `es`/`en`.
- `apps/tablet` y `crates/lumik-mobile`: `deleteProject` en `projectActions.ts` deja de borrar; `projects.ts` (`walk`) se salta `_trash`; sección "Papelera" en el rail de `HomeScreen.tsx`; sale la vista de papelera dentro del proyecto (`AppScreen`/`ProjectScreen`).
- `packages/ui` / `apps/uikit`: componentes de la sección (tarjetas de proyecto y foto en la papelera, grupo por disco) con sus stories.
- Sin cambios en `apps/server` ni `apps/web`.
- `docs/useCases/`: actualizar los diagramas de borrado de proyecto y de papelera.
