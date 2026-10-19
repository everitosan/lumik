# Proposal

## Why

Los iconos de la interfaz son SVGs copiados a mano del set Feather dentro de `Icon.tsx`. Feather ya no se mantiene, los dibujos se copiaron con atributos inconsistentes (algunos sin `strokeLinecap`/`strokeLinejoin` redondeados, `image` con trazo 1.5) y hay errores de significado: el icono `import` es en realidad la flecha de *subir* de Feather, que choca con importar fotos desde la tarjeta SD al disco (badge "Importado", botones de importar). Lucide es el fork mantenido de Feather, con el mismo estilo visual y miles de iconos, así que cambiar la fuente corrige estos problemas sin cambiar el lenguaje visual de la suite.

## What Changes

- `@lumik/ui` añade la dependencia `lucide-react`.
- `Icon` resuelve cada nombre de `IconName` a un componente de Lucide en lugar de un SVG escrito a mano. La API pública (`name`, `size`, `color`, props de SVG) y la lista de nombres no cambian, así que ninguno de los usos en desktop, portal web y uikit se modifica.
- El icono `import` pasa a mostrar una flecha que *entra* en una bandeja (Lucide `Import`), en lugar de la flecha de subir.
- Los iconos que Lucide no tiene con un dibujo equivalente se conservan como SVG propio dentro de `Icon` (hoy solo `sd-card`).
- La story `AllIcons` del UI kit muestra todos los iconos de `IconName` (hoy le faltan 12), para revisar la migración y detectar regresiones visuales. `@lumik/ui` exporta la lista de nombres para que la story no se desactualice.
- Pequeñas diferencias visuales esperadas: trazos con extremos redondeados en todos los iconos, `tags` pasa de relleno a trazo y `image` pasa de trazo 1.5 a 2.

## Capabilities

### New Capabilities
<!-- Ninguna -->

### Modified Capabilities
<!-- Ninguna: el cambio es de implementación visual. El requisito "Representación visual del estado de flujo" de project-management pide un icono de importación, de lápiz y de check, y se sigue cumpliendo. El change declara skip_specs. -->

## Impact

- `packages/ui/package.json` y `pnpm-lock.yaml`: nueva dependencia `lucide-react`.
- `packages/ui/src/components/Icon/Icon.tsx`: el mapa de iconos pasa a Lucide.
- `packages/ui/src/components/Icon/index.ts` y `packages/ui/src/index.ts`: exportan `iconNames` (adición, sin cambios incompatibles).
- `apps/uikit/src/stories/Icon.stories.tsx`: la story `AllIcons` usa la lista exportada.
- Desktop, portal web y uikit heredan los iconos nuevos sin cambios de código.
- Fuera de alcance: los tres checks dibujados a mano dentro de checkboxes (`ProjectDetailFooter`, `PhotoSidebar`, `Stepper`), el `Logo` y los logos de proveedores de `ProviderIcon` (react-icons).
