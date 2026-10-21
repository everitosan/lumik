# Design

## Context

Ver `proposal.md` (Why) para la motivación. Estado actual relevante:

- `ProjectDatabase` (`db/mod.rs`) ya tiene `project_dir`, que es la carpeta padre del `project.db` en el momento de abrirlo, y `mount_point`, que es el montaje con el que se descubrió. Las miniaturas, los previews y `cover_photo_path` (`.thumbs/{id}.jpg`) ya se resuelven con `project_dir`. Solo `photo.dng_path` y `photo.jpg_path` usan `mount_point`.
- Los casos de uso que tocan archivos de fotos reciben `mount_point: &Path` y hacen `mount_point.join(&photo.dng_path)`: `cull_photo`, `trash_photo` (`TrashPhoto` y `RestorePhoto`), `rate_photo`, `rename_project_files` y `rewrite_project_description`. `rotate_photo` ya recibe `project_dir`. Los comandos de `commands/photo.rs` (preview, rotar, mostrar en carpeta y regenerar miniaturas) hacen la misma unión directamente.
- Las reglas de destino de `domain/paths.rs` (`cull_destination`, `trash_destination`, `restore_destination`, etc.) operan sobre rutas relativas genéricas: separan "carpeta del proyecto" y "subcarpeta + archivo" contando niveles desde el final. Con un prefijo de proyecto vacío (`_media/x.dng`) devuelven `_culled/x.dng`, `_trash/x.dng`, etc. sin cambios.
- La importación calcula `dest_folder.strip_prefix(mount_point)` (`import_photos.rs`). `sync_exported` (`commands/backup.rs`) calcula `jpg_path` con `strip_prefix(&project_db.mount_point)`.
- `relocate_project_folder` (`commands/project.rs`) mueve la carpeta y luego llama a `update_photo_paths_prefix(old_rel, new_rel)` para reescribir las rutas.
- Las migraciones de `db/migrations.rs` son idempotentes y no usan `PRAGMA user_version`, a propósito: las BDs viven en discos externos con estados heterogéneos. Se ejecutan en cada `ProjectDatabase::open` y `create`.
- `sync_exported` vuelve a enlazar todos los exportados cada vez que se abre un proyecto y publica el catálogo si enlazó alguno. Así, un `jpg_path` viejo en el server se corrige solo la próxima vez que se abre un proyecto con exportados.

## Goals / Non-Goals

**Goals:**
- Un único punto que convierte una ruta guardada en una ruta absoluta, para que ningún caso de uso vuelva a unir rutas de fotos con el montaje.
- Una migración que se pueda aplicar sobre cualquier `project.db` existente, sin conocer dónde se importó el proyecto.
- Proteger los datos reales: dejar una copia de la BD antes de reescribir rutas.

**Non-Goals:**
- Cambiar los puertos (`PhotoRepository`, `FileStore`, etc.) o las reglas de `domain/paths.rs`, que ya sirven con rutas relativas al proyecto.
- Versionar el esquema con `PRAGMA user_version`.
- Hacer que una versión anterior de Lumik lea los proyectos migrados.

## Decisions

### D1. La base de las rutas es `project_dir`, inyectado en lugar de `mount_point`

Los casos de uso que hoy tienen un campo `mount_point: &Path` para resolver fotos pasan a tener `project_dir: &Path`, con el comentario "los `dng_path` son relativos a él". Los comandos les pasan `project_db.project_dir`. `mount_point` se queda en `ProjectDatabase` para lo que sí es del disco: reubicar el proyecto dentro de `lumik/` y no subir más allá del montaje al limpiar carpetas vacías.

`ProjectDatabase` gana un helper `photo_file(&self, rel: &str) -> PathBuf` (`project_dir.join(rel)`) para los comandos que resuelven la ruta directamente, como `get_photo_preview`, `rotate_photo`, `reveal_photo` y `regenerate_project_thumbnails`.

- *Alternativa: guardar la ruta absoluta.* Se descarta porque cambia con cada montaje y cada plataforma.
- *Alternativa: mantener las rutas relativas al montaje y recalcular el prefijo al abrir.* Es la "autocorrección" que se consideró en la exploración. Arregla los proyectos movidos, pero sigue obligando a reescribir rutas en cada reubicación y deja dos formas de construir rutas. Se descarta.

### D2. Migración por la primera subcarpeta del proyecto

`migrate_photo_paths_to_project_relative(conn)` se agrega a `run_project_migrations`. Para cada fila de `photo`, y para `dng_path` y `jpg_path` por separado:

