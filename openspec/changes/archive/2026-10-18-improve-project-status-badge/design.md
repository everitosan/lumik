# Design

## Context

El badge se renderiza en un solo lugar, `ProjectCard` de `@lumik/ui`. El desktop (`Projects.tsx`) y el portal web (`Dashboard.tsx`) lo consumen sin estilos propios. Hoy cada estado tiene un bloque de estilos inline duplicado en `statusBadgeStyles`, con los colores tomados de los tokens `--lumik-primary` (azul), `--lumik-secondary` (amarillo) y `--lumik-tertiary` (naranja) de `tokens.css`. `Icon` ya expone `import`, `edit` y `check`, y pinta con `currentColor`.

## Goals / Non-Goals

**Goals:**
- Que todo el cambio quede dentro de `ProjectCard`, para que ambos consumidores lo hereden sin tocarse.
- Usar solo tokens existentes; no añadir colores nuevos a `tokens.css`.

**Non-Goals:**
- Hacer del badge un componente reutilizable aparte (`StatusBadge`). Hoy solo lo usa la tarjeta.
- Cambiar el contrato de props de `ProjectCard` o el tipo `WorkflowStatus`.

## Decisions

**Una sola tabla de configuración por estado.** Se reemplazan los tres bloques de estilos por un estilo base común y un mapa `estado -> { color, fondo, icono }`. Así se elimina la duplicación y el mapa sirve como única fuente de la relación estado-color-icono. Alternativa descartada: mantener tres objetos completos, que obliga a repetir padding, radio y fuente en cada uno.

**Colores a partir de tokens:**

| Estado | Texto/icono | Fondo |
|---|---|---|
| imported | `--lumik-secondary` (#e9c349) | `rgba(233, 195, 73, 0.15)` |
| edited | `--lumik-primary` (#b0c6ff) | `rgba(176, 198, 255, 0.15)` |
| delivered | `--lumik-outline` (#8c90a0) | `rgba(140, 144, 160, 0.15)` |

Para `delivered` se elige gris `--lumik-outline` en lugar de blanco (`--lumik-on-surface`). El blanco destaca sobre el fondo oscuro más que el amarillo y contradice la idea de que lo terminado pierde peso. El fondo translúcido al 15 % sigue el patrón actual de los otros badges.

**Icono antes del texto.** El badge pasa a `inline-flex` con `gap: 4px` y un `Icon` de tamaño `sm` (12px) que hereda el color del texto con `currentColor`. El icono es decorativo (`aria-hidden`), porque la etiqueta ya comunica el estado. Se reutilizan `import`, `edit` y `check` en lugar de dibujar iconos nuevos.

**Etiquetas solo desde i18n.** Se borra el `statusLabels` con textos fijos a nivel de módulo, que ya no se usa porque el componente lo sobrescribe con `t()`. Solo cambia el valor de `components.projectCard.status.edited`; las claves se mantienen.

## Risks / Trade-offs

- [El icono `edit` también aparece en el menú de 3 puntos como "Editar proyecto"] → El contexto (badge de color frente a ítem de menú) basta para distinguirlos. Si en pruebas resulta confuso, se cambia el icono del badge sin tocar la spec, que solo pide "un icono de lápiz".
- [El gris podría leerse como "deshabilitado"] → `--lumik-outline` sobre la superficie oscura mantiene un contraste aproximado de 5:1, legible a 11px, y el icono de check refuerza que es un estado completado.
- [Las stories de Storybook usan valores inválidos] → Se corrigen en este change; si no, el badge no se puede revisar visualmente en el UI kit.
