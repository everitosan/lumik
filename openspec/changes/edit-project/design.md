# Design

## Context

- `rename_project` (`commands/project.rs:232`) valida el nombre, calcula la carpeta nueva conservando el prefijo de día y llama a `relocate_project_folder`. Esa función cierra la BD, hace `fs::rename` de la carpeta, reabre la BD, reescribe el prefijo de `dng_path` y vuelve a registrar el proyecto. La importación usa la misma función para la fecha inferida. Después, el dashboard llama a `relocate_backup` en segundo plano.
- Al crear un proyecto sin fecha, `session_date` queda `NULL` y la carpeta usa la fecha de hoy en UTC (`commands/project.rs:64-71`), que coincide con la parte de fecha de `created_at`.
- Las fotos importadas con renombrado se llaman `{AAAA-MM-DD}__{HH-MM-SS}__{saneado}[-NNN].{ext}` (`import/pipeline.rs:114-119` en Android, `:167` con exiftool), con `sanitize_name` (privada en `import/pipeline.rs:358`).
- Precedente para tocar archivos ya importados: `CullPhoto` (`application/use_cases/cull_photo.rs`) mueve la foto, su `.xmp` (`with_extension("xmp")`) y el sidecar AppleDouble (`FinderTagWriter::move_sidecar`) a través de los puertos `FileStore` y `FinderTagWriter`, y tiene tests con dobles. `MetadataTool::set_orientation` escribe dentro del RAW en desktop y en el `.xmp` en Android.
- `sync_exported` enlaza por *stem*, pero una vez enlazada la foto guarda `jpg_path`, así que renombrar la foto no rompe los enlaces existentes.
- `RenameProjectModal` ya se titula "Editar proyecto". `ProjectDashboard` no trae la descripción, así que el modal la pide con `get_project`.

## Goals / Non-Goals

**Goals:**
- Un único guardado que deje BD, carpeta, archivos (si se pide) y metadatos (si se pide) coherentes, o que informe con claridad de lo que no se pudo hacer.
- Que la lógica frágil (fecha efectiva, qué archivos se renombran, rollback) quede en `domain`/`application` con tests.

**Non-Goals:**
- Renombrar `_exported/`, o que `sync_exported` reconozca nombres anteriores.
- Renombrar archivos importados sin renombrado.
- Transacción atómica entre carpeta, archivos y metadatos (ver Risks).

## Decisions

**Un comando `edit_project` que sustituye a `rename_project`.** Firma: `edit_project(id, name, session_date, description, rename_files, rewrite_metadata) -> EditProjectResult { project, renamed_files, metadata_updated, metadata_failed: Vec<FailedFile> }`. Corre en `spawn_blocking` (como los comandos de respaldo), porque reescribir metadatos puede tardar. Emite el evento `edit-project-progress { done, total }` durante la reescritura. La alternativa de varios comandos encadenados desde el frontend se descarta, porque dejaría al cliente decidir el orden y la recuperación.

**Orden de pasos dentro del comando:**
1. Validar el nombre (no vacío tras `trim`) y la fecha (`ProjectFolder::date_parts` válida).
2. Resolver la fecha efectiva (regla de abajo) y calcular la carpeta destino con `ProjectFolder::path`. Si difiere de la actual, `relocate_project_folder`. Una colisión falla aquí sin cambios.
3. Actualizar nombre, descripción y `session_date` en `project.db`.
4. Si `rename_files` y cambió el nombre: caso de uso `RenameProjectFiles` (todo o nada).
5. Si `rewrite_metadata`: caso de uso `RewriteProjectDescription` (por foto, informando fallos).

Si el paso 4 falla, los pasos 2 y 3 ya están aplicados: el proyecto queda con el nombre y la carpeta nuevos y los archivos con su nombre anterior, que es un estado válido (equivale a no haber marcado la casilla). Se devuelve el error y el usuario puede reintentar marcando la casilla.

**Fecha efectiva como regla pura.** En `domain/project.rs`: `effective_session_date(stored: Option<&str>, created_at: &str, submitted: &str) -> Option<String>`. Si `stored` es `None` y `submitted` coincide con la fecha de `created_at` (sus 10 primeros caracteres, la misma que usó la carpeta), devuelve `None`. En cualquier otro caso devuelve `Some(submitted)`. La carpeta destino usa `submitted` en ambos casos, así que en el caso `None` la carpeta no se mueve por la fecha. El frontend precarga `session_date ?? created_at.slice(0, 10)`. Tests: sin fecha y sin cambio, sin fecha y con cambio, con fecha.

