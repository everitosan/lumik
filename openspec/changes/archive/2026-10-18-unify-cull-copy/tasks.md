# Tasks

## 1. Textos del desktop (i18n)

- [x] 1.1 En `apps/desktop/src/i18n/locales/es.json`, cambiar:
  - `photo.sidebar.cull` a "Aprobación".
  - `settings.keybindings.descriptions.photo_detail.cull` a "Aprobar / quitar aprobación".
  - `settings.keybindings.descriptions.project.show_culled` a "Mostrar solo fotos aprobadas".

  Verificar con `jq empty` sobre el archivo.
- [x] 1.2 En `apps/desktop/src/i18n/locales/en.json`, cambiar:
  - `photo.sidebar.cull` a "Approval".
  - `photo.sidebar.selectPhoto` a "Approve photo".
  - `projectDetail.culled` a "approved".
  - `projectDetail.showCulled` a "Show approved".
  - `…descriptions.photo_detail.cull` a "Approve / remove approval".
  - `…descriptions.project.show_culled` a "Show only approved photos".

  Verificar con `jq empty`.
- [x] 1.3 Comprobar que ningún valor de cadena de los dos archivos use "cull"/"culled", "Descartar", "Recover" ni "Revisión" para la acción o el estado. Las claves y la palabra "culling" en `about.description` sí se quedan. Verificar con `jq -r '.. | strings' <archivo> | grep -inE '\bcull(ed)?\b|descart|recover|revisión'` sin resultados.

## 2. Semilla de atajos en el backend

- [x] 2.1 En `apps/desktop/src-tauri/src/db/queries.rs`, cambiar las descripciones semilla:
  - `photo_detail.cull` a "Aprobar / quitar aprobación".
  - `project.show_culled` a "Mostrar solo fotos aprobadas".

  Verificar con `cargo check` en `apps/desktop/src-tauri`.

## 3. Paquete compartido `@lumik/ui`

- [x] 3.1 Eliminar las claves `photo.cull` y `photo.uncull` de `packages/ui/src/i18n/locales/es.json` y `en.json`. Verificar con:
  - `jq empty` sobre ambos archivos.
  - `grep -rnE "photo\.(un)?cull" apps packages --include=*.ts --include=*.tsx` (excluyendo `node_modules` y `dist`), que no debe dar resultados.
- [x] 3.2 Recompilar el paquete con `pnpm --filter @lumik/ui build` y verificar que `packages/ui/dist/i18n/locales/*.json` ya no contiene `uncull`.

## 4. Documentación

- [x] 4.1 Actualizar `docs/useCases/flujo-culling.md`:
  - "'Solo culled'" pasa a "'Solo aprobadas'".
  - "checkbox 'Select photo'" pasa a "checkbox 'Aprobar foto'".
  - "badge 'culled'" pasa a "badge de aprobada".

  Verificar que el diagrama Mermaid sigue renderizando en el preview de Markdown.

## 5. Verificación integrada

- [x] 5.1 Compilar el desktop con `pnpm --filter @lumik/desktop build` y verificar que termina sin errores.
- [x] 5.2 Revisión manual con `pnpm tauri dev`, en español y en inglés:
  - El panel del visor muestra "Aprobación / Aprobar foto" y "Approval / Approve photo".
  - El footer muestra "N aprobadas · Mostrar aprobadas" y "N approved · Show approved".
  - Ajustes → Atajos describe la barra espaciadora y `Ctrl+c` con los textos nuevos.

## Workflow follow-up

- Archivar el change con `/opsx:archive unify-cull-copy` tras la revisión, para que el requisito de vocabulario quede en `openspec/specs/photo-culling/spec.md`.
