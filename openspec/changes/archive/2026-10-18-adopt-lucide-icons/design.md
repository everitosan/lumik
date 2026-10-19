# Design

## Context

`Icon` (`packages/ui/src/components/Icon/Icon.tsx`) define el tipo `IconName` con 33 nombres y un mapa `nombre -> función que devuelve un <svg>`. El componente calcula el tamaño (`sm` 12, `md` 16, `lg` 20, `xl` 24 o un número), aplica `color ?? 'currentColor'` y `flexShrink: 0`, y pasa el resto de las props al SVG. Hay unos 60 usos en desktop, portal web y `@lumik/ui`, incluidos nombres calculados (`name={filled ? 'star-filled' : 'star'}`). Por eso la API tiene que seguir igual.

Revisé el paquete `lucide-react`: tiene más de 4 000 iconos, licencia ISC, `sideEffects: false` (se empaqueta solo lo que se importa) y es compatible con React 19.

## Goals / Non-Goals

**Goals:**
- Cambiar la fuente de los dibujos sin tocar a ningún consumidor de `Icon`.
- Que `IconName` siga siendo la lista cerrada de iconos de la suite, para que el vocabulario visual no crezca sin control.

**Non-Goals:**
- Exponer Lucide directamente a las apps (importar `lucide-react` desde desktop o web).
- Renombrar iconos de `IconName`, aunque algunos nombres no coincidan con los de Lucide.
- Migrar los checks dentro de checkboxes, el `Logo` o `ProviderIcon`.

## Decisions

**`Icon` sigue siendo la capa intermedia; el mapa apunta a componentes de Lucide.** El mapa pasa a ser `Record<IconName, ComponentType<SVGProps>>` con componentes importados por nombre (`import { Search, Import, ... } from 'lucide-react'`). Alternativa descartada: que las apps importen de `lucide-react` directamente. Eso tocaría los 60 usos y abriría la puerta a cualquier icono.

**Tamaño y color se siguen pasando como hasta ahora.** El estilo combinado (`width`, `height`, `color`, `flexShrink`) se pasa por `style` igual que hoy, y Lucide pinta con `currentColor` por defecto. No se usa la prop `size` de Lucide, para que `style` tenga la última palabra como hoy.

**Correspondencia de iconos:**

| IconName | Lucide | Nota |
|---|---|---|
| search, settings, calendar, aperture, check, x, plus, star, info, folder, folder-open, zoom-in, zoom-out, rotate-ccw, rotate-cw, eject | mismo nombre en PascalCase | Mismo dibujo que Feather |
| chevron-down/right/left | `ChevronDown/Right/Left` | |
| projects | `LayoutGrid` | Cuadrícula 2x2, igual que hoy |
| import | `Import` | **Corrección:** flecha que entra en una bandeja |
| drive | `Database` | El dibujo actual es el cilindro de `database` de Feather |
| image, photo | `Image` | Hoy son el mismo dibujo; `image` pasa de trazo 1.5 a 2 |
| menu | `EllipsisVertical` | Hoy son puntos rellenos; Lucide los dibuja con trazo, y a simple vista se ven igual |
| star-filled | `Star` con `fill="currentColor"` | |
| trash | `Trash2` | Feather `trash-2`, el dibujo de hoy |
| edit | `SquarePen` | Feather `edit` (lápiz sobre cuadro), el de hoy |
| upload-cloud | `CloudUpload` | |
| tags | `Tag` | Hoy es una etiqueta rellena; pasa a trazo |
| sd-card | SVG propio (se conserva) | Lucide no tiene tarjeta SD; `CardSim` es una SIM y confunde |

**El mapa admite SVG propio.** Las entradas del mapa son componentes de React, así que `sd-card` sigue siendo una función local que dibuja el SVG, con atributos de trazo iguales a los de Lucide (`strokeLinecap`/`strokeLinejoin` redondeados), para que no desentone.

**Versión fijada.** Se instala `lucide-react@^1.51.0` (publicada el 2026-10-03). Lucide saca versiones casi a diario; un rango con caret recibe iconos nuevos sin cambios incompatibles dentro de la 1.x.

**La lista de iconos sale del mapa.** `@lumik/ui` exporta `iconNames` (las claves del mapa, tipadas como `IconName[]`). La story existente `Components/Icon/AllIcons` usa esa lista en lugar de la que tiene escrita a mano, a la que hoy le faltan 12 iconos (`chevron-left`, `sd-card`, `upload-cloud`, `zoom-in`, `zoom-out`, `rotate-ccw`, `rotate-cw`, `info`, `tags`, `eject`, entre otros). Así la story sirve para revisar la migración completa y no se vuelve a quedar desactualizada.

## Risks / Trade-offs

- [Los extremos redondeados de Lucide cambian un poco el aspecto de iconos que hoy los tienen rectos (p. ej. `settings`, `folder`, `trash`)] → Es el estilo canónico del set y se revisa en la story de todos los iconos antes de dar el cambio por bueno.
- [`tags` pasa de relleno a trazo y se ve más ligero en la barra lateral de la foto] → Se acepta por coherencia. Si se ve débil, se le puede pasar `fill` en el mapa sin tocar a los consumidores.
- [Un icono podría no existir en la versión instalada] → El mapa importa componentes por nombre; si falta alguno, `pnpm --filter @lumik/ui build` falla al compilar y lo detecta en el momento.
- [Más peso en el bundle] → Solo se empaquetan los ~30 iconos importados, unos cientos de bytes cada uno, más o menos lo mismo que los SVGs escritos a mano de hoy.
