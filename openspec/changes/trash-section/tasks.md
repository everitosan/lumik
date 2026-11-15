# Tasks

## 1. Reglas de rutas en `lumik-core`

- [x] 1.1 En `domain/paths.rs`, agregar `project_trash_destination`, `project_restore_destination` y `parse_trashed_at` (design, decisión 1), con hora local y formato `_trash/{AAAA-MM-DD_HH-MM-SS}/{ruta}` (antes `{ruta}__{fecha}`; ver 7.1). Verificar con tests unitarios: estructura por fecha, ruta plana, sufijo inválido o ausente, nombre con `__` propio (`Boda__Ana`) y dos sufijos distintos para el mismo proyecto.
- [x] 1.2 Agregar a `paths.rs` la guarda "la ruta resuelta está bajo `{mount}/lumik/_trash/`", que usan los borrados de proyecto. Verificar con tests que rechaza `lumik/2026/09/x`, `lumik/_trash/../2026/x` y rutas fuera del disco.

## 2. Proyectos a la papelera, restaurar y eliminar (núcleo + desktop)

- [x] 2.1 Extraer de `relocate_project_folder` el helper "cerrar SQLite, mover, limpiar ancestros vacíos hasta un tope", y que `relocate_project_folder` lo use. Verificar que los tests existentes de editar proyecto (renombrar y cambiar de fecha) siguen pasando con `cargo test`.
- [x] 2.2 Reemplazar `delete_project` por `trash_project` (design, decisión 3): candado, sesiones, índice, `backup_queue`, registro, `rename` y rollback si el `rename` falla. Verificar con un test sobre un disco temporal: la carpeta queda en `lumik/_trash/…__fecha`, `discover_projects_on_device` ya no la lista y un proyecto con candado ajeno no se mueve. *(Los movimientos y el escaneo se prueban en `project_trash.rs`. El candado reutiliza `ensure_not_locked`; el comando no tiene test propio porque no hay un `AppState` de prueba.)*
- [x] 2.3 Caso de uso `ListTrash` por disco (design, decisión 2): proyectos de `lumik/_trash/` con nombre, fecha de sesión, fotos, portada y fecha de papelera, y fotos sueltas de los proyectos activos del disco, sin tocar el registro de proyectos abiertos. Verificar con un test: un proyecto en la papelera con fotos en su propio `_trash/` aparece como un solo elemento, y las fotos sueltas de otro proyecto se listan con su proyecto.
- [x] 2.4 `restore_project` y `check_restore_target` (design, decisión 4), con error tipado de colisión y restauración con otro nombre a través del flujo de editar nombre (`RenameProjectFiles`). Verificar con tests: ruta libre, ruta ocupada sin mover nada, otro nombre con fotos renombradas, `.lumik-lock` viejo quitado, y el proyecto vuelve al índice de búsqueda.
- [x] 2.5 `purge_project` (design, decisión 6) con la guarda de 1.2, limpieza de ancestros vacíos bajo `lumik/_trash/` y salida del índice. Verificar con tests que borra la carpeta, quita `lumik/_trash/2026/09/` si queda vacía y rechaza una ruta de un proyecto activo.
- [x] 2.6 Registrar los comandos nuevos en `lib.rs` y en `lib/api.ts` (`trash_project`, `list_trash`, `restore_project`, `check_restore_target`, `purge_project`), y quitar `delete_project`. Verificar que `cargo test`, `cargo clippy` y `tsc --noEmit` del desktop pasan.

## 3. Fotos: restaurar con otro nombre y eliminar definitivamente (núcleo + desktop)

