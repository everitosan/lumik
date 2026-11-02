# Tasks

## 1. El motor como paquete

- [x] 1.1 Tomar `libonnxruntime.1.31.0.dylib` de la distribución oficial `onnxruntime-osx-arm64-1.31.0` (la versión de `ort` 2.0.0-rc.15, API 31), comprobar que es `arm64` y su versión mínima de macOS, empacarla en un zip y publicarla en un release del repo `ort-runtime-macos-<versión>` (con permiso del usuario antes de publicar). Verificar descargando el zip del release y comparando su sha256.
- [x] 1.3 Lo mismo para Linux: `libonnxruntime.so.1.31.0` de `onnxruntime-linux-x64-1.31.0` (comprobar con `objdump -T` que pide glibc 2.28 o menos), en un zip con su licencia, publicado en el release `ort-runtime-linux-1.31.0` (con permiso del usuario), y `ORT_RUNTIME_LINUX` en `domain/embedding.rs`. Verificar como la 1.1 y con `cargo test -p lumik-core --features embeddings`.
- [x] 1.2 `ORT_RUNTIME_MACOS: RuntimePackage` en `crates/lumik-core/src/domain/embedding.rs` (id, URL, tamaño, sha256 y nombre de la librería). Verificar con `cargo test -p lumik-core --features embeddings`.

## 2. `ort` dinámico en macOS

- [ ] 2.1 `crates/lumik-core/Cargo.toml`: `ort` con `load-dynamic` en macOS y Linux (D1), y `ClipModel::load` con la ruta opcional del motor (`init_from` en macOS y Linux, `init` en Windows). Verificar con `cargo test -p lumik-core --features embeddings` y con los jobs `build-macos` y `build-linux` (2.3).
- [x] 2.2 `ContentModel` en el desktop (D2): en macOS, descarga, verificación e instalación del motor como paquete junto al modelo, antes que él,, con un solo progreso; `is_installed` exige los dos; borrar el modelo borra el motor. Pasar la ruta del dylib a `ClipModel::load`. Tests de `content_model.rs` con un paquete `file://` falso para el motor: instalar, sha256 incorrecto y borrar. Verificar con `cargo test` en `src-tauri`.
- [ ] 2.3 Comprobar que el `.dmg` universal y los paquetes de Linux compilan: lanzar el workflow con `v0.2.0-beta.5` (con permiso del usuario), y revisar que `build-macos` pasa sin `___isPlatformVersionAtLeast` y `build-linux` sin `__isoc23_strtoll`.

## 3. Mac Intel o macOS anterior a 14

- [x] 3.1 `ModelState::Unsupported { requires }` (D3): `"apple-silicon"` en la parte `x86_64` del binario de macOS y `"macos-14"` con `macos_supports_runtime` y `sysinfo::System::os_version()` en Apple Silicon. Test de `macos_supports_runtime` con `"12.7"`, `"13.6.1"`, `"14.0"`, `"14.2"` y `"15.0"`. Verificar con `cargo test` en `src-tauri`.
- [x] 3.2 Ajustes: con `Unsupported`, "Requiere una Mac con Apple Silicon" o "Requiere macOS 14 o posterior" en lugar del botón de descarga (`es.json`, `en.json`, `lib/types.ts`). Verificar con `pnpm --filter @lumik/desktop build` y `jq empty` en los dos JSON.

## 4. Integración

- [ ] 4.1 En una Mac con Apple Silicon y macOS 14 o posterior, con el `.dmg` de la beta: descargar el modelo (baja también el motor), buscar por contenido y obtener resultados, y borrar el modelo (se borra el motor). Si hay una Mac Intel o con macOS 12: la app abre y Ajustes dice qué falta. En Linux (Ubuntu 22.04 o Debian 12, con el `.deb` o el `.AppImage` de la beta): descargar el modelo (baja también el motor) y buscar por contenido. Correr `cargo test`, `pnpm --filter @lumik/desktop build` y `openspec validate ort-dynamic-macos --strict`.

## Workflow follow-up

- Archivar después de `semantic-search` y de probar en una Mac.
