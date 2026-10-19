# Tasks

## 1. Etiqueta del estado editado

- [x] 1.1 Cambiar `components.projectCard.status.edited` a "Editado" en `packages/ui/src/i18n/locales/es.json` y a "Edited" en `en.json`, sin tocar las claves; verificar con `grep -n '"edited"' packages/ui/src/i18n/locales/*.json` que ya no aparecen "Editando" ni "Editing"

## 2. Badge en ProjectCard

- [x] 2.1 En `packages/ui/src/components/ProjectCard/ProjectCard.tsx`, sustituir los tres bloques de `statusBadgeStyles` por un estilo base (`inline-flex`, `gap: 4px`, padding/radio/fuente actuales) y un mapa estado -> color, fondo e icono, con los valores de la tabla de design.md (imported: `--lumik-secondary`, edited: `--lumik-primary`, delivered: `--lumik-outline`); verificar que `pnpm --filter @lumik/ui build` compila sin errores
- [x] 2.2 Renderizar dentro del badge un `Icon` de tamaño `sm` con `aria-hidden` antes de la etiqueta (`import`, `edit`, `check` según el estado); verificar con el build de `@lumik/ui` y revisando el JSX que el icono hereda el color con `currentColor`
- [x] 2.3 Eliminar el `statusLabels` de nivel de módulo con textos fijos en inglés; verificar con `grep -n "statusLabels" ProjectCard.tsx` que solo queda el mapa traducido dentro del componente y que el build sigue pasando

## 3. Storybook

- [x] 3.1 En `apps/uikit/src/stories/ProjectCard.stories.tsx`, reemplazar `importada`/`editada`/`entregada` por `imported`/`edited`/`delivered` en `argTypes`, en las stories y en la grilla; verificar que `pnpm --filter uikit build` (o el nombre del paquete en `apps/uikit/package.json`) compila sin errores de tipos
- [x] 3.2 Asegurar que exista una story con los tres estados uno junto a otro (la grilla actual o una nueva `AllStatuses`); verificar en `pnpm --filter uikit dev` que cada badge muestra el color, icono y etiqueta de la spec, en español y en inglés

## 4. Integración

- [x] 4.1 Abrir el dashboard del desktop (`apps/desktop`) con proyectos en los tres estados y comprobar que los badges cumplen los escenarios de la spec delta de `project-management`
- [x] 4.2 Abrir el dashboard del portal web (`apps/web`) y comprobar que un proyecto con el mismo estado muestra el mismo badge que en el desktop
- [x] 4.3 Añadir una sección "Sin publicar" en `CHANGELOG.md` (si no existe) con la entrada del nuevo badge de estado (colores, icono, "Editado"); verificar que el formato sigue Keep a Changelog como las entradas existentes

## Workflow follow-up

- Archivar el change con `/opsx:archive` tras revisar los badges, para incorporar el requisito "Representación visual del estado de flujo" a `openspec/specs/project-management/spec.md`.
