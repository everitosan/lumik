# Proposal

## Why

Con el botón "Archivados (N)" y el desplegable "Ordenar", la cabecera del dashboard reúne título, contadores, buscador, dos controles de filtro y "Nuevo proyecto". En pantallas chicas todo queda pegado. Además, no hay un sitio previsto para los filtros que vengan después. El detalle de proyecto ya resuelve esto con una barra inferior (contadores e interruptores a la izquierda, desplegables a la derecha).

## What Changes

- El dashboard añade una **barra inferior** con el mismo diseño que el pie del detalle de proyecto. Los controles de filtro se alinean a la derecha, separados por una línea vertical: el botón "Archivados (N)" (el mismo de antes, que solo aparece si hay archivados) y "Ordenar", cuyo desplegable pasa a abrir hacia arriba.
- La cabecera conserva el título y los contadores ("12 proyectos · 3,420 fotos") y deja de tener los controles de filtro: queda título con contadores, buscador y "Nuevo proyecto".
- Se extraen a `@lumik/ui` dos componentes, usados por los dos pies:
  - `FilterBar`: la barra con zonas izquierda y derecha y su separador.
  - `FilterToggle`: el interruptor redondo con etiqueta (hoy lo usa "Mostrar solo aprobadas" del detalle).
  `ProjectDetailFooter` pasa a usarlos y debe verse igual que hoy.
- En pantallas estrechas, las dos zonas de la barra pasan a dos líneas en lugar de apretarse (en los dos pies).
- El comportamiento no cambia: filtrado, orden, persistencia y conteos siguen igual.

## Capabilities

### New Capabilities
<!-- Ninguna -->

### Modified Capabilities
- `project-management`: el escenario de estadísticas de "Archivar proyectos" deja de situar el contador "en la cabecera". El comportamiento no cambia.

## Impact

- `@lumik/ui`: `FilterBar` y `FilterToggle` nuevos, exportados en `index.ts`, con stories en `apps/uikit`.
- Frontend desktop: `pages/Project/Projects/Projects.tsx` (estructura de la página, controles de filtro de la cabecera a la barra inferior y `SortMenu` abriendo hacia arriba) y `pages/Project/ProjectDetail/ProjectDetailFooter.tsx` (usa los componentes nuevos sin cambio visual).
- Sin cambios en backend, base de datos ni i18n.