- [x] 3.1 Generalizar `FileStore::remove_file` como borrado general de un archivo y que no falle si el archivo no existe, en `infrastructure/fs.rs` y en el `FileStore` de los tests. Verificar con un test de `fs.rs`. *(`StdFileStore::remove_file` ya era general; `PurgePhoto` tolera `NotFound`. Se verificó con el e2e `restore_as_and_purge_on_real_disk_and_db` en vez de un test de `fs.rs`.)*
- [x] 3.2 `RestorePhoto` con `new_stem` opcional (design, decisión 5): todos los destinos (original, `.xmp`, `._`, JPG del par, exportado) se comprueban antes de mover, con error de colisión tipado. Verificar con tests: colisión del original, colisión solo del exportado, par con otro nombre, y que el siguiente `sync_exported` mantiene el enlace.
- [x] 3.3 Caso de uso `PurgePhoto` (design, decisión 6): rechaza fotos sin `deleted = 1`, borra archivos ignorando los faltantes, borra `photo_embedding` y `photo` en una transacción y refresca el índice. Verificar con tests: foto con exportado y sidecars, par RAW+JPG, archivo ya ausente, y foto que no está en la papelera (rechazada).
- [x] 3.4 `empty_trash(device)` (design, decisión 7), con reporte de eliminados y fallidos, y candado ajeno reportado como fallo por foto. Verificar con un test: dos proyectos en la papelera y fotos sueltas en un proyecto con candado ajeno → se borran los proyectos, las fotos quedan y se reportan. *(Sin test del comando, por la misma falta de `AppState` de prueba: los borrados que encadena están probados en `project_trash.rs`, `trash_photo.rs` y el e2e.)*
- [x] 3.5 Exponer `restore_photo` con `new_stem`, `purge_photo` y `empty_trash` en `commands/photo.rs` / `lib/api.ts`. Verificar que `cargo test`, `cargo clippy` y `tsc --noEmit` pasan.

## 4. UI del desktop

- [x] 4.1 Componentes en `packages/ui`: `ProjectCard` y `PhotoCard` aceptan `onRestore`/`onPurge` en su menú y la tarjeta de proyecto muestra la fecha de papelera; nuevo `TrashDiskGroup` (encabezado con disco y "Vaciar papelera de {disco}"). Stories en `apps/uikit` con papelera vacía, un disco y dos discos. Verificar que `pnpm --filter @lumik/ui build` pasa y que las stories se ven en Storybook. *(`TrashDiskGroup` quedó dentro de `pages/Trash/TrashPage.tsx` por la regla de componentes exclusivos de una página, sin story. Hay stories nuevas `ProjectCard/InTrash`, `PhotoCard/InTrash` con eliminar y `SectionButton/WithBadge`. El build pasa; falta revisar Storybook a mano.)*
- [x] 4.2 Entrada "Papelera" en `Sidebar.tsx` justo antes de Configuración, con icono `trash` y conteo, visible solo si el conteo es mayor que cero, y la sección `'trash'` en el `Layout`, que vuelve a `'projects'` si el conteo llega a cero. Hook `useTrashCount`, que se recalcula al cambiar los dispositivos y después de cada acción (también desde `ProjectDetail` y el dashboard). Verificar a mano: sin nada en la papelera no hay entrada; aparece al mandar la primera foto; el conteo coincide con los discos conectados; al restaurar lo último estando en la sección, se vuelve a Proyectos y la entrada desaparece; expulsar el único disco con papelera la quita. *(Código escrito y `tsc` pasa; falta la verificación manual.)*
- [x] 4.3 Página `Trash` con un grupo por disco conectado, la lista de proyectos y el grid de fotos, el estado vacío, y "Restaurar" y "Eliminar definitivamente" por elemento. Verificar a mano con dos discos conectados. *(Código escrito y `tsc` pasa; falta la verificación manual.)*
- [x] 4.4 Modales `TrashProjectModal` (reemplaza `DeleteProjectModal` en el dashboard), `PurgeModal`, `EmptyTrashModal` y `RestoreCollisionModal` con validación en vivo por `check_restore_target`; textos i18n `es`/`en` que avisan que el respaldo en la nube se conserva y se borra desde el portal web. Verificar a mano cada modal, incluido "Cancelar" sin cambios, y que no quedan claves i18n sin usar ni usadas sin definir. *(Código escrito y `tsc` pasa; falta la verificación manual.)*
- [x] 4.5 `ProjectDetail`: quitar el `FilterToggle` "Papelera", la lista de fotos en la papelera y el `onRestore` de las cards; cambiar el texto de `TrashPhotoModal` a "desde la sección Papelera". Verificar que `tsc --noEmit` pasa y que el grid del proyecto nunca muestra fotos en la papelera.
- [x] 4.6 Actualizar `docs/useCases/flujo-papelera-foto.md` (restaurar y eliminar desde la sección) y crear `docs/useCases/flujo-papelera-proyecto.md` (mandar, restaurar con colisión, eliminar, vaciar), enlazados en `docs/useCases/README.md`, y revisar las menciones en `apps/desktop/CLAUDE.md`. Verificar que los diagramas Mermaid se renderizan en la vista previa. *(Falta abrir la vista previa.)*

