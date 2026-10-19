# Proposal

## Why

Al importar, Lumik aplica en silencio el renombrado y los datos del fotógrafo de la configuración global: el wizard no los muestra en ningún paso, y el backend los lee de la base global en el momento de importar. Si el fotógrafo olvidó cómo los dejó (o necesita otro copyright para un cliente concreto), se entera cuando las fotos ya están renombradas y firmadas en el disco. Hace falta que pueda revisarlos, y cambiarlos solo para esa importación, antes de empezar.

## What Changes

- El wizard de importación añade un paso **Ajustes** entre Origen y Confirmación. Muestra, precargados con los valores globales:
  - si se renombran las fotos por fecha de captura, con un ejemplo del nombre resultante usando el proyecto real;
  - si se firman las fotos con los datos del fotógrafo, y los campos autor, copyright y URL, editables.
- Los valores elegidos en ese paso se aplican solo a esa importación y **no** modifican la configuración global. Un botón restablece los valores globales.
- El paso de Confirmación resume los ajustes elegidos (renombrado sí/no, con qué autor se firma).
- El botón de iniciar la importación se desactiva cuando el disco del proyecto no tiene espacio suficiente para los archivos seleccionados. Hoy el espacio libre solo se muestra.
- `start_import` acepta los ajustes de la importación como parte de la petición, de forma opcional. Si no llegan, sigue usando los globales.
- Se elimina la rama del wizard que importaba desde el dashboard (paso de destino, selector de proyecto y disco, creación de proyecto dentro del wizard). Ninguna pantalla la usa: la importación solo se abre desde un proyecto.
- Las etiquetas para iPad/Mac (sidecars de Finder tags) quedan fuera del paso: se escriben al etiquetar fotos, no al importar, y siguen siendo un ajuste global.

## Capabilities

### New Capabilities
<!-- Ninguna -->

### Modified Capabilities
- `photo-import`: el wizard pasa a ser Origen → Ajustes → Confirmación y solo se abre desde un proyecto; el renombrado y los metadatos usan los ajustes de la importación; los metadatos escritos se limitan a lo que el fotógrafo puede configurar (autor, copyright, URL); se bloquea iniciar sin espacio suficiente.
- `app-settings`: el requisito de metadatos del fotógrafo se corrige a los campos que realmente se editan (autor, copyright, URL) y aclara que son la base de cada importación, que puede cambiarlos sin modificarlos.

## Impact

- Frontend desktop: `apps/desktop/src/pages/ImportPage.tsx` (paso nuevo, limpieza de la rama muerta), `src/lib/types.ts` (`ImportRequest`), `src/i18n/locales/{es,en}.json` (claves nuevas del paso; se eliminan `import.destination.*` y `import.footer.newImport`).
- Backend desktop: `src-tauri/src/commands/import_cmd.rs` (resolver ajustes de la petición contra los globales). El caso de uso `ImportPhotos` ya recibe `rename_on_import` y `metadata` como parámetros y no cambia.
- Documentación: `docs/useCases/flujo-importar-imagenes.md` y la tabla del wizard en `apps/desktop/CLAUDE.md`.
- Sin cambios en la base de datos ni en el portal web.
- Fuera de alcance: escribir en las fotos el email, el teléfono o las condiciones de uso (existen en la base global pero ninguna pantalla los edita).
