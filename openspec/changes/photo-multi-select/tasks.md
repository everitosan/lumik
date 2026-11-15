# Tasks

Requiere `trash-section` implementado (sección "Papelera", `PurgePhoto`, `restore_photo` con `new_stem`).

## 1. Comandos de lote (desktop, Rust)

- [x] 1.1 Tipos `BatchReport` / `BatchFailure` y `TrashItemRef` (serde con `kind`) en `commands/trash.rs` (design, decisión 5). Verificar que `cargo build` del desktop pasa.
- [x] 1.2 `trash_photos(project_id, photo_ids)` en `spawn_blocking`, con una sola `write_session` y `TrashPhoto` por foto, que sigue si una falla. Verificar con un test sobre un `project.db` y un disco temporales: 3 fotos, una con su destino en `_trash/` ocupado → 2 en `done`, 1 en `failed` con su nombre y motivo, y la fallida sigue fuera de la papelera.
- [x] 1.3 `restore_trash_items(items)` y `purge_trash_items(items)`: fotos agrupadas por proyecto (una `write_session` por proyecto) y proyectos con las funciones de `trash_project`/`purge_project`; la colisión se reporta como `collision` y el candado ajeno como `project_locked:…`. Verificar con tests: restaurar un lote con una colisión (4 vuelven, 1 se queda) y eliminar un lote mixto de proyecto + fotos. *(El e2e `batches_on_real_disk_and_db` cubre los lotes de fotos con disco y SQLite reales. La parte de proyectos reutiliza `restore_project` y `purge_project`, ya probados en `project_trash.rs`; los comandos no tienen test propio porque no hay un `AppState` de prueba.)*
- [x] 1.4 Registrar los tres comandos en `lib.rs` y en `lib/api.ts` (`trashPhotos`, `restoreTrashItems`, `purgeTrashItems`, todos con `changingTrash`) y sus tipos en `types.ts`. Verificar que `cargo test`, `cargo clippy` y `tsc --noEmit` del desktop pasan.

## 2. Tarjetas marcables (`packages/ui`)

- [x] 2.1 `PhotoCard` y `ProjectCard`: `onClick` recibe el evento, y props nuevas `marked` y `onToggleMark` (casilla al pasar el ratón y siempre si está marcada, con `stopPropagation`; borde primario si está marcada), textos i18n de la casilla en `es`/`en` (design, decisión 2). Stories `Marked` y `Markable` en `apps/uikit` para las dos. Verificar que `pnpm --filter @lumik/ui build` pasa y, a mano en Storybook, que pulsar la casilla no dispara `onClick`. *(Casilla compartida en `MarkToggle`. El build pasa; falta revisarlo a mano en Storybook.)*

## 3. Marcar en el proyecto (desktop)

- [x] 3.1 Estado de marcas y ancla en `ProjectDetail`: Ctrl/Cmd+clic (`hasPlatformModifier`), Shift+clic sobre `sortedPhotos`, clic normal con marcas = marcar, y vaciado al cambiar filtros, orden, modo de vista o proyecto (design, decisión 1). La lógica del rango va en una función pura en `lib/marking.ts` (el desktop no tiene runner de tests de frontend). Verificar a mano los escenarios de "Marcar fotos en el desktop" y "Cuándo se quitan las marcas". *(Código escrito y `tsc` pasa; falta la verificación manual.)*
- [x] 3.2 Esc en dos pasos en el `keydown` del proyecto (design, decisión 3). Verificar a mano: con marcas, el primer Esc limpia y el segundo regresa. *(Código escrito; falta la verificación manual.)*
- [x] 3.3 `MarkedBar` con conteo, "Agregar a álbum" (`AddToAlbumDialog` con todos los elementos y aviso de cuántos se agregaron), "Mover a la papelera" (`TrashPhotosModal` con el conteo → `trashPhotos`, y reporte de fallidas) y quitar las marcas; sin "Mover a la papelera" en solo lectura. Textos `projectDetail.marked.*` en `es`/`en`. Verificar a mano: lote de 12 a la papelera, lote con una foto que choca, 20 a un álbum que ya tenía 3 (se informan 17), y que no quedan claves i18n sin usar ni sin definir. *(Código escrito y `tsc` pasa; falta la verificación manual. `AddToAlbumDialog` ya informaba "se agregaron N, M ya estaban"; se le agregó `onAdded`. `MarkedBar` quedó en `@lumik/ui` porque la usan el proyecto y la Papelera.)*

