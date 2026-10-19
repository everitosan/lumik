# Tasks

## 1. Dominio y persistencia

- [x] 1.1 Añadir `SortDirection` y `DashboardSort` (con `Default` = años `Desc`, proyectos `Asc`, serializados como `"asc"`/`"desc"`) en `apps/desktop/src-tauri/src/domain/`, con una función que construya el orden a partir de los valores guardados y tests para: claves ausentes, valores válidos y valor desconocido; verificar con `cargo test`
- [x] 1.2 Añadir `get_dashboard_sort` y `update_dashboard_sort` en `db/queries.rs` sobre las claves `dashboard_year_order` y `dashboard_project_order` de la tabla `settings`, sin tocar `get_app_settings`/`update_app_settings`; verificar con `cargo check`
- [x] 1.3 Exponer los comandos Tauri `get_dashboard_sort` y `update_dashboard_sort` en `commands/settings.rs`, registrarlos en `lib.rs` y añadirlos a la lista de comandos de `apps/desktop/CLAUDE.md`; verificar con `cargo check` y `cargo test`

## 2. Frontend: datos

- [x] 2.1 Añadir los tipos `SortDirection` y `DashboardSort` en `src/lib/types.ts`, las funciones `getDashboardSort`/`updateDashboardSort` en `src/lib/api.ts` y el hook `useDashboardSort()` en `src/lib/hooks.ts` (valores por defecto mientras carga, actualización optimista y reversión si falla); verificar con `pnpm exec tsc --noEmit` en `apps/desktop`

## 3. Frontend: orden y control

- [x] 3.1 Añadir el icono `sort` (Lucide `ArrowUpDown`) a `IconName` en `packages/ui/src/components/Icon/Icon.tsx`; verificar con `pnpm --filter @lumik/ui build` y que aparece en la story `AllIcons`
- [x] 3.2 Hacer que `projectsByYear` en `Projects.tsx` aplique `DashboardSort` a los años, a la fecha de cada proyecto y al desempate por `created_at`; verificar con `tsc` y en la app que, con los valores por defecto, el dashboard se ve igual que antes salvo los empates del mismo día, que ahora siguen la dirección elegida
- [x] 3.3 Añadir `SortMenu` en `Projects.tsx` (botón con icono `sort` y "Ordenar", panel con dos grupos de radios, cierre con clic fuera o Escape) en la cabecera antes de "Nuevo proyecto", con las claves `dashboard.sort.*` en `es.json` y `en.json`; verificar con `jq empty`, `tsc` y en la app que cada opción reordena al instante

## 4. Verificación

- [x] 4.1 Verificar en la app los escenarios de la spec delta: años más antiguos primero, proyectos más recientes primero, el orden se conserva tras reiniciar y se aplica igual a los proyectos de otro disco conectado después
- [x] 4.2 Verificar que guardar Ajustes > Importación no cambia el orden del dashboard y que cambiar el orden no cambia los ajustes de importación
- [x] 4.3 Añadir en `[Sin publicar]` de `CHANGELOG.md` la entrada del orden configurable; verificar que sigue el formato de las entradas existentes