**Nombre de archivo renombrado como regla pura.** `sanitize_name` se mueve a `domain::project::sanitize_file_name` (pública) y `import/pipeline.rs` la usa desde ahí. Una función `renamed_photo_file(file_name, old_sanitized, new_sanitized) -> Option<String>` reconoce `^\d{4}-\d{2}-\d{2}__\d{2}-\d{2}-\d{2}__{old}(-\d{3})?\.{ext}$` (comparando `old` literalmente, no como regex) y devuelve el nombre con `new`. Devuelve `None` si no coincide o si `old == new`. Tests: nombre limpio, con sufijo, otro proyecto con prefijo parecido (`Boda` frente a `Boda_Ana`), `IMG_4821.CR3`, nombre con acentos.

**`RenameProjectFiles` con rollback.** Caso de uso en `application/use_cases/rename_project_files.rs` con los puertos `PhotoRepository`, `FileStore` y `FinderTagWriter`. Para cada foto cuyo basename `renamed_photo_file` acepta: comprueba que el destino no existe (si existe, es un fallo), renombra el archivo, su `.xmp` si existe y el sidecar AppleDouble, y actualiza `dng_path` con un método nuevo del repositorio (`update_dng_path`). Registra cada paso hecho y, ante el primer fallo, los deshace en orden inverso (archivos y `dng_path`) antes de devolver el error. Tests con dobles: todo correcto, colisión, fallo a mitad con rollback, fotos que no coinciden se saltan.

**`RewriteProjectDescription` por foto.** Caso de uso que recorre las fotos del proyecto y llama a un método nuevo `MetadataTool::set_image_description(file, Option<&str>)`. En desktop, exiftool con `-ImageDescription=` y `-XMP-dc:Description=` (vacío elimina el valor) y `-overwrite_original`. En Android actualiza o quita la descripción en el `.xmp` (función nueva en `import/xmp.rs`, junto a `update_xmp_orientation`). El valor se construye igual que al importar: `{descripción}@{año}`, con el año de la fecha efectiva o de `created_at`. Los fallos se acumulan y no detienen el resto. Informa el progreso a través del `ProgressReporter`. Tests con dobles: todas bien, algunas fallan, descripción vacía.

**Cuándo aparecen las casillas.** En el frontend: "Renombrar archivos" si `name.trim() !== nombre original` y el proyecto tiene fotos. No se cuentan de antemano las que coinciden con el patrón, para no pedir la lista entera al abrir el modal. Si ninguna coincide, el resultado dice "0 archivos renombrados". "Actualizar metadatos" si cambió la descripción o el año de la fecha y `photo_count > 0`. Las dos empiezan desmarcadas. Con el nombre cambiado se muestra el aviso sobre editores externos.

**Modal.** `EditProjectModal` sustituye a `RenameProjectModal` (mismo sitio, `pages/Project/Projects/`). Campos: `Input` de nombre, `Input variant="date"` (como en `CreateProjectModal`) y `Input variant="textarea"` para la descripción, más los dos `Checkbox` de `@lumik/ui`. Al guardar con casillas, el botón muestra el progreso (`n / total`) y, al terminar con fallos, el modal se queda abierto con el resumen. Sin fallos, se cierra.

**Después de guardar.** El dashboard hace `refetch`, llama a `relocate_backup` (como hoy) y a `ensure_project_synced` en segundo plano, para que el portal web vea el nombre y la fecha nuevos.

## Risks / Trade-offs

- [Desconectar el disco a mitad de la reescritura de metadatos] → Cada foto es independiente. Las que fallan se listan y volver a guardar con la casilla marcada las reescribe (el resultado es idempotente).
- [El rollback del renombrado también puede fallar (disco desconectado)] → Se registra cada archivo que no pudo volver atrás y se informa en el error. El proyecto sigue abriéndose porque `dng_path` se actualiza junto con cada archivo.
- [Exportaciones futuras desde un editor externo con los nombres viejos no se enlazan] → Limitación aceptada. El modal avisa al cambiar el nombre.
- [Elegir a propósito la fecha de creación en un proyecto sin fecha no la fija] → Se acepta. La carpeta ya está en esa fecha y una importación posterior puede inferir otra. Queda documentado en la spec.
- [Reescribir metadatos modifica los originales] → Solo con la casilla marcada explícitamente, y con el mismo mecanismo que la rotación.
