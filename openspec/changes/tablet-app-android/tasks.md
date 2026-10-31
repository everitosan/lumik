# Tasks

## 1. Rama

- [x] 1.1 Integrar `spike/tablet-app` en `release/0.2.0` con avance rápido (`git merge --ff-only`) y seguir allí. Verificar que `git log release/0.2.0` incluye `0bfb132` y que `cargo test` del núcleo y del desktop pasan.

## 2. De prototipo a app

- [ ] 2.1 `git mv apps/tablet-spike apps/tablet`; paquete `@lumik/tablet`, nombre "Lumik", id `com.lumik.tablet` en Android e iOS; ajustar rutas de `pnpm-workspace.yaml`, `modules/core-native/ubrn.config.yaml` y referencias en docs. Verificar con `pnpm install`, `pnpm --filter @lumik/tablet typecheck` y un APK de release instalado que abre.
- [ ] 2.2 Quitar las pestañas de experimentos: `App.tsx` monta la app sin pestañas; borrar `BridgeScreen`, `SafScreen`, el benchmark y las fotos de demo (`photos.ts`) de `CullingScreen`, moviendo a la app lo que `ProjectScreen` use de ahí; `src/proto/` pasa a `src/`. Verificar con `typecheck`, `grep -r "Experimento\|2\.4\|2\.5\|2\.1" src` vacío y la app abriendo en el inicio.
- [ ] 2.3 Quitar de `lumik-mobile` las funciones de experimentos (`start_test_import`, `start_progress_burst`, `start_fd_copy`, `start_fd_read`, `start_fd_write`, `start_copy_to_dir` y lo que solo ellas usen) y regenerar `@lumik/core-native`. Verificar con `cargo test -p lumik-mobile`, `typecheck` y un APK de release que importa y abre un proyecto.
- [x] 2.4 Actualizar `apps/tablet/AGENTS.md` (navegación por estado, no Expo Router; build con `ubrn` y Gradle; reglas de SAF de `tablet-storage`) y crear `apps/tablet/CLAUDE.md` con la estructura de la app. Verificar leyendo los dos archivos.

## 3. Ajustes e import

- [x] 3.1 En `lumik-mobile`: registro `ImportSettings { sign, rename, author, copyright, url }` en `start_card_import`, pasado a `ImportParams` (`metadata`, `rename_on_import`), con la firma en `.xmp` por `AndroidMetadata`; `set_install_name` para el nombre de la tablet. Tests: import con firma deja `.xmp` con el autor y no cambia el RAW; sin renombrar conserva el nombre de la cámara. Verificar con `cargo test -p lumik-mobile`.
- [ ] 3.2 Pantalla "Ajustes" en el rail: fotógrafo (autor, copyright, URL), "Firmar las fotos al importar" y "Renombrar por fecha de captura al importar" (activos por defecto) y nombre de la tablet, guardados en la sandbox; el import los usa. Verificar en la tablet: importar con autor deja `.xmp` en el disco, desactivar el renombrado conserva `DSCF0001.RAF`, y el desktop muestra el proyecto abierto en el nombre nuevo de la tablet.
- [ ] 3.3 Espacio libre en el paso de importar: tamaño total de lo elegido contra `freeBytes` del volumen; si no alcanza, botón desactivado con el aviso. Verificar en la tablet eligiendo más fotos de lo que cabe en una memoria USB chica.
- [ ] 3.4 Import y candado: si el proyecto lo tiene otro dispositivo, el import no empieza y lo dice (`tablet-import`). Verificar con un `.lumik-lock` ajeno en el disco.
- [ ] 3.5 Entrega al aprobar: comprobar en la tablet que aprobar un par en un proyecto con entrega "el JPG de los pares" deja el RAW en `_culled/` y el JPG en `_exported/`, y que "Deshacer" los regresa a `_media/`; corregir lo que falle. Verificar abriendo el proyecto en el desktop con las mismas rutas.

## 4. Retirar Tauri Android

- [x] 4.1 Borrar `apps/desktop/src-tauri/gen/android`, `exif_android.rs`, la sección `[target.'cfg(target_os = "android")'.dependencies]` y las ramas `#[cfg(target_os = "android")]` de `src-tauri/src` (con lo que solo existía para Android: `scan_mounted_devices_android`, `friendly_volume_name`, `AndroidMetadata`, `AndroidImaging`, firma de import en Android). Verificar con `cargo test` del desktop, `cargo build` y `grep -rn 'target_os = "android"' apps/desktop/src-tauri/src` vacío.
- [x] 4.2 Quitar las ramas de Android de la UI del desktop (`Sidebar`, `Layout`, `Projects`, `ProjectDetail`, `PhotoDetail`, `PhotoSidebar`, `ImportPage`, `hooks.ts`, `api.ts`). Verificar con `pnpm --filter @lumik/desktop build` y `grep -rn android apps/desktop/src` sin ramas de plataforma.
- [x] 4.3 Actualizar `apps/desktop/CLAUDE.md` (sin Android: estructura, pipeline, tabla de estado), la fila del desktop en `CLAUDE.md` ("Linux, Windows, macOS") y la memoria del build de Android de Tauri. Verificar leyendo los cambios.

## 5. Google Play

- [x] 5.1 `app.config.ts` en lugar de `app.json`: versión y `versionCode` desde `LUMIK_VERSION` (`major*1 000 000 + minor*10 000 + patch*100 + beta`, estable = 99), y config plugin de firma de release desde variables de entorno (keystore, alias y contraseñas). Verificar con `expo prebuild` y un `bundleRelease` local firmado cuya `versionCode` para `0.2.0-beta.3` sea 20003.
- [x] 5.2 Comprobar el manifiesto del AAB: sin `MANAGE_EXTERNAL_STORAGE`, `READ_EXTERNAL_STORAGE` ni `WRITE_EXTERNAL_STORAGE`; quitar los que agreguen dependencias con `tools:node="remove"` si hace falta. Verificar con `bundletool dump manifest`.
- [ ] 5.3 Job `build-tablet-android` en `release.yml`: Rust, NDK r26, `cargo-ndk`, `ubrn build android --release`, `expo prebuild`, `bundleRelease` firmado con secretos, AAB al release de GitHub y, si existe el secreto de la cuenta de servicio, a la pista interna de Play. Verificar con un tag de prueba en un fork o con `act`, y luego con la primera beta.
- [ ] 5.4 Crear la app en Play Console (lo hace el usuario: cuenta, ficha, clave de subida, cuenta de servicio) y documentar en `apps/tablet/CLAUDE.md` qué secretos usa el workflow. Verificar que la primera beta aparece en la pista interna y se instala en la DBR-W09 desde Play.

## 6. Integración

- [ ] 6.1 Sesión de campo con la beta de Play: importar una tarjeta real (100+ fotos) al disco, culling de 200 fotos con gestos, expulsar, y abrir el proyecto en el desktop con las aprobadas, estrellas, colores y `.xmp` al día. Anotar fricciones. Correr `cargo test`, `pnpm --filter @lumik/desktop build`, `pnpm --filter @lumik/tablet typecheck` y `openspec validate tablet-app-android --strict`.

## Workflow follow-up

- Archivar `tablet-app-spike` después de 1.1, dejando los experimentos de iPad como pendiente del change que construya la app de iPad.
- Archivar este change tras la 6.1 y sincronizar sus deltas.
- Un change posterior: inglés en la tablet, y la app de iPad cuando exista la cuenta de Apple.
