# Tasks

## 1. Migración de rutas

- [x] 1.1 Agregar en `db/migrations.rs` la función pura que convierte una ruta vieja en relativa al proyecto (D2): normaliza `\` a `/`, recorta desde el primer segmento `_media`, `_culled`, `_trash` o `_exported`, no toca las rutas que ya empiezan con uno de ellos y devuelve `None` si ninguno aparece. Verificar con tests unitarios: `lumik/2024/06/15_boda/_media/IMG.cr2` → `_media/IMG.cr2`, `lumik/a/_trash/_exported/x.jpg` → `_trash/_exported/x.jpg`, `_culled/x.dng` sin cambios, una ruta con `\` y una sin subcarpeta reconocida → `None`.
- [x] 1.2 Agregar `migrate_photo_paths_to_project_relative` a `run_project_migrations`, que convierte `dng_path` y `jpg_path` de todas las fotos en una sola transacción, solo escribe si alguna fila cambia y registra un `warn!` por cada ruta no reconocida. Verificar con tests sobre una BD en memoria: fotos con rutas viejas, ya convertidas, movidas (prefijo distinto a la ubicación actual), con `jpg_path` nulo y con una ruta no reconocida; una segunda ejecución no cambia nada.
- [x] 1.3 Hacer la copia `.project.db.pre-relative-paths` con `VACUUM INTO` antes de la primera conversión (D3): `run_project_migrations` recibe la carpeta del proyecto (opcional para BDs en memoria); la copia se hace solo si hay filas por convertir y el archivo no existe; si la copia falla, se registra un `warn!` y no se migra. Verificar con un test sobre un `project.db` temporal en disco en modo WAL que la copia existe, contiene las rutas viejas y se abre con `rusqlite`, y que una segunda apertura no la sobrescribe.
- [x] 1.4 Actualizar el comentario de `dng_path` y `jpg_path` en `db/project_schema.sql` a "relativa a la carpeta del proyecto" y la cabecera de `db/migrations.rs`. Verificar que `cargo test` del crate desktop pasa completo.

## 2. Resolución de rutas con `project_dir`

- [x] 2.1 Agregar `ProjectDatabase::photo_file(rel)` y reemplazar el campo `mount_point` por `project_dir` en los casos de uso que resuelven fotos (`cull_photo`, `trash_photo` con `TrashPhoto` y `RestorePhoto`, `rate_photo`, `rename_project_files`, `rewrite_project_description`), con sus comentarios. Actualizar sus tests para que construyan rutas relativas al proyecto (`_media/IMG.dng`) sobre un `project_dir` temporal. Verificar que pasan los tests de los cinco casos de uso.
- [x] 2.2 Cambiar los comandos de `commands/photo.rs` (`get_photo_preview`, `rotate_photo`, `reveal_photo`, `trash_photo`, `restore_photo`, `save_photo_culled`, rating y `regenerate_project_thumbnails`) y `edit_project` en `commands/project.rs` para que pasen o usen `project_dir` / `photo_file`. Verificar con `grep` que en `src/` ya no queda ningún `mount_point.join(` ni `Path::new(&mount).join(` sobre `dng_path` o `jpg_path`, y que `cargo clippy` pasa sin warnings nuevos.
- [x] 2.3 Quitar de `relocate_project_folder` la llamada a `update_photo_paths_prefix` y eliminar esa función y sus tests de `db/queries.rs` (D4). Verificar con un test (o actualizando el existente de reubicación) que, tras mover la carpeta, las rutas guardadas no cambian y `photo_file` apunta al archivo existente en la nueva carpeta.

## 3. Importación y exportados

- [x] 3.1 Hacer que `build_inserts` en `import_photos.rs` reciba `project_dir` y guarde `dest_folder.strip_prefix(project_dir)/{archivo}`, y que la foto falle si el destino no está bajo el proyecto (D5); ajustar `commands/import_cmd.rs`. Actualizar la aserción del test existente a `_media/IMG_0001.cr2`. Verificar que pasan los tests de `import_photos`.
- [x] 3.2 Calcular `jpg_path` relativo a `project_dir` en `sync_exported` (`commands/backup.rs`) y actualizar el comentario y los fixtures de `application/use_cases/sync_exported.rs` a `_exported/…`. Verificar que pasan los tests de `sync_exported` y que `build_catalog_data` publica `_exported/IMG_0001.jpg`, igual que espera `apps/server/tests/catalog.rs`.
- [x] 3.3 Actualizar `docs/useCases/flujo-importar-imagenes.md`, `docs/desktop-clean-architecture.md` (los casos de uso reciben `project_dir`) y `apps/desktop/CLAUDE.md` (formato de `dng_path` y `jpg_path`, y la copia `.project.db.pre-relative-paths`). Verificar con `grep -rn "relativ" docs apps/desktop/CLAUDE.md` que ninguna mención a `dng_path` o `jpg_path` dice "relativa al punto de montaje".

## 4. Verificación integral

- [x] 4.1 Probar la migración sobre copias de los `project.db` reales del KINGSTON en el scratchpad, sin escribir en el disco: abrirlas con un test o binario de prueba y comparar con `sqlite3` el número de fotos y una muestra de `dng_path` y `jpg_path` antes y después; confirmar que se creó `.project.db.pre-relative-paths` y que ninguna ruta quedó sin convertir. *(Ajuste: la migración corrió sobre el KINGSTON real a las 23:18 porque `pnpm tauri dev` relanzó la app durante la implementación, antes de la prueba con copias. Se verificó sobre copias tomadas después con `cp`: `10_prueba linux` 4/4 fotos y `10_Migrantes` 1/1, todas convertidas a `_media/…` o `_culled/…`, cada archivo existe en el disco, y `.project.db.pre-relative-paths` guarda las rutas viejas `lumik/…`. Se decidió conservar la migración.)*
- [x] 4.2 Verificar a mano en macOS con `pnpm tauri dev`, usando la USB de prueba: el proyecto copiado a `lumik/10_sesion/` muestra previews, se puede rotar, aprobar, descartar, mandar a la papelera, restaurar y mostrar en carpeta; editar el nombre de un proyecto mueve su carpeta y sus fotos se siguen abriendo; importar en ese proyecto guarda `_media/…` (comprobar con `sqlite3` sobre una copia, cerrando Lumik antes de copiar).
- [x] 4.3 Verificar en Linux (sesión Xorg) con el mismo disco que los proyectos migrados en macOS se abren con previews y edición, y correr `openspec validate project-relative-photo-paths --strict`.

## Workflow follow-up

- Actualizar Lumik en macOS y Linux antes de abrir los discos reales con la nueva versión.
- Archivar el change con `/opsx:archive` cuando la verificación en macOS y Linux esté hecha.
