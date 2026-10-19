# Tasks

## 1. Línea base del flujo actual

- [x] 1.1 Antes de tocar código, ejecutar `pnpm exec tsc --noEmit` en `apps/desktop` y `cargo test` en `apps/desktop/src-tauri`, y guardar los resultados como referencia; verificar que ambos terminan sin errores (o anotar los errores previos)
- [x] 1.2 Recorrer en la app el flujo actual (abrir proyecto → Importar → elegir fotos → Siguiente → confirmar → progreso → Aceptar) y anotar lo que se ve en cada paso como referencia para la tarea 2.3

## 2. Eliminar la rama muerta de ImportPage

- [x] 2.1 En `apps/desktop/src/pages/ImportPage.tsx`, eliminar `ALL_STEPS`, `renderDestinationStep`, `projectOptions`, `handleProjectChange`, `handleDriveSelect`, el estado y el uso de `CreateProjectModal`, `handleCreateProject`, `createProject`, `canProceedFromDestination`, `handleNewImport` y el botón "Nueva importación", y simplificar `renderStepContent` y `canProceed` a un solo flujo; conservar `useConnectedDevices`, `selectedDrive`, `selectedProject`, `hasEnoughSpace` y `useActivePhotographer`. Verificar que `pnpm exec tsc --noEmit` en `apps/desktop` no da errores
- [x] 2.2 Hacer obligatorias las props `initialProjectId`, `initialProjectName`, `initialDeviceId` y `onClose` de `ImportPage`, y quitar de `es.json`/`en.json` las claves `import.destination.*` e `import.footer.newImport` (mantener `import.errors.createProject`, que usa `Projects.tsx`); verificar con `tsc`, con `jq empty` sobre los JSON y con `grep` que ninguna clave borrada sigue referenciada
- [x] 2.3 Repetir el recorrido de la tarea 1.2 y verificar que cada paso se comporta igual que en la referencia (origen → confirmar → progreso → Aceptar vuelve al proyecto)

## 3. Ajustes de sesión en el backend

- [x] 3.1 Añadir `ImportSessionSettings` y la función pura de resolución en `apps/desktop/src-tauri/src/application/` según design.md (sin sesión = globales; firma desactivada = `None`; sustituir autor/copyright/URL y conservar email y condiciones de uso; cadenas vacías = `None`), con tests unitarios para cada caso; verificar con `cargo test`
- [x] 3.2 Añadir `settings: Option<ImportSessionSettings>` con `#[serde(default)]` a `ImportRequest` en `commands/import_cmd.rs` y usar la función de resolución en `start_import` en lugar de leer los ajustes a mano; verificar con `cargo test` y `cargo check` que compila, y que una petición sin `settings` produce los mismos parámetros que hoy (test de la función)
- [x] 3.3 Añadir `ImportSessionSettings` y `settings?` a `ImportRequest` en `apps/desktop/src/lib/types.ts`; verificar con `tsc`

## 4. Checkbox compartido

- [x] 4.1 Crear `Checkbox` en `packages/ui/src/components/` (casilla + etiqueta + descripción opcional, estilos con tokens de `@lumik/ui`), exportarlo desde `packages/ui/src/index.ts` y añadir su story en `apps/uikit`; verificar con `pnpm --filter @lumik/ui build` y el build del uikit
- [x] 4.2 Sustituir las casillas de `SettingsPage.tsx` por `Checkbox` sin cambiar su comportamiento; verificar con `tsc` y abriendo Ajustes > Importación, que las tres casillas guardan y recuperan su valor como antes

## 5. Paso de ajustes en el wizard

- [x] 5.1 Añadir el paso `settings` entre `origin` y `confirm` en `ImportPage.tsx`, con su estado inicial armado una sola vez desde `useAppSettings()` y los metadatos del fotógrafo activo, casillas de renombrar y firmar, campos de autor, copyright y URL (deshabilitados si la firma está desactivada) y un botón para restablecer; verificar con `tsc` y en la app que el paso muestra los valores globales
- [x] 5.2 Mostrar el ejemplo de renombrado con el nombre del proyecto saneado (regex `\p{L}\p{N}_-`, igual que Rust) e interpolación i18n, y añadir todas las claves nuevas del paso en `es.json` y `en.json`; verificar con `jq empty` y viendo el ejemplo con un proyecto con acentos y espacios (p. ej. "Boda García" → `Boda_García`)
- [x] 5.3 Enviar los ajustes del paso en `startImport` como `settings`; verificar importando con un copyright cambiado que el archivo importado lo lleva (`exiftool -Copyright` sobre la foto en `_media/`), que Ajustes conserva el copyright anterior y que la siguiente importación vuelve a mostrar el global
- [x] 5.4 Verificar en la app que desactivar el renombrado en el paso conserva los nombres originales sin cambiar el ajuste global, y que desactivar la firma no escribe autor ni copyright

## 6. Confirmación y espacio

- [x] 6.1 Añadir al resumen de confirmación si se renombra y con qué autor se firma (o "sin firma"); verificar en la app que refleja lo elegido en el paso de ajustes
- [x] 6.2 Hacer que el botón de iniciar dependa de `hasEnoughSpace` y mostrar el aviso cuando no cabe; verificar con `tsc` y, en la app, con una selección más grande que el espacio libre del disco (o simulando el valor) que el botón queda desactivado

## 7. Documentación

- [x] 7.1 Actualizar `docs/useCases/flujo-importar-imagenes.md`: wizard de 3 pasos origen → ajustes → confirmar, solo desde un proyecto, `ImportRequest` con `settings` y la resolución contra los globales; verificar que los diagramas Mermaid siguen siendo válidos
- [x] 7.2 Actualizar en `apps/desktop/CLAUDE.md` la descripción de `ImportPage` y del pipeline de importación, y añadir en `[Sin publicar]` de `CHANGELOG.md` la entrada del paso de ajustes y el bloqueo por espacio; verificar que el formato sigue el de las entradas existentes