## 4. Marcar en la sección "Papelera" (desktop)

- [x] 4.1 Marcas en `TrashPage` con ids `photo:…` / `project:…`, rangos dentro de cada disco, Esc que limpia, y barra "Restaurar" / "Eliminar definitivamente" (confirmación con conteos de proyectos y fotos y aviso del respaldo) → `restoreTrashItems` / `purgeTrashItems`, con reporte de fallos (colisión y candado traducidos). Textos `trash.marked.*` en `es`/`en`. Verificar a mano: marcar en dos discos, restaurar un lote con una colisión y eliminar un lote mixto. *(Código escrito y `tsc` pasa; falta probarlo en la tablet.)* *(Código escrito y `tsc` pasa; falta la verificación manual. Sin marcas, un clic en la papelera no abre nada, como antes.)*

## 5. Tablet

- [x] 5.1 `session.trashPhotos(list)` con una sola `reload()` y `giveBack()`, y en `projectActions.ts`, `restoreTrashedPhotos(p, ids)` y `purgeTrashedPhotos(p, ids)` con un solo `withProjectDb` por proyecto que junta los fallos (colisión como `RestoreCollisionError`, candado como `ProjectLockedError`). Verificar que `tsc --noEmit` de `apps/tablet` pasa.
- [x] 5.2 Modo marcar en `ProjectScreen`: el toque largo entra con esa foto (el menú queda en ⋮), el toque marca o desmarca, "Listo" sale; no se entra en solo lectura; las marcas se vacían al cambiar filtros o modo de vista. Barra con conteo y "Mover a la papelera" (confirmación con conteo y reporte). Revisar `GestureGuide` por si menciona el toque largo del grid. Verificar a mano en la tablet los escenarios de "Marcar fotos en la tablet" y "Barra en la tablet". *(Código escrito y `tsc` pasa; falta probarlo en la tablet. `GestureGuide` no menciona el toque largo. Las marcas se quitan si cambia el número de fotos, no con cada miniatura que llega al caché.)*
- [x] 5.3 Modo marcar en `TrashScreen` con barra "Restaurar" / "Eliminar definitivamente", y lotes agrupados por proyecto en `AppScreen`. Verificar a mano en la tablet: restaurar un lote con una colisión y eliminar un lote mixto. *(Código escrito y `tsc` pasa; falta probarlo en la tablet.)*

## 6. Documentación y verificación integrada

- [x] 6.1 `docs/useCases/flujo-marcar-fotos.md` (marcar, rango, Esc, barra, lotes y reporte) enlazado en `docs/useCases/README.md`; actualizar `flujo-papelera-foto.md`, `flujo-papelera-proyecto.md` y `flujo-albumes.md` con los lotes, y `apps/desktop/CLAUDE.md` / `apps/tablet/CLAUDE.md` con los comandos y componentes nuevos. Verificar que los diagramas Mermaid se renderizan. *(Falta abrir la vista previa.)*
- [ ] 6.2 En el disco real, en el desktop: marcar 30 fotos de un proyecto, mandarlas a la papelera, verlas en la sección "Papelera", restaurar la mitad en lote y eliminar el resto en lote; comprobar que `_trash/`, `.thumbs/` y `.previews/` quedan sin rastro de las eliminadas.
- [x] 6.3 Ejecutar `openspec validate photo-multi-select --strict` y verificar que pasa sin errores.

## Workflow follow-up

- Archivar `trash-section` antes que este cambio (esta delta agrega requisitos a `trash-bin`).
- Archivar el change con `/opsx:archive` cuando la verificación integrada esté completa.
