# Tasks

## 1. Backend: volver a registrar lo que ya está en `_media/`

- [x] 1.1 Agregar `registered_dng_paths(&[String]) -> Result<HashSet<String>, String>` a `PhotoRepository` (`application/ports.rs`), con la consulta en `db/queries.rs` (`SELECT dng_path FROM photo WHERE dng_path IN (...)`, en tandas) y la implementación en el adaptador de persistencia (D2). Verificar con un test de BD sobre un proyecto temporal: devuelve solo las rutas insertadas, incluido un lote de más de una tanda.
- [x] 1.2 Agregar `ImportPhotos::register_existing` en `application/use_cases/import_photos.rs`, que resuelve cada nombre contra `{project_dir}/_media/`, marca los que no existen como `register_failed`, cuenta como importados los ya registrados y pasa el resto al `register` actual (D2). Actualizar los fakes y verificar con tests: todos nuevos → registrados con miniaturas; uno ya registrado → no se inserta y cuenta en `successful`; uno desaparecido → fallido; falla `create_batch` → todos `register_failed`.
- [x] 1.3 Sacar de `start_import` la inferencia de fecha y el renombrado de carpeta a `finish_import` en `commands/import_cmd.rs`, sin cambiar su comportamiento (D3). Verificar con `cargo test` y `cargo check`.
- [x] 1.4 Agregar el comando `retry_import_register({ session_id, project_id, project_name, file_names })` que devuelve `ImportResult` (`Err` solo para precondiciones), llama a `finish_import` y emite los eventos con el `session_id` recibido; registrarlo en `lib.rs` (grupo de Import) y agregar `api.retryImportRegister` y sus tipos en `src/lib/api.ts` y `src/lib/types.ts` (D2). Verificar con `cargo check`, `cargo test` y `pnpm exec tsc --noEmit` en `apps/desktop`.

## 2. Frontend: botón y resultado acumulado

- [x] 2.1 Función pura `mergeRetryResult(prev, retried, next)` junto a `useImport()` en `src/lib/hooks.ts`, según la tabla de D4. Verificar con `pnpm exec tsc --noEmit` y, a mano en 3.1, los casos de reintento exitoso y parcial de la spec.
- [x] 2.2 `useImport().retry(steps)`: ejecuta en serie la tanda de registro y luego la de copia, cada una con su `session_id`, sin limpiar el registro; fusiona con `mergeRetryResult`; ante un `Err` conserva el resultado anterior y expone el error (D4). Verificar con `pnpm exec tsc --noEmit`.
- [x] 2.3 En `pages/ImportPage.tsx`: botón "Reintentar N" en la cabecera de la lista de fallidos; separa `register_failed` (por `name`) del resto (por `path`); antes de copiar, `refetch` de los dispositivos y comparación del espacio libre con la suma de `sizeBytes` de esos orígenes (botón desactivado y aviso si no cabe); tiempo total acumulado; vista de progreso durante el reintento (D5, D6). Verificar con `pnpm exec tsc --noEmit`.
- [x] 2.4 Agregar las claves `import.result.retry*` (botón, aviso de espacio, error del reintento) a `src/i18n/locales/es.json` y `en.json`. Verificar con `jq empty` (o Python) que los JSON son válidos, sin claves duplicadas, y que todas las claves que usa `ImportPage` existen en los dos idiomas.

## 3. Documentación y prueba de punta a punta

- [x] 3.1 Actualizar `docs/useCases/flujo-importar-imagenes.md` con la rama de reintento (copia y solo registrar) y la sección "Pipeline de importación" de `apps/desktop/CLAUDE.md` (comando nuevo en la lista de comandos y en el flujo). Verificar que el diagrama Mermaid renderiza y que el texto coincide con el código.
- [ ] 3.2 Prueba manual con `pnpm tauri dev` en Linux: (a) quitar el permiso de lectura a 2 fotos del origen, importar, devolverlo y reintentar → entran con los mismos ajustes y el total es el de la sesión; (b) forzar `register_failed` (p. ej. bloqueando `project.db` con otra conexión con `BEGIN EXCLUSIVE`) y reintentar → se registran sin `-001` ni copias nuevas; (c) reintentar algo que vuelve a fallar → sigue en la lista y el botón sigue. Revisar los textos en español y en inglés, y correr `openspec validate import-retry-failed --strict`.

## Workflow follow-up

- Archivar con `/opsx:archive` tras la prueba manual. Este change agrega requisitos propios a `photo-import`, así que no depende de archivar antes `import-feedback-and-errors`.
