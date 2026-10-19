# Design

## Context

- `project.archived` (INTEGER 0/1) existe en `project.db` desde el schema inicial. `ProjectDatabase::archive_project` (`db/queries.rs`) la pone a 1 y el comando `archive_project` la expone. `api.archiveProject` existe en el frontend, pero nadie la llama.
- `get_project_dashboard_entry` filtra `WHERE p.deleted = 0 AND p.archived = 0` y devuelve `None` para los archivados. `get_projects_dashboard` los descarta. `ProjectDashboard` no tiene campo `archived`.
- Consumidores de `get_projects_dashboard`: el dashboard (`Projects.tsx`) y `ImportPage`, que busca ahí el proyecto destino. Hoy, un proyecto archivado no se encontraría y la importación saldría sin hacer nada.
- `Projects.tsx` ya filtra por texto (`filteredProjects`), agrupa por año con el orden configurable (`useDashboardSort`) y tiene el `SortMenu` en la cabecera. La preferencia del orden vive en `settings` de la BD global, con comandos propios separados de `AppSettings`.
- `ProjectCard` (`@lumik/ui`) construye su menú de 3 puntos a partir de los handlers que recibe (`onOpenFolder`, `onEdit`, `onDelete`), con textos de `components.projectCard.menu.*`.

## Goals / Non-Goals

**Goals:**
- Archivar y desarchivar de forma reversible y sin efectos secundarios fuera del dashboard.
- Que los archivados ocultos nunca parezcan perdidos: el botón con contador los delata.

**Non-Goals:**
- Sincronizar `archived` con el servidor o el portal web.
- Una vista o sección específica de archivados.
- Archivar en lote.

## Decisions

**El dashboard devuelve todos y el frontend filtra.** `get_project_dashboard_entry` quita `AND p.archived = 0` y selecciona `p.archived`. `ProjectDashboard` gana `archived: bool`. El filtrado (archivados más buscador) ocurre en `Projects.tsx`, igual que el buscador. Alternativa descartada: un parámetro `include_archived` en el comando, que obligaría a pedir de nuevo la lista al cambiar el interruptor y no permitiría contar los archivados ocultos sin una segunda consulta. Como efecto colateral, `ImportPage` encuentra los proyectos archivados.

**`set_project_archived(id, archived)` sustituye a `archive_project`.** Un solo comando para las dos direcciones, sobre `ProjectDatabase::set_archived(bool)`. `archiveProject` se reemplaza por `setProjectArchived` en `api.ts`.

**Preferencia con comandos propios.** `get_dashboard_show_archived` y `set_dashboard_show_archived(show)` guardan la clave `dashboard_show_archived` (`"true"`/`"false"`, por defecto `false`) en `settings`, siguiendo el patrón del orden del dashboard: independiente de `AppSettings` para que guardar Ajustes no la pise. Hook `useDashboardShowArchived()` con el mismo comportamiento que `useDashboardSort`: valor por defecto mientras carga, cambio optimista, vuelta atrás si falla y el cambio del usuario gana a una carga tardía.

**Filtrado y conteos en `Projects.tsx`.**
- `archivedCount` = archivados de la lista completa (sin buscador).
- `visibleProjects` = lista que pasa el buscador y, si los archivados están ocultos, sin ellos. De ahí salen `projectsByYear` y las estadísticas (proyectos y fotos).
- El botón aparece si `archivedCount > 0`.

**Botón "Archivados (N)".** Va en `headerRightStyles`, antes de `SortMenu`, con el mismo estilo de borde que "Ordenar" e icono `archive`. Activo: color y borde `--lumik-tertiary`, como los filtros activos del detalle de proyecto. Lleva `aria-pressed`. Es un toggle, no un desplegable.

**Tarjeta archivada.** `ProjectCard` añade `archived?: boolean` y `onToggleArchive?: () => void`.
- Con `archived`, la tarjeta baja la opacidad (0.55, y 0.85 al pasar el ratón para que siga siendo legible) y muestra la etiqueta "Archivado" sobre la miniatura (esquina superior izquierda, fondo translúcido).
- El menú incluye "Archivar" (icono `archive`) o "Desarchivar" (icono `archive-restore`) si hay `onToggleArchive`, entre "Editar" y "Eliminar".
- Claves nuevas en `components.projectCard`: `archived`, `menu.archive`, `menu.unarchive`. `IconName` gana `archive` y `archive-restore` (Lucide `Archive` y `ArchiveRestore`, presentes en la 1.51.0).

**Acción del menú.** `Projects.tsx` llama a `setProjectArchived(id, !archived)` y después a `refetch()`. Sin actualización optimista: la llamada es local y rápida, y el `refetch` mantiene una única fuente de verdad. Los errores se registran en consola.

## Risks / Trade-offs

- [Un proyecto con `archived = 1` de alguna versión anterior aparecerá ahora contado en el botón] → Es el comportamiento deseado: hasta ahora era inaccesible.
- [Con los archivados visibles y al desarchivar el último, el botón desaparece pero la preferencia sigue en "mostrar"] → Inocuo: no hay nada que ocultar. Si se archiva otro, se mostrará atenuado hasta que el usuario apague el botón.
- [El portal web muestra archivados como activos] → Fuera de alcance y documentado en la proposal.
- [Opacidad baja y legibilidad] → La etiqueta y el aumento de opacidad al pasar el ratón compensan. Se revisa en la story de `ProjectCard`.
