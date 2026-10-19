# Proposal

## Why

El dashboard del desktop agrupa los proyectos por año, con los años de más reciente a más antiguo y los proyectos de cada año de más antiguo a más reciente. Ese orden está fijo en el código y no se puede cambiar, aunque cada fotógrafo busca sus sesiones de forma distinta (por ejemplo, ver primero lo último dentro de cada año). Además, la spec "Orden del dashboard" describe solo el orden que entrega el backend (todo descendente), no el que se ve en pantalla, porque el frontend reordena dentro de cada año.

## What Changes

- La cabecera del dashboard añade un botón **Ordenar** con un desplegable de dos grupos: orden de las secciones de año (más recientes o más antiguos primero) y orden de los proyectos dentro de cada año (ídem).
- La elección se aplica al momento y se guarda en la base global de Lumik (tabla `settings`), no en la base de cada proyecto, así que se conserva entre reinicios y es la misma para todos los discos.
- Los valores iniciales replican el orden actual: años de más reciente a más antiguo y proyectos de más antiguo a más reciente.
- Cuando dos proyectos tienen la misma fecha, se desempatan por fecha de creación en la misma dirección que el orden elegido. Hoy salen en sentido contrario dentro de la lista ascendente.
- Los proyectos sin fecha de sesión siguen agrupándose y ordenándose por su fecha de creación, como hoy.
- Se añade el icono `sort` a `IconName` (Lucide `ArrowUpDown`).

## Capabilities

### New Capabilities
<!-- Ninguna -->

### Modified Capabilities
- `project-management`: el requisito "Orden del dashboard" pasa a describir la agrupación por año y el orden visible, y añade que el usuario puede elegir la dirección de los años y de los proyectos, guardada en la configuración de la instalación.

## Impact

- Frontend desktop: `apps/desktop/src/pages/Project/Projects/Projects.tsx` (botón, desplegable y orden configurable), `src/lib/api.ts`, `src/lib/hooks.ts`, `src/lib/types.ts`, `src/i18n/locales/{es,en}.json`.
- Backend desktop: comandos nuevos para leer y guardar el orden (`commands/settings.rs`, registro en `lib.rs`), consultas en `db/queries.rs` sobre la tabla `settings` existente, sin migración.
- `@lumik/ui`: icono `sort` en `Icon`.
- Documentación: `apps/desktop/CLAUDE.md` (lista de comandos).
- Fuera de alcance: el portal web, que mantiene su orden fijo porque no lee la base de Lumik; la sección "Sin fecha" que el código intenta tener pero nunca se activa.
