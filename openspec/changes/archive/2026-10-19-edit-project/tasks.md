# Tasks

## 1. Reglas de dominio

- [x] 1.1 Mover `sanitize_name` de `import/pipeline.rs` a `domain::project::sanitize_file_name` (pública) y usarla desde el pipeline; verificar con `cargo test` que los tests existentes del pipeline siguen pasando
- [x] 1.2 Añadir `effective_session_date(stored, created_at, submitted)` en `domain/project.rs` con tests (sin fecha y sin cambio → `None`; sin fecha y con cambio → `Some`; con fecha → `Some(submitted)`); verificar con `cargo test`
- [x] 1.3 Añadir `renamed_photo_file(file_name, old_sanitized, new_sanitized)` en `domain/project.rs` con tests (nombre limpio, sufijo `-001`, prefijo parecido `Boda` frente a `Boda_Ana`, `IMG_4821.CR3`, acentos, `old == new`); verificar con `cargo test`

## 2. Persistencia y puertos

- [x] 2.1 Añadir en `db/queries.rs` `update_description(Option<&str>)`, permitir `update_session_date` con `None`, y `update_dng_path(id, new_path)`, exponiendo `update_dng_path` en el puerto `PhotoRepository` (con su implementación y los dobles de test existentes); verificar con `cargo check` y `cargo test`
- [x] 2.2 Añadir `MetadataTool::set_image_description(file, Option<&str>)`: desktop con exiftool (`-ImageDescription=`, `-XMP-dc:Description=`, `-overwrite_original`; vacío elimina) y Android actualizando o quitando la descripción del `.xmp` con una función nueva en `import/xmp.rs`, con un test de caracterización en desktop sobre un fixture; verificar con `cargo test`

## 3. Casos de uso

- [x] 3.1 Crear `application/use_cases/rename_project_files.rs` (`RenameProjectFiles`): renombra foto, `.xmp` y sidecar AppleDouble, actualiza `dng_path`, falla ante colisión y deshace en orden inverso ante cualquier fallo, con tests con dobles (todo bien, colisión, fallo a mitad con rollback, fotos que no coinciden se saltan); verificar con `cargo test`
- [x] 3.2 Crear `application/use_cases/rewrite_project_description.rs` (`RewriteProjectDescription`): construye `{descripción}@{año}` igual que el import, escribe por foto con `set_image_description`, acumula fallos sin detenerse y reporta progreso, con tests con dobles (todas bien, algunas fallan, descripción vacía); verificar con `cargo test`

## 4. Comando

- [x] 4.1 Sustituir `rename_project` por `edit_project` en `commands/project.rs` siguiendo el orden de pasos de design.md (validar, fecha efectiva, mover carpeta, actualizar BD, renombrar archivos, reescribir metadatos), en `spawn_blocking`, emitiendo `edit-project-progress` y devolviendo `EditProjectResult`; registrarlo en `lib.rs` en lugar de `rename_project` y actualizar la lista de comandos de `apps/desktop/CLAUDE.md`; verificar con `cargo check` y `cargo test`

## 5. Frontend

- [x] 5.1 Añadir los tipos `EditProjectRequest`/`EditProjectResult` en `lib/types.ts` y `editProject` en `lib/api.ts` (quitando `renameProject`), con la escucha de `edit-project-progress`; verificar con `pnpm exec tsc --noEmit` en `apps/desktop`
- [x] 5.2 Crear `EditProjectModal.tsx` en lugar de `RenameProjectModal.tsx`: carga el proyecto con `get_project`, campos nombre/fecha/descripción con validación de obligatorios, fecha precargada con `session_date ?? created_at.slice(0, 10)`, casillas que aparecen según design.md (desmarcadas), aviso de editores externos al cambiar el nombre, progreso y resumen de fallos; claves `project.edit.*` en `es.json`/`en.json` (sustituyendo `project.rename.*`); verificar con `jq empty` y `tsc`
- [x] 5.3 Conectar el modal en `Projects.tsx` (`handleEdit` en lugar de `handleRename`): tras guardar, `refetch`, `relocateBackup` y `ensureProjectSynced` en segundo plano; verificar con `tsc` y en la app que el menú "Editar" abre el modal nuevo

## 6. Verificación y documentación

- [x] 6.1 Verificar en la app los escenarios de la spec delta de `project-management`: cambiar nombre, fecha y descripción por separado; nombre y fecha vacíos rechazados; guardar sin tocar la fecha en un proyecto sin fecha lo deja sin fecha (y una importación posterior la infiere)
- [x] 6.2 Verificar en la app el renombrado de archivos (con y sin casilla, `IMG_*` intactos, `_exported/` intacto y fotos enlazadas siguen enlazadas) y la reescritura de metadatos (`exiftool -ImageDescription` sobre fotos de `_media/` y `_culled/`, con y sin casilla)
- [x] 6.3 Añadir `docs/useCases/flujo-editar-proyecto.md` con el diagrama Mermaid del flujo (validación, fecha efectiva, carpeta, archivos, metadatos, nube) y la entrada en `[Sin publicar]` de `CHANGELOG.md`; verificar que el formato sigue el de los documentos existentes
