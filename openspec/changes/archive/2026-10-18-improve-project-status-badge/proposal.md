# Proposal

## Why

El badge de estado de las tarjetas de proyecto distingue `imported`, `edited` y `delivered` solo por el color, y los colores actuales (azul, amarillo, naranja) no comunican prioridad: un proyecto entregado llama la atención tanto como uno pendiente. Además, la etiqueta "Editando"/"Editing" usa una forma verbal distinta a la de los otros dos estados, y las stories de Storybook pasan valores de estado inválidos, así que el badge no se puede revisar ahí.

## What Changes

- El color de cada estado del badge se reasigna para que se lea como un semáforo de trabajo: `imported` en amarillo (pendiente), `edited` en azul (en curso) y `delivered` en gris (terminado, con menos peso visual).
- El badge muestra un icono por estado, además del texto, para que el estado no dependa solo del color: `import` para importado, `edit` para editado y `check` para entregado.
- La etiqueta de `edited` cambia de "Editando"/"Editing" a "Editado"/"Edited".
- Las stories de `ProjectCard` en el UI kit usan los valores válidos de `WorkflowStatus` (`imported`, `edited`, `delivered`) en lugar de `importada`/`editada`/`entregada`.
- Se elimina el mapa `statusLabels` con textos fijos en inglés que quedó sin uso en `ProjectCard.tsx`.

## Capabilities

### New Capabilities
<!-- Ninguna -->

### Modified Capabilities
- `project-management`: se añade el requisito "Representación visual del estado de flujo" (color, icono y etiqueta por estado). El requisito existente "Información del proyecto en el dashboard", que define cómo se calcula el estado, no cambia.

## Impact

- `packages/ui/src/components/ProjectCard/ProjectCard.tsx`: estilos e icono del badge.
- `packages/ui/src/i18n/locales/es.json` y `en.json`: etiqueta de `edited`.
- `apps/uikit/src/stories/ProjectCard.stories.tsx`: valores de estado válidos.
- Desktop (`Projects.tsx`) y portal web (`Dashboard.tsx`) heredan el cambio por el componente compartido; no requieren cambios propios.
- Fuera de alcance: el fallback del portal web que deduce el estado a partir del progreso de subida (`Dashboard.tsx:30-33`) y atenuar la tarjeta completa de los proyectos entregados.
