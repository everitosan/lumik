# Design

## Context

- `get_projects_dashboard` entrega la lista ordenada por `compare_dashboard` (`domain/project.rs:56`): fecha de sesión descendente, desempate por creación descendente, sin fecha al final.
- `Projects.tsx:191` (`projectsByYear`) ignora en la práctica ese orden: agrupa por el año de `session_date ?? created_at`, ordena los años descendente y, dentro de cada año, los proyectos ascendente por `session_date ?? created_at`. Como `Array.sort` es estable, los empates heredan el orden del backend (creación descendente), al revés de la dirección visible.
- La BD global tiene una tabla `settings` clave/valor con `get_setting`/`set_setting` (`db/queries.rs:123`). `AppSettings` agrupa tres de esas claves, y `SettingsPage` lee y guarda el struct completo con `update_app_settings`.
- La cabecera del dashboard tiene título y estadísticas, el `SearchBar` y el botón "Nuevo proyecto". El desplegable de referencia es `FilterPopovers.tsx`: botón con borde, panel `position: absolute` y cierre al hacer clic fuera.
- El desktop no tiene tests de frontend; la lógica testeable vive en Rust.

## Goals / Non-Goals

**Goals:**
- Que el orden elegido se vea al instante y sobreviva a reinicios, sin cambiar nada para quien no lo toque.
- Que guardar Ajustes y cambiar el orden del dashboard no se pisen entre sí.

**Non-Goals:**
- Cambiar la agrupación (por año de `session_date ?? created_at`) o activar la sección "Sin fecha".
- Ordenar por otros criterios (nombre, número de fotos).
- Tocar el portal web.

## Decisions

**Comandos propios, no campos en `AppSettings`.** Se añaden `get_dashboard_sort` y `update_dashboard_sort` con un struct `DashboardSort { years, projects }` guardado en dos claves de `settings` (`dashboard_year_order`, `dashboard_project_order`). Alternativa descartada: añadir los campos a `AppSettings`. `SettingsPage` guarda el struct entero, así que al guardar Ajustes se restablecería el orden a sus valores por defecto (o haría falta que Ajustes conociera unos campos que no muestra). Con comandos separados, cada pantalla solo escribe lo suyo.

**Tipos de dominio con valores por defecto testeados.** En `domain/` (junto a `compare_dashboard`) se define `SortDirection { Asc, Desc }`, serializado como `"asc"`/`"desc"`, y `DashboardSort` con `Default` = `{ years: Desc, projects: Asc }`. Leer una clave ausente o con un valor desconocido devuelve el valor por defecto de esa clave. Se cubre con tests unitarios (por defecto, valores válidos, valor inválido).

**El orden lo aplica el frontend.** `projectsByYear` recibe el `DashboardSort` y aplica la dirección a: la comparación de años, la clave del proyecto (`session_date ?? created_at`) y el desempate (`created_at`). Así ya no depende del orden del backend. `compare_dashboard` no cambia, porque también lo usan otros consumidores del comando. La rama `'sin-fecha'` se conserva tal cual.

**Hook con actualización optimista.** `useDashboardSort()` devuelve el orden y una función para cambiarlo: actualiza el estado local al momento y llama a `update_dashboard_sort`. Si falla, revierte al valor anterior y lo registra en consola. Mientras carga usa los valores por defecto, que coinciden con el orden actual, así que quien no lo ha cambiado no ve ningún salto.

**Desplegable dentro de la página.** El componente `SortMenu` vive en `Projects.tsx`, porque solo lo usa esa página (convención del repo). Imita a `FilterPopovers`: botón con icono `sort` y la etiqueta "Ordenar", un panel con dos grupos de opciones de radio (Años / Proyectos dentro del año, cada uno con "Más recientes primero" y "Más antiguos primero"), y cierre con clic fuera o Escape. Va en `headerRightStyles`, antes de "Nuevo proyecto".

**Icono.** `IconName` añade `sort`, mapeado a `ArrowUpDown` de Lucide (existe en la versión instalada, 1.51.0).

## Risks / Trade-offs

- [Escrituras rápidas seguidas (cambiar varias opciones seguidas) pueden llegar desordenadas] → Cada llamada envía el struct completo y SQLite las serializa. El último cambio que el usuario ve es el último que se manda, así que el estado guardado coincide con el visible.
- [El orden del backend (`compare_dashboard`) y el visible siguen siendo distintos] → Se documenta en el design y la spec describe el visible. Unificarlos implicaría mover la agrupación al backend, lo que queda fuera del alcance.
- [En pantallas estrechas o Android la cabecera se aprieta un poco más] → El botón ocupa lo mismo que los filtros del pie. Si hiciera falta, se puede mostrar solo el icono en anchos pequeños, sin cambiar el comportamiento.
