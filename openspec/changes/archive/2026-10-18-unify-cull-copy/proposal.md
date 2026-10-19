# Proposal

## Why

En Lumik, "culled" significa **aprobada**: la foto se aparta en `_culled/` porque vale la pena editarla. Los textos de la interfaz no lo transmiten de forma consistente: en español conviven "Aprobar foto", "Revisión" y "Mostrar solo fotos culled", y en inglés "Select photo", "Cull" y "Select for culling". Además, el paquete compartido `@lumik/ui` define "Descartar"/"Recover", que dicen justo lo contrario. Así se confunde la acción principal del culling.

## What Changes

- Unificar el vocabulario visible de la acción y del estado: **aprobar / quitar aprobación / aprobadas** en español y **approve / remove approval / approved** en inglés.
- Desktop, `es.json` y `en.json`:
  - Título de la sección del panel lateral: "Revisión"/"Cull" pasa a "Aprobación"/"Approval".
  - Checkbox del panel: "Select photo" pasa a "Approve photo". El español ya decía "Aprobar foto".
  - Contador y filtro del footer: "culled"/"Show culled" pasan a "approved"/"Show approved". El español ya estaba bien.
  - Descripciones de atajos en Ajustes:
    - `photo_detail.cull` pasa a "Aprobar / quitar aprobación" / "Approve / remove approval".
    - `project.show_culled` pasa a "Mostrar solo fotos aprobadas" / "Show only approved photos".
- Backend del desktop: las descripciones semilla de esos dos atajos en la BD global (`db/queries.rs`) usan el mismo texto en español.
- `@lumik/ui`: eliminar las claves `photo.cull` y `photo.uncull`, que no se usan y son el origen de "Descartar"/"Recover".
- Docs: actualizar `docs/useCases/flujo-culling.md` con los textos nuevos ("Solo aprobadas", "Aprobar foto").
- Fuera de alcance:
  - Los identificadores internos (`culled`, `_culled/`, `show_culled`, `save_photo_culled`, las claves i18n) no cambian, así que no hay migración de datos ni de API.
  - "culling", como nombre de la actividad en textos descriptivos (por ejemplo, la descripción de "Acerca de"), se mantiene.

## Capabilities

### New Capabilities

_Ninguna._

### Modified Capabilities

- `photo-culling`: se añade un requisito sobre el vocabulario visible de la aprobación (aprobar / quitar aprobación / aprobadas en español, approve / remove approval / approved en inglés), con la prohibición explícita de "descartar"/"recover" o de "cull/culled" como etiqueta de un control.

## Impact

- `apps/desktop/src/i18n/locales/es.json`, `apps/desktop/src/i18n/locales/en.json`: solo cambian valores; las claves se mantienen.
- `apps/desktop/src-tauri/src/db/queries.rs`: los textos semilla de dos atajos. Las BD existentes conservan el texto anterior, porque la semilla usa `INSERT OR IGNORE`, pero la UI lo muestra traducido desde i18n, así que no hace falta migración.
- `packages/ui/src/i18n/locales/es.json`, `packages/ui/src/i18n/locales/en.json`: se eliminan dos claves. `dist/` se regenera al compilar.
- `docs/useCases/flujo-culling.md`.
- Sin cambios en el portal web, que no tiene textos de culling, ni en el servidor.
