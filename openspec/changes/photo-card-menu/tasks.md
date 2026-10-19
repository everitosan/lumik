# Tasks

## 1. Menú compartido en la UI kit

- [x] 1.1 Extraer el dropdown de `ProjectCard` (portal, posición y cierre por clic fuera, `Escape` y scroll/resize) a un componente `CardMenu` en `packages/ui`, y hacer que `ProjectCard` lo use sin cambiar su comportamiento. Verificar que la story de `ProjectCard` en `apps/uikit` sigue abriendo y cerrando el menú igual y que `pnpm --filter @lumik/ui build` pasa.
- [x] 1.2 Añadir a `PhotoCard` los props opcionales `onRevealInFolder`, `onDelete` y `onRestore` y el botón ⋮ en el header, sin cambiar `PHOTO_CARD_HEADER_HEIGHT`. El botón y los items hacen `stopPropagation`, "Eliminar" usa el estilo de peligro y, si no llega ningún handler, no hay botón. Textos i18n en `es`/`en` con el vocabulario de la spec. Verificar con una story de `PhotoCard` en `apps/uikit` (con menú, sin menú y en modo eliminada) que al hacer clic en ⋮ no se dispara `onClick`. *(Hecho con el vocabulario anterior; se adapta en 6.1.)*

## 2. Mostrar en carpeta (backend + UI)

- [x] 2.1 Añadir `reveal_file` en `infrastructure/folder.rs` con `reveal_item_in_dir` de `tauri-plugin-opener` (error si el archivo no existe; variante Android que devuelve error) y el comando `reveal_photo(project_id, photo_id)`, que resuelve `mount_point + dng_path`. Registrarlo en `lib.rs` y en `lib/api.ts`. Verificar que `cargo check` y `cargo clippy` pasan y que el permiso del opener está en `capabilities` si lo exige.
- [x] 2.2 En `ProjectDetail`, pasar `onRevealInFolder` a cada card solo fuera de Android y mostrar el error si el comando falla. Verificar a mano en Linux que el explorador se abre con la foto seleccionada en `_media/` y en `_culled/`. *(Código escrito; falta la verificación manual.)*

## 3. Reglas de dominio y persistencia

- [x] 3.1 Añadir en `domain/paths.rs` las funciones que calculan el destino de la papelera (original → `_deleted/`, exportado → `_deleted/_exported/`) y de restaurar (según `culled`, a `_media/` o `_culled/`, y a `_exported/`), con tests unitarios como los de `cull_destination`, incluido el caso del original JPG con exportado del mismo nombre. *(Hecho con `_deleted/`; pasa a `_trash/` en 6.2.)*
- [x] 3.2 Añadir a `PhotoRepository` y a su implementación SQLite `set_deleted(id, deleted, dng_path, jpg_path)` y la limpieza de portada si `cover_photo_path == ".thumbs/{id}.jpg"`, junto con `get_project_deleted_photos`. Verificar con tests de `queries.rs` sobre una BD en memoria que `get_project_photos` y el conteo del dashboard excluyen la foto en la papelera y que la portada queda en `NULL`.
- [x] 3.3 Hacer que `update_photo_paths_prefix` reescriba también `jpg_path` con el mismo prefijo. Verificar con un test que, tras mover un proyecto, `dng_path` y `jpg_path` de una foto en la papelera apuntan a la nueva carpeta.

## 4. Casos de uso papelera / restaurar

- [x] 4.1 Implementar `DeletePhoto` en `application/use_cases/`: comprobar que ningún destino existe, mover el original y el exportado (obligatorios) y el `.xmp` y los `._` (mejor esfuerzo), y llamar a `set_deleted`. Si falla un paso obligatorio o la BD, deshacer en orden inverso. Verificar con tests con fakes (siguiendo `cull_photo.rs`): foto sin exportado, con exportado, fallo al mover el exportado y fallo de BD, comprobando que en los fallos no queda nada movido.
- [x] 4.2 Implementar `RestorePhoto` con la misma estructura, incluido el conflicto de nombre en destino (falla sin mover nada) y la alternativa cuando `jpg_path` no existe (buscar en la carpeta de exportados de la papelera y, si no está, limpiar `jpg_path`). Verificar con tests con fakes de esos tres casos y del caso de una foto aprobada que vuelve a `_culled/`.
- [x] 4.3 Hacer que `CullPhoto` rechace fotos con `deleted = true` sin mover archivos. Verificar con un test nuevo en `cull_photo.rs`.
- [x] 4.4 Exponer los comandos `delete_photo`, `restore_photo` y `get_project_deleted_photos` en `commands/photo.rs`, registrarlos en `lib.rs` y en `lib/api.ts`. Verificar que `cargo test` del crate desktop pasa completo.

## 5. Flujo en `ProjectDetail`

