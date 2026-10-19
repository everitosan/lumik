# Design

## Context

- `ImportPage.tsx` (~1040 líneas) tiene dos ramas según `embedded = !!initialProjectId`. `ImportPage` solo se monta desde `ProjectDetail.tsx`, siempre con `initialProjectId`, `initialProjectName` e `initialDeviceId`. Por tanto la rama no embebida (paso de destino, `Select` de proyecto, `DriveSelector`, `CreateProjectModal`, botón "Nueva importación") es código muerto.
- `start_import` (`commands/import_cmd.rs`) recibe solo archivos, proyecto y disco. Lee `rename_on_import` y `embed_metadata_on_import` de `get_app_settings()` y, si toca, los metadatos del fotógrafo activo de la base global. Después llama a `ImportPhotos::execute` con `ImportParams { rename_on_import, metadata: Option<PhotographerMetadata>, … }`. El caso de uso ya recibe estos valores como parámetros.
- Ajustes solo edita `artist`, `copyright` y `contact_url`. `contact_email` y `usage_terms` se escriben en el XMP si existen, pero ninguna pantalla los rellena.
- El renombrado sanea el nombre del proyecto con `char::is_alphanumeric()` (Unicode: conserva acentos), más `-` y `_` (`import/pipeline.rs:358`).
- En el flujo real, el espacio libre (`hasEnoughSpace`) solo se muestra en el resumen de confirmación. La única validación que bloqueaba estaba en el paso de destino, que es la rama muerta.

## Goals / Non-Goals

**Goals:**
- Que el fotógrafo vea y pueda cambiar el renombrado y la firma antes de cada importación, sin tocar la configuración global.
- Que un frontend que no mande ajustes (o una versión anterior) siga importando exactamente como hoy.
- Dejar `ImportPage` con un solo flujo antes de añadir el paso nuevo.

**Non-Goals:**
- Recordar los ajustes de una importación para la siguiente o guardarlos en el proyecto.
- Editar email, teléfono o condiciones de uso.
- Tocar los sidecars de Finder tags.
- Cambiar el caso de uso `ImportPhotos` o el pipeline.

## Decisions

**Ajustes opcionales en la petición.** `ImportRequest` añade `settings: Option<ImportSessionSettings>` (`#[serde(default)]`), con:

```
ImportSessionSettings {
  rename_on_import: bool,
  embed_metadata: bool,
  artist: Option<String>,
  copyright: Option<String>,
  contact_url: Option<String>,
}
```

Del lado de TypeScript, `ImportRequest.settings?: ImportSessionSettings`. La alternativa de guardar los ajustes en la BD global "temporalmente" y restaurarlos al terminar se descarta: es frágil si la importación falla a medias o la app se cierra, y es justo lo que el usuario pidió evitar.

**La resolución es una función pura con tests.** Una función en la capa `application` (p. ej. `application/import_settings.rs`) recibe los ajustes globales, los metadatos guardados y los de la sesión, y devuelve `(rename_on_import, Option<PhotographerMetadata>)`:

- Sin ajustes de sesión: el mismo resultado que hoy (globales).
- Con ajustes de sesión: `rename_on_import` sale de la sesión. Si `embed_metadata` es `false`, los metadatos son `None`. Si es `true`, se parte de los metadatos guardados (o de unos vacíos con el `photographer_id` activo si no hay fila) y se sustituyen `artist`, `copyright` y `contact_url` por los de la sesión. `contact_email` y `usage_terms` se conservan tal como están guardados.
- Los textos se recortan y una cadena vacía cuenta como `None`, para que un campo borrado no escriba una etiqueta vacía.

`start_import` solo llama a esta función en lugar de leer los ajustes a mano. Así la lógica nueva queda cubierta por tests unitarios sin Tauri, siguiendo el patrón de los casos de uso.

**Valores iniciales del paso.** `ImportPage` carga `useAppSettings()` y los metadatos del fotógrafo activo (`getPhotographerMetadata(photographer.id)`, como hace `SettingsPage`) y arma el estado inicial del paso una sola vez, cuando ambos llegan. No vuelve a sobrescribirlo si el usuario ya editó algo. "Restablecer" vuelve a ese snapshot inicial. Por eso `useActivePhotographer` **se conserva** en `ImportPage`: hoy solo lo usa la rama muerta (para crear proyectos), pero el paso nuevo lo necesita.

**Ejemplo de renombrado con el proyecto real.** El ejemplo usa una fecha fija de muestra y el nombre del proyecto saneado en TypeScript con la misma regla que Rust: `name.trim().replace(/[^\p{L}\p{N}_-]/gu, '_')`. La clave i18n lleva interpolación (`{{project}}`).

**Checkbox compartido.** Ajustes ya dibuja sus casillas con `<input type="checkbox">` y estilos propios. Como el paso nuevo necesita las mismas, y la convención del repo es que un componente usado en dos páginas va a `@lumik/ui`, se extrae un `Checkbox` (casilla + etiqueta + descripción opcional) a `packages/ui` y se usa en las dos páginas. Los campos de texto usan el `Input` de `@lumik/ui` que ya usa Ajustes.

**El espacio bloquea el botón de iniciar.** En el paso de confirmación, `canProceed` pasa a ser `hasEnoughSpace` (el disco sale de `initialDeviceId`), y el resumen muestra el aviso cuando no cabe. `useConnectedDevices` y `selectedDrive` se conservan: de ahí sale el `mountPath` que necesita `start_import`.

**Primero la limpieza, después el paso nuevo.** La rama muerta se elimina en tareas propias y se verifica que el flujo sea idéntico antes de insertar el paso de ajustes. Así, si algo se rompe, se sabe qué lo causó. Se eliminan: `ALL_STEPS`, `renderDestinationStep`, `projectOptions`, `handleProjectChange`, `handleDriveSelect`, el estado y el uso de `CreateProjectModal` (el componente se queda, porque lo usa el dashboard), `createProject`, `canProceedFromDestination`, `handleNewImport` y el botón "Nueva importación", y las claves i18n `import.destination.*`, `import.footer.newImport` e `import.errors.createProject` si no tienen otro uso. La prop `initialProjectId` pasa a ser obligatoria (igual que `onClose`), para que TypeScript impida montar el wizard sin proyecto.

## Risks / Trade-offs

- [Borrar por error algo que el flujo real usa, como `useConnectedDevices`, que da el `mountPath`] → La lista de lo que se queda está arriba. Además, `tsc` en `apps/desktop` y una importación real tras la limpieza (antes del paso nuevo) lo confirman.
- [Que el ejemplo de renombrado y el renombrado real no coincidan en nombres con símbolos raros] → La regex de TypeScript replica `is_alphanumeric` con `\p{L}\p{N}`. Diferencias marginales (p. ej. dígitos de otros alfabetos) solo afectarían al ejemplo, no al archivo.
- [Que un usuario espere que lo editado en el paso quede guardado] → El paso lo dice en su título ("solo para esta importación") y la siguiente importación vuelve a mostrar los globales, como pide la spec.
- [Bloquear por espacio con información de disco desactualizada] → `useConnectedDevices` ya se refresca por sondeo. En el peor caso, el botón tarda unos segundos en habilitarse tras liberar espacio.
