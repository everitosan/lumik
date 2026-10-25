# Proposal

## Why

La tablet tiene que hacer el flujo completo de Lumik (importar la tarjeta, culling, entregar y respaldar) en iPad y en tablets Android, publicada en App Store y Google Play, y sentirse natural y táctil en cada sistema. La UI actual es web (Tauri en Android, portal en iPad) y su techo está justo en esa sensación nativa. Antes de comprometer una app nueva hay que comprobar con prototipos los riesgos que deciden la arquitectura.

## What Changes

- Registrar la arquitectura candidata: app de tablet en React Native (Expo) con componentes nativos de cada sistema (Expo UI sobre SwiftUI y Jetpack Compose) y el núcleo Rust de Lumik compartido con el desktop mediante uniffi; el desktop sigue en Tauri.
- Registrar el modelo de almacenamiento por plataforma: acceso directo en iPad con bookmarks y modo espejado de `project.db` en Android por SAF, con el candado de `project-lock`.
- Registrar los principios de interacción táctil y los flujos de iPad y Android.
- Hacer un spike con criterios de éxito medibles y cerrar con una decisión escrita: seguir con esta arquitectura, ajustarla o descartarla.
- No se publica nada ni cambia el comportamiento de las apps actuales.

## Capabilities

### New Capabilities

_Ninguna: el spike no cambia el comportamiento del producto (`skip_specs: true`). Las capacidades de la app de tablet se especificarán en el change que la construya._

### Modified Capabilities

_Ninguna._

## Impact

- Código de prototipo en una carpeta de spike (`apps/tablet-spike/`) que no se distribuye.
- Extracción posible del núcleo a `crates/lumik-core` (domain, application, import) sin dependencia de Tauri; si se hace, el desktop debe seguir pasando sus tests.
- Depende de `project-lock` para el formato del candado.
- `CLAUDE.md` del monorepo: la fila "App iPad (Swift/SwiftUI)" se revisará según la decisión.