- [x] 5.1 Crear el modal de confirmación (estructura de `DeleteProjectModal`, sin el aviso de irreversibilidad, con nombre de archivo y mención a la papelera `_trash/`, y `loading`/`error`) y conectar la acción de mover a la papelera: tras confirmar se llama al comando y la foto sale del grid cuando responde. Si estaba abierta en el visor, el visor se cierra. Textos i18n `es`/`en`. Verificar a mano: cancelar no cambia nada; confirmar mueve los archivos a `_trash/` (y `_trash/_exported/` si hay exportado) y el contador del footer baja. *(Código escrito como `DeletePhotoModal`; se adapta en 6.4 y falta la verificación manual.)*
- [x] 5.2 Añadir el `FilterToggle` "Papelera" con contador en la `FilterBar`, sin guardarlo en `project_settings`. Con el toggle activo, la lista sale de la consulta de fotos en la papelera, se combinan los demás filtros y las cards reciben `onRestore` en lugar de la acción de papelera. El atajo de aprobar no aplica la actualización optimista si la foto está en la papelera. Verificar a mano: restaurar devuelve los archivos a su carpeta y la foto reaparece en el grid normal; al reabrir el proyecto el toggle está apagado. *(Código escrito como "Eliminadas"; se adapta en 6.1 y 6.4 y falta la verificación manual.)*
- [x] 5.3 Documentar el flujo en `docs/useCases/` en Mermaid (papelera, restaurar y rollback), siguiendo el formato de `flujo-culling.md`, y enlazarlo en `docs/useCases/README.md`. Verificar que el diagrama se renderiza en la vista previa de Markdown. *(Hecho como `flujo-eliminar-foto.md`; se adapta en 6.5.)*

## 6. Vocabulario de papelera en el código ya escrito

Ver design, decisión 8. La columna `deleted` de la BD, `set_deleted` y `photo.deleted` **no** se renombran.

- [x] 6.1 Textos i18n de `packages/ui` y `apps/desktop` (`es`/`en`): "Eliminar" → "Mover a la papelera", "Eliminadas ({{count}})" → "Papelera ({{count}})", y el mensaje del modal pasa a "Se moverá a la papelera del proyecto (`_trash/`)", sin la frase de que no se borra ningún archivo. Renombrar las claves `deletePhoto` → `trashPhoto`, `showDeleted` → `showTrash` y las de la UI kit equivalentes. Verificar que no queda ninguna clave sin usar ni usada sin definir en ningún idioma.
- [x] 6.2 `domain/paths.rs`: los destinos pasan de `_deleted/` y `_deleted/_exported/` a `_trash/` y `_trash/_exported/`, y se renombran las funciones al vocabulario de papelera. Ajustar sus tests y los de los casos de uso que comprueben rutas. Verificar con `rg _deleted apps packages` que no queda ninguna referencia.
- [x] 6.3 Props y story de `PhotoCard`: `onDelete` → `onTrash` (y el modo "eliminada" de la story pasa a "en la papelera"). Verificar que `pnpm --filter @lumik/ui build` pasa y que la story abre el menú igual.
- [x] 6.4 Backend y API: `DeletePhoto` → `TrashPhoto` (`delete_photo.rs` → `trash_photo.rs`), comandos `delete_photo` → `trash_photo` y `get_project_deleted_photos` → `get_project_trashed_photos` (también en `lib.rs`, `lib/api.ts` y `useProjectDeletedPhotos` → `useProjectTrashedPhotos`), y `DeletePhotoModal` → `TrashPhotoModal` junto con el estado de `ProjectDetail` (`pendingDelete`, `showDeleted`…). Verificar que `cargo test`, `cargo clippy` y `tsc --noEmit` del desktop pasan.
- [x] 6.5 Docs: `docs/useCases/flujo-eliminar-foto.md` → `flujo-papelera-foto.md` con el vocabulario y las rutas nuevas, actualizar el enlace en `docs/useCases/README.md` y revisar las menciones en `apps/desktop/CLAUDE.md`. Verificar que el diagrama se sigue renderizando.
- [x] 6.6 Si en algún disco de prueba quedó una carpeta `_deleted/`, restaurar esas fotos antes de aplicar 6.2 o mover la carpeta a mano a `_trash/`.

## 7. Verificación integrada

- [x] 7.1 En un disco exFAT real con un proyecto que tenga foto aprobada, foto con exportado y foto de portada: mover las tres a la papelera, comprobar el dashboard (conteo y portada), lanzar un respaldo y comprobar que no se suben, restaurarlas y comprobar que `sync_exported` mantiene el enlace del exportado al reabrir el proyecto.
- [ ] 7.2 Verificar en Android que el menú de la foto muestra solo "Mover a la papelera" (y "Restaurar" en la vista de papelera), y que mover a la papelera y restaurar funcionan con el sidecar `.xmp`.
- [x] 7.3 Ejecutar `openspec validate photo-card-menu --strict` y verificar que pasa sin errores. *(Vuelve a quedar pendiente porque las specs cambiaron.)*

## Workflow follow-up

- Registrar como pendiente el cambio "propagar la papelera al catálogo del servidor y al portal web / iPad".
- Registrar como pendiente "Vaciar papelera" desde la app.
- Archivar el change con `/opsx:archive` cuando la verificación integrada esté completa.
