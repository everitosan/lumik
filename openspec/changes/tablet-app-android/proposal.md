# Proposal

## Why

El spike de tablet (`tablet-app-spike`, 4.1) cerró con "seguir": en Android, React Native con el núcleo por uniffi y el modo espejado por SAF cumplen, y el prototipo ya hace el flujo de proyectos, import y culling con los casos de uso del núcleo. Falta convertirlo en una app publicable en Google Play. Mientras tanto, Android se cubre con el build de Tauri, que necesita `MANAGE_EXTERNAL_STORAGE`, un permiso que Google Play no aprueba para Lumik.

## What Changes

- **La app de tablet sale del prototipo.** `apps/tablet-spike` pasa a `apps/tablet`, con su propio id de app. Se quitan las pestañas de experimentos y lo que solo era medición, y se conserva el registro de errores por etapa del import.
- **El flujo de campo completo en la tablet:**
  - proyectos (inicio, crear, editar, archivar, eliminar), con las mismas reglas que el desktop;
  - import desde el proyecto en tres pasos, como el desktop: archivos, ajustes (renombrar, firmar y la acción al aprobar del proyecto) e importar;
  - culling táctil, con la entrega del JPG de cámara al aprobar y rotación (botones, guardada en el `.xmp`), con las fotos verticales bien giradas;
  - buscador en el inicio que filtra los proyectos (sin búsqueda de fotos);
  - papelera y filtros;
  - discos con nombre;
  - el candado del proyecto.
- **Ajustes en la tablet:**
  - datos del fotógrafo;
  - firmar y renombrar al importar;
  - nombre del dispositivo para el candado.
- **Publicación en Google Play:**
  - AAB firmado, sin permisos de almacenamiento amplio;
  - ficha y prueba interna;
  - versión ligada a los tags del monorepo.
- **BREAKING: se retira el build de Android de la app Tauri.** El desktop deja de compilarse para Android: se quitan `gen/android`, su código `#[cfg(target_os = "android")]` y las ramas de Android de la UI. Las specs que hablaban de "Android" pasan a referirse a la app de tablet o pierden ese caso.
- **En las dos apps:** el formulario de crear proyecto dice "Nombre del proyecto" y "Disco destino", y en la tablet la fecha se elige con un selector de fecha. Un proyecto sin portada elegida muestra su primera foto por fecha de captura.
- **Fuera de alcance:** respaldo a la nube, búsqueda de fotos (texto y contenido; necesita un índice en la tablet), análisis por contenido y álbumes en la tablet; la app de iPad, que espera la cuenta de Apple; y anunciar Android en la landing.

## Capabilities

### New Capabilities

- `tablet-storage`: acceso a los discos por SAF (permiso persistente, dar acceso, expulsar) y modo espejado de `project.db`: copia de trabajo, devolución al disco, recuperación y escritura sin sobrescribir.
- `tablet-import`: import de la tarjeta en la tablet: elegir archivos, una sola lectura de la tarjeta hacia el disco, firma en sidecar `.xmp`, progreso y errores por etapa.
- `tablet-culling`: visor táctil con gestos, guía, deshacer y háptica; estrellas y colores; modo limpio; aprobar con la opción de entrega del proyecto.
- `tablet-settings`: ajustes de la tablet: fotógrafo, firma y renombrado al importar, y nombre del dispositivo.
- `tablet-distribution`: publicación en Google Play: id de la app, firma, permisos, versión y prueba interna.

### Modified Capabilities

- `device-detection`: el almacenamiento interno de Android y el refresco de discos pasan a la app de tablet. Se quita "Nombre amigable en Android", que era de la app Tauri.
- `photo-import`: la firma en sidecar `.xmp` es de la app de tablet, no de "Android".
- `photo-rating`: el caso "Android" de los sidecars para iPad pasa a la app de tablet.
- `photo-rotation`: el caso de Android pasa a la app de tablet, que guarda la orientación en el `.xmp`.
- `project-management`: portada por defecto (primera foto por fecha de captura) y textos y fecha del formulario de crear proyecto.
- `photo-search`: en la tablet el buscador del dashboard solo filtra proyectos.
- `photo-viewer`: los gestos táctiles dejan de citar Android, y "Mostrar en carpeta" existe en la tablet, donde abre la app de Archivos.

## Impact

- **Tablet:**
  - `apps/tablet-spike` pasa a `apps/tablet` (paquete, `app.json`, id `com.lumik.tablet`).
  - Cambia el `ubrn.config.yaml` de `modules/core-native`.
  - `crates/lumik-mobile` se adapta a los ajustes y a la firma del import.
- **Desktop:**
  - Se borra `apps/desktop/src-tauri/gen/android`.
  - Se quita el código Android de `src-tauri`: `exif_android.rs`, `AndroidMetadata`, `AndroidImaging`, `scan_mounted_devices_android` y la firma del import en Android.
  - Se quitan las ramas `android` de la UI del desktop.
  - `apps/desktop/CLAUDE.md`.
- **Monorepo:**
  - La fila de la tablet en `CLAUDE.md`.
  - Los workspaces de pnpm.
  - El workflow de release, que necesita un job de Android para el AAB (o un paso manual documentado).
- **Rama:** se trabaja en `release/0.2.0`, después de integrar `spike/tablet-app`.
- **Cuentas:** hace falta una cuenta de Google Play Console y la clave de subida, que pone el usuario.
