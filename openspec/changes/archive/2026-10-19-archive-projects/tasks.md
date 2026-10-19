# Tasks

## 1. Backend

- [x] 1.1 En `db/queries.rs`, quitar `AND p.archived = 0` de `get_project_dashboard_entry`, seleccionar `p.archived` y añadir `archived: bool` a `ProjectDashboard` (`db/models.rs`); sustituir `archive_project` por `set_archived(bool)`; añadir un test sobre una `ProjectDatabase` temporal que archive, compruebe que la entrada del dashboard sigue existiendo con `archived = true` y desarchive; verificar con `cargo test`
- [x] 1.2 Añadir `get_dashboard_show_archived`/`set_dashboard_show_archived` en `db/queries.rs` sobre la clave `dashboard_show_archived` de `settings` (por defecto `false`); verificar con `cargo check`
- [x] 1.3 Sustituir el comando `archive_project` por `set_project_archived(id, archived)` en `commands/project.rs` y añadir `get_dashboard_show_archived`/`set_dashboard_show_archived` en `commands/settings.rs`, registrarlos en `lib.rs` y actualizar la lista de comandos y el estado de "Archivar proyectos" en `apps/desktop/CLAUDE.md`; verificar con `cargo check` y `cargo test`

## 2. UI kit

- [x] 2.1 Añadir los iconos `archive` y `archive-restore` (Lucide `Archive`, `ArchiveRestore`) a `IconName`; verificar con `pnpm --filter @lumik/ui build`
- [x] 2.2 Añadir a `ProjectCard` las props `archived` (opacidad 0.55, 0.85 al pasar el ratón, etiqueta "Archivado" sobre la miniatura) y `onToggleArchive` (opción "Archivar"/"Desarchivar" en el menú entre "Editar" y "Eliminar"), con las claves `components.projectCard.archived`, `menu.archive` y `menu.unarchive` en `es.json`/`en.json` de `@lumik/ui`, y una story `Archived` en `apps/uikit`; verificar con el build de `@lumik/ui`, el build del uikit y revisando la story

## 3. Frontend desktop

- [x] 3.1 Añadir `archived` a `ProjectDashboard` en `lib/types.ts`, sustituir `archiveProject` por `setProjectArchived` en `lib/api.ts`, añadir `getDashboardShowArchived`/`setDashboardShowArchived` y el hook `useDashboardShowArchived()` en `lib/hooks.ts` (por defecto mientras carga, optimista, vuelta atrás si falla, el cambio del usuario gana a la carga); verificar con `pnpm exec tsc --noEmit` en `apps/desktop`
- [x] 3.2 En `Projects.tsx`: calcular `archivedCount` y `visibleProjects` (buscador más filtro de archivados) y usarlos en `projectsByYear` y en las estadísticas; añadir el botón "Archivados (N)" (solo si `archivedCount > 0`, `aria-pressed`, resaltado al estar activo) antes de "Ordenar"; pasar `archived` y `onToggleArchive` a cada `ProjectCard` (llama a `setProjectArchived` y `refetch`); claves `dashboard.archived.*` en `es.json`/`en.json`; verificar con `jq empty` y `tsc`

## 4. Verificación y documentación

- [x] 4.1 Verificar en la app los escenarios de la spec delta: archivar oculta el proyecto y aumenta el contador; mostrar archivados los pone atenuados en su año y orden; desarchivar; la preferencia se conserva tras reiniciar; sin archivados no hay botón; estadísticas y buscador solo cuentan lo visible; importar en un proyecto archivado funciona y sigue archivado
- [x] 4.2 Añadir la entrada en `[Sin publicar]` de `CHANGELOG.md`; verificar que sigue el formato de las entradas existentes