1. Normaliza `\` a `/` y divide la ruta en segmentos.
2. Busca el primer segmento que sea exactamente `_media`, `_culled`, `_trash` o `_exported`.
3. Si lo encuentra en una posición mayor a 0, la nueva ruta son los segmentos desde ahí, unidos con `/`. Si está en la posición 0, la ruta ya está en el formato nuevo y no se toca. Si no lo encuentra, no se toca y se registra un `warn!` con el id de la foto.

Todas las actualizaciones se hacen en una sola transacción. La función decide en Rust, no con `LIKE`, porque `_` es un comodín en SQL. Si ninguna fila cambia, no se escribe nada.

Por qué funciona: las carpetas de proyecto nunca se llaman exactamente así. El descubrimiento ignora las carpetas que empiezan con `_`, y los nombres de proyecto tienen la forma `{día}_{slug}`. El prefijo que se descarta incluye la ubicación vieja del proyecto, así que la migración no necesita saber dónde está ahora la carpeta. Por eso también repara los proyectos que se movieron con una versión anterior (escenario "Proyecto movido antes de la conversión").

- *Alternativa: quitar el prefijo `project_dir.strip_prefix(mount_point)`.* Solo sirve si el proyecto no se movió desde que se importó. Se descarta.
- *Alternativa: prefijo común de todas las rutas.* Falla con una sola foto o con fotos repartidas en `_media` y `_trash/_exported`. Se descarta.

### D3. Copia de seguridad de la BD antes de la primera conversión

Si la migración va a cambiar al menos una fila, antes de abrir la transacción ejecuta `VACUUM INTO '{project_dir}/.project.db.pre-relative-paths'`, solo si ese archivo no existe todavía. `VACUUM INTO` produce una copia consistente aunque la BD esté en modo WAL, sin cerrar la conexión. El nombre empieza con `.`, así que el descubrimiento (que busca `project.db`) y la carpeta del proyecto en Finder lo ignoran. Si la copia falla, se registra un `warn!` y la migración no se aplica en esa apertura: el proyecto se abre con las rutas viejas y se reintenta en la siguiente.

Para que `migrations.rs` conozca la ruta de la copia, `run_project_migrations` recibe el `project_dir` (o la ruta del `project.db`). Las pruebas en memoria pasan `None` y se saltan la copia.

- *Alternativa: no hacer copia.* Es más simple, pero el `project.db` es la única copia de las calificaciones y etiquetas, y vive en discos con datos reales. Se descarta.

### D4. Reubicar el proyecto ya no toca la BD

`relocate_project_folder` deja de llamar a `update_photo_paths_prefix`, y la función se elimina de `db/queries.rs` junto con sus pruebas. La reubicación queda en mover la carpeta, limpiar los ancestros vacíos y reabrir.

### D5. Importación y exportados relativos al proyecto

- `build_inserts` (`import_photos.rs`) recibe `project_dir` y calcula `dest_folder.strip_prefix(project_dir)`. Si el destino no está bajo el proyecto, que no debería pasar, el import falla para esa foto en lugar de guardar una ruta absoluta.
- `sync_exported` calcula `jpg_path` con `strip_prefix(&project_db.project_dir)`. El comentario de `ExportedFile.jpg_path` pasa a decir "relativo a la carpeta del proyecto".

## Risks / Trade-offs

- **[Una versión anterior de Lumik abre un proyecto migrado]** → No encuentra los originales: no hay previews y las acciones sobre archivos fallan, aunque la BD no se corrompe. Se acepta (un único usuario). Para volver atrás, se reemplaza `project.db` por la copia `.project.db.pre-relative-paths`.
- **[El descubrimiento escribe en la BD al conectar el disco]** → La migración se aplica una sola vez por proyecto y en una transacción. Expulsar justo durante la migración deja la transacción sin confirmar y SQLite la descarta. La copia de D3 cubre el resto.
- **[Una ruta sin subcarpeta reconocida]** → Se conserva tal cual y se registra en el log. Esa foto falla como hoy, sin afectar a las demás.
- **[El catálogo del server conserva un `jpg_path` viejo]** → El server no lo interpreta y se corrige en la siguiente publicación (ver Context). No requiere acción.
- **[El orden del grid por `dng_path`]** → Dentro de un proyecto todas las rutas tenían el mismo prefijo, así que quitarlo no cambia el orden relativo.

## Migration Plan

1. Implementar y probar la migración con BDs en memoria que tengan rutas viejas, nuevas, movidas y no reconocidas.
2. Probar sobre copias de `project.db` reales del KINGSTON en el scratchpad, comparando el conteo de fotos y una muestra de rutas antes y después.
3. Actualizar Lumik a la vez en macOS y Linux antes de abrir los discos reales con la nueva versión.
4. Rollback: reemplazar `project.db` por `.project.db.pre-relative-paths` y volver a la versión anterior de Lumik.
