# Design

## Context

- `ProjectDetailFooter.tsx` es un `<footer>` con `borderTop`, `padding: 10px 32px`, `flexShrink: 0` y `justifyContent: space-between`.
  - A la izquierda: contadores en mono 12px ("248 fotos • 31 aprobadas") y el interruptor "Mostrar solo aprobadas", unas 40 líneas inline (un `label`, un `span role="checkbox"` circular de 16px y el check en SVG, en color terciario cuando está activo).
  - A la derecha: `RatingFilter`, `ColorsFilter` y `TagsFilter`, separados por divisores de 1×20px inline. Sus paneles abren hacia arriba (`popoverPanelTopStyles`, `bottom: 100%`).
- `ProjectDetail.tsx` monta el pie fuera de la zona con scroll: el contenedor es una columna `height: 100%` sin padding, así que el pie ocupa todo el ancho.
- `Projects.tsx` usa un contenedor con `padding: 24px 32px` que envuelve la cabecera y la cuadrícula. La cabecera tiene tres zonas `flex: 1`: título y estadísticas, `SearchBar`, y botón de archivados + `SortMenu` + "Nuevo proyecto". El panel de `SortMenu` abre hacia abajo (`top: 100%`).

## Goals / Non-Goals

**Goals:**
- Que el dashboard y el detalle de proyecto compartan la misma barra inferior y el mismo interruptor, y que un filtro nuevo se añada en cualquiera de los dos sin copiar estilos.
- Que el detalle de proyecto se vea igual que antes.

**Non-Goals:**
- Mover `RatingFilter`/`TagsFilter`/`ColorsFilter` a `@lumik/ui` (solo los usa el detalle).
- Extraer `SortMenu` (solo lo usa el dashboard).
- Cambiar filtros, orden o persistencia.

## Decisions

**`FilterBar` en `@lumik/ui`.** API: `<FilterBar start={…} end={…} />`, más `FilterBarDivider` para el separador vertical.
- Renderiza el `<footer>` con los estilos actuales del pie del detalle, y añade `flexWrap: 'wrap'` y `rowGap: '8px'`.
- `start` y `end` son contenedores flex. `start` con `gap: 16px`, como el detalle. `end` con `gap: 8px` y `marginLeft: auto`, para que al pasar a dos líneas quede alineado a la derecha.
- Sin texto propio: los hijos los aporta cada página. Así no hace falta i18n en el componente.

**`FilterToggle` en `@lumik/ui`.** API: `<FilterToggle label checked onChange />`, donde `label` es `ReactNode` para poder incluir el contador.
- Reproduce el aspecto del interruptor de aprobadas: círculo de 16px, borde de 2px, relleno y texto en terciario cuando está activo, y el check SVG en `--lumik-on-tertiary`.
- Se renderiza como `<button type="button" role="switch" aria-checked>`, en lugar del `label` con `onClick`. Así es accesible por teclado sin cambiar el aspecto.

**Contadores.** En el detalle de proyecto siguen en la zona `start` de la barra. En el dashboard se quedan en la cabecera, bajo el título, con su estilo actual: tras probarlo en la app, el usuario prefirió no moverlos a la barra. No se extrae un componente de contadores (son distintos en cada página).

**Estructura de `Projects.tsx`.** El contenedor exterior pasa a ser una columna `height: 100%` sin padding (como en `ProjectDetail`). Dentro hay una zona con `padding: 24px 32px`, `flex: 1` y `overflow: hidden` para la cabecera y la cuadrícula con scroll, seguida del `FilterBar` a todo el ancho. La cabecera pierde los controles de filtro y conserva título con contadores, buscador y "Nuevo proyecto".

**Barra del dashboard.** Solo usa la zona `end`, alineada a la derecha: el botón "Archivados (N)" tal como era (borde, icono `archive`, resaltado en terciario cuando está activo, `aria-pressed`; solo si `archivedCount > 0`), un `FilterBarDivider` y `SortMenu`. Se descartó el `FilterToggle` para este filtro: tras probarlo en la app, el usuario prefirió conservar el botón.

**`SortMenu` abre hacia arriba.** Su panel cambia `top: '100%'`/`marginTop` por `bottom: '100%'`/`marginBottom`, como `popoverPanelTopStyles`. Se mantiene `right: 0`.

**i18n.** Sin cambios: el botón conserva `dashboard.archived.label`/`show`/`hide`.

## Risks / Trade-offs

- [El detalle de proyecto podría cambiar de aspecto al migrar a los componentes] → Se toma una captura de referencia antes de tocarlo y se compara después. El interruptor conserva medidas, colores y espaciados.
- [Con una ventana muy estrecha, las dos líneas de la barra quitan algo de alto a la cuadrícula] → Solo ocurre cuando no cabe en una línea, que es justo el caso que hoy se rompe.
