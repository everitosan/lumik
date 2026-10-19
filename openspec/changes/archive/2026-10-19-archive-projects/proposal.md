# Proposal

## Why

Con muchos proyectos, el dashboard mezcla las sesiones en curso con las ya terminadas y cuesta encontrar en qué se está trabajando. El backend ya tiene una bandera `archived` y un comando `archive_project`, pero ninguna pantalla los usa. Además, hoy archivar equivaldría a hacer desaparecer el proyecto: el dashboard excluye los archivados, no se pueden volver a ver ni desarchivar, y la importación ni siquiera encontraría un proyecto archivado como destino.

## What Changes

- El menú de la tarjeta del proyecto añade **Archivar** o, si ya está archivado, **Desarchivar**. Se aplica al instante y sin confirmación.
- La cabecera del dashboard añade un botón **"Archivados (N)"**, visible solo si hay proyectos archivados, que muestra u oculta los archivados. Por defecto están ocultos.
- Al mostrarse, los archivados aparecen en su año y en su orden como los demás, atenuados y con la etiqueta "Archivado".
- La preferencia de mostrar u ocultar se guarda en la base global de Lumik y se conserva al reabrir la app.
- Las estadísticas de la cabecera y el buscador solo consideran los proyectos visibles.
- Un proyecto archivado se abre y funciona igual (importar, editar, respaldar). Archivar solo cambia su visibilidad en el dashboard.
- **BREAKING (interno):** `archive_project` se sustituye por `set_project_archived(id, archived)`, y `get_projects_dashboard` devuelve también los archivados con el campo `archived`. Los únicos consumidores son el dashboard y el wizard de importación del desktop, y el segundo se beneficia porque pasa a encontrar proyectos archivados como destino.

## Capabilities

### New Capabilities
<!-- Ninguna -->

### Modified Capabilities
- `project-management`: nuevo requisito "Archivar proyectos" (archivar, desarchivar, mostrar u ocultar con preferencia persistente). "Orden del dashboard" deja de excluir siempre los archivados: dependen del filtro y, cuando se muestran, se ordenan como los demás.

## Impact

- Backend desktop: `db/queries.rs` (la entrada del dashboard incluye archivados y el campo `archived`; `set_archived`; preferencia `dashboard_show_archived` en `settings`), `db/models.rs` (`ProjectDashboard.archived`), `commands/project.rs` y `commands/settings.rs` (comandos nuevos), `lib.rs`.
- `@lumik/ui`: `ProjectCard` acepta `archived` (atenuado y etiqueta) y `onToggleArchive` (opción de menú); iconos `archive` y `archive-restore` (Lucide); claves i18n de la tarjeta.
- Frontend desktop: `Projects.tsx` (botón, filtro, estadísticas, buscador), `lib/api.ts`, `lib/hooks.ts`, `lib/types.ts`, i18n.
- Documentación: `apps/desktop/CLAUDE.md` (comandos y estado de "Archivar proyectos").
- Fuera de alcance: el portal web, que sigue mostrando todos los proyectos porque el catálogo del servidor no incluye `archived`.