## 5. Tablet Android

- [x] 5.1 Exponer en `crates/lumik-mobile` las reglas de rutas de 1.1–1.2, `restore_photo` con `new_stem` y `purge_photo` sobre la copia espejada, y `remove_file` en `DiskFiles`. Verificar con tests en `disk.rs` como `trash_and_restore_move_the_file`. *(Están en `trash.rs`: `rules_match_the_desktop` y `restore_as_and_purge_on_the_working_copy`, no ignorados. Los bindings se regeneraron con `ubrn build android`.)*
- [x] 5.2 `projects.ts`: `walk` se salta las carpetas que empiezan con `_`. Verificar con el disco real que un proyecto en `lumik/_trash/` no aparece en "Proyectos". *(Código escrito y `tsc` pasa; falta probar en la tablet.)*
- [x] 5.3 `projectActions.ts`: `deleteProject` pasa a `trashProject` (`Saf.move` + `Saf.rename` y limpieza de carpetas vacías), más `restoreProject` (con colisión y otro nombre vía `editProjectDetails`) y `purgeProject` (`Saf.delete` solo bajo `lumik/_trash/`). Verificar a mano en la tablet: mandar, restaurar a ruta libre, restaurar con otro nombre y eliminar. *(Código escrito y `tsc` pasa; falta probar en la tablet.)*
- [x] 5.4 Nuevo `trash.ts` (proyectos de `lumik/_trash/` y fotos sueltas desde las copias de `covers.ts`), entrada "Papelera" en el `RAIL` de `HomeScreen.tsx` justo antes de "Ajustes", visible solo si hay algo en la papelera (vuelve a "Proyectos" si se vacía estando ahí), y pantalla agrupada por disco con restaurar, eliminar, vaciar y los mismos modales que el desktop. Verificar a mano con el disco real. *(Código escrito y `tsc` pasa; falta probar en la tablet.)*
- [x] 5.5 Quitar la vista de papelera de `ProjectScreen`/`AppScreen` (`trashGrid`, `fromTrash`, `onRestore`) y cambiar el texto de confirmación del menú del proyecto. Verificar que `tsc --noEmit` de `apps/tablet` pasa y que el grid del proyecto no muestra fotos en la papelera. *(`tsc` pasa; falta revisar el grid en la tablet.)*

## 6. Verificación integrada

- [ ] 6.1 En el disco exFAT real (KINGSTON): mandar un proyecto a la papelera desde el desktop, conectar el disco a la tablet y comprobar que aparece en su "Papelera" y no en "Proyectos"; restaurarlo en la tablet y verlo de nuevo en el dashboard del desktop.
- [ ] 6.2 En el disco real: mandar fotos sueltas de dos proyectos a la papelera, vaciar la papelera del disco con un segundo disco conectado, y comprobar que el segundo disco no cambia, que no quedan archivos en `_trash/`, `.thumbs/` ni `.previews/` de esas fotos y que el respaldo en la nube sigue intacto en el portal web.
- [ ] 6.3 Medir en la tablet cuánto tarda mandar a la papelera un proyecto grande (más de 1 000 fotos) en el disco real, para confirmar que `Saf.move` mueve sin copiar.
- [x] 6.4 Ejecutar `openspec validate trash-section --strict` y verificar que pasa sin errores.

## Workflow follow-up

- Archivar el change con `/opsx:archive` cuando la verificación integrada esté completa.
- Registrar como pendiente "papelera o deshacer para álbumes" si se quiere después.

## 7. Un solo movimiento y avance animado (tras medir en la tablet)

- [x] 7.1 Cambiar la ubicación a `_trash/{fecha}/{ruta}` en `paths.rs`, sin dejar de leer ni restaurar el formato `{ruta}__{fecha}`; `parse_trashed_at` lee la ruta completa (también en `lumik-mobile` y `trash.ts`). Verificar con tests de los dos formatos.
- [x] 7.2 `MoveSheet` en la tablet: confirmación con íconos al mandar, animación y segundos transcurridos mientras se mueve, marca de hecho al terminar; la misma hoja al restaurar (uno, con otro nombre y en lote con proyectos).
- [ ] 7.3 Probar en la tablet con el disco real: mandar y restaurar un proyecto grande (se espera la mitad de 37.7 s), restaurar uno del formato anterior y ver la animación.
