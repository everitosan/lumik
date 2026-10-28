# Tasks

## 1. Base del spike

- [x] 1.1 Extraer `domain`, `application`, `import` y `db` a `crates/lumik-core` sin dependencia de Tauri y hacer que el desktop lo use. Verificar con `cargo test` del desktop (todos los tests siguen pasando) y `pnpm --filter @lumik/desktop build`.
- [ ] 1.2 *(Android hecho (2026-10-27): Expo SDK 58 / RN 0.88 con Fabric en `apps/tablet-spike/` (`com.lumik.tabletspike`), APK de debug arm64 instalado y abierto en una Huawei DBR-W09 (Android 12, 2560×1600). Falta el iPad, en espera de la cuenta Apple Developer.)* Crear `apps/tablet-spike/` (Expo, iPad y Android) que compile y abra en un iPad y una tablet Android reales. Verificar abriendo la app en los dos dispositivos.

## 2. Experimentos

- [ ] 2.1 *(Android medido (2026-10-28) en la DBR-W09, build de release: `crates/lumik-mobile` (uniffi 0.31) expone `start_test_import` (pipeline real `pipeline_passthrough`) y `start_progress_burst`, con el callback `ImportListener` llamado desde un hilo de Rust; turbo module `@lumik/core-native` generado con uniffi-bindgen-react-native 0.31.0-6 en `apps/tablet-spike/modules/core-native`. Resultados con la lista desplazándose y el FPS del hilo JS medido con `requestAnimationFrame`: 1 000 eventos sin pausa en 1.5 s, mínimo 52 fps (reposo 52–60); 100 RAF de 25 MB (2.3 GB) en 10.5 s, 752 eventos, mínimo 58 fps. En debug, 1 000 eventos tardan 53 s con mínimo 5 fps: cada callback bloquea el hilo de Rust hasta que JS lo atiende, así que la app real debe limitar los eventos (p. ej. uno cada 100 ms) para no frenar la copia. Falta repetir en el iPad y mirar el FPS del hilo de UI con el monitor de rendimiento.)* Puente React Native ↔ Rust: exponer con uniffi-bindgen-react-native un import de prueba con progreso por callback. Éxito: la UI sigue respondiendo (desplazar una lista a 60 fps) mientras llegan 1 000 eventos de progreso. Verificar midiendo con el monitor de rendimiento de React Native.
- [ ] 2.2 Tarjeta SD en iPad: elegir la tarjeta en Archivos, guardar el bookmark, reabrir la app sin volver a pedirla e importar 300 RAW con rawler. Éxito: el bookmark sobrevive al reinicio, el tiempo queda dentro de 2× el del desktop, y se documenta qué pasa al mandar la app a segundo plano a mitad de la copia.
- [ ] 2.3 WAL en el disco del iPad: abrir un `project.db` existente en un SSD exFAT por USB-C, escribir 1 000 calificaciones y reabrirlo en el desktop. Éxito: sin errores ni corrupción (`PRAGMA integrity_check` = ok). Si falla, el iPad usará el modo espejado.
- [ ] 2.4 Grid y visor: 2 000 miniaturas en FlashList con expo-image, y visor con pellizco, doble toque y deslizar para aprobar, en iPad y Android. Éxito: desplazamiento sin saltos visibles y zoom que sigue al dedo; validación subjetiva del fotógrafo en sesión real.
- [ ] 2.5 SAF en Android: elegir la carpeta del disco con `OPEN_DOCUMENT_TREE`, copiar 7 GB de la tarjeta al proyecto leyendo por fd en Rust, y devolver `project.db` con `VACUUM INTO` y renombrado. Éxito: el permiso persiste tras reiniciar, la copia queda dentro de 2× el desktop y el disco abre en el desktop con el proyecto al día.

## 3. Interacción táctil

- [ ] 3.1 Prototipo navegable de inicio, importar y culling en las dos variantes (D4 y D5) con los principios de D3. Verificar con una sesión de culling real de 200 fotos en cada tablet y anotar fricciones.

## 4. Decisión

- [ ] 4.1 Escribir la decisión (seguir, ajustar o descartar la arquitectura) con los resultados de 2.1–2.5 y 3.1 en `design.md`, y actualizar la fila de la app de tablet en `CLAUDE.md`. Verificar que `openspec validate tablet-app-spike --strict` pasa.

## Workflow follow-up

- Si la decisión es seguir: proponer el change que construye la app de tablet con sus specs (import en tablet, culling táctil, almacenamiento espejado).
- Retirar `MANAGE_EXTERNAL_STORAGE` del build de Android de Tauri cuando exista la app de tablet.
