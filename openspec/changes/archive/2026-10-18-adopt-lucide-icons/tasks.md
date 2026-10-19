# Tasks

## 1. Dependencia

- [x] 1.1 Añadir `lucide-react@^1.51.0` a las dependencias de `@lumik/ui` con `pnpm --filter @lumik/ui add lucide-react@^1.51.0`; verificar que aparece en `packages/ui/package.json` y en `pnpm-lock.yaml`

## 2. Icon sobre Lucide

- [x] 2.1 En `packages/ui/src/components/Icon/Icon.tsx`, sustituir el mapa de SVGs por un mapa `IconName -> componente` con la correspondencia de la tabla de design.md (`import` -> `Import`, `drive` -> `Database`, `projects` -> `LayoutGrid`, `edit` -> `SquarePen`, `trash` -> `Trash2`, `menu` -> `EllipsisVertical`, `tags` -> `Tag`, `upload-cloud` -> `CloudUpload`, `image`/`photo` -> `Image`, `star-filled` -> `Star` relleno, etc.), sin cambiar `IconName`, `IconProps` ni el cálculo de tamaño y color; verificar que `pnpm --filter @lumik/ui build` compila
- [x] 2.2 Conservar `sd-card` como SVG propio en el mapa, con `strokeLinecap` y `strokeLinejoin` redondeados como los de Lucide; verificar con el build y revisando que es la única entrada que no viene de `lucide-react`
- [x] 2.3 Exportar `iconNames` (claves del mapa como `IconName[]`) desde `Icon/index.ts` y `packages/ui/src/index.ts`; verificar con el build que el `.d.ts` generado incluye `iconNames`
- [x] 2.4 Comprobar que los consumidores compilan sin cambios: `pnpm exec tsc --noEmit` en `apps/desktop` y `apps/web` sin errores nuevos relacionados con `Icon` (comparar con la salida antes del cambio si ya había errores previos)

## 3. Storybook

- [x] 3.1 En `apps/uikit/src/stories/Icon.stories.tsx`, cambiar la lista `allIcons` escrita a mano por `iconNames` de `@lumik/ui`; verificar que `pnpm --filter @lumik/uikit build` compila y que la story `AllIcons` muestra los 33 iconos
- [x] 3.2 Revisar la story `AllIcons` en `pnpm --filter @lumik/uikit dev`, comparándola con la versión anterior (captura previa de la misma story): `import` muestra la flecha entrando en la bandeja, `star-filled` sale relleno, `sd-card` no desentona y ningún icono sale vacío o desalineado

## 4. Integración

- [x] 4.1 Revisar en el desktop las pantallas con más iconos (barra lateral, página de importación, detalle de proyecto con estrellas y rotación, detalle de foto con `tags`) y confirmar que se ven bien en tamaños `sm` y `md`
- [x] 4.2 Revisar en el portal web el dashboard (badge de estado con el icono `import` nuevo) y confirmar que coincide con el desktop
- [x] 4.3 Añadir en `[Sin publicar]` de `CHANGELOG.md` la entrada del cambio a Lucide y la corrección del icono de importar; verificar que sigue el formato de las entradas existentes
