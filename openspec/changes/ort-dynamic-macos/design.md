# Design

## Context

- `crates/lumik-core`, con la feature `embeddings`, usa `ort = "=2.0.0-rc.15"`, que descarga y enlaza en la app un ONNX Runtime precompilado. `ClipModel::load` (`embedding.rs`) crea el entorno con `ort::environment::init()` y las sesiones desde el directorio del modelo.
- El desktop activa `embeddings` en todas las plataformas. El modelo (`CLIP_MULTILINGUAL` en `domain/embedding.rs`: URL de un release del repo, tamaño y sha256) se descarga bajo demanda con `ContentModel` (`infrastructure/content_model.rs`), que lo verifica, lo instala en `{datos}/models/{id}/` y expone `ModelState` (`NotInstalled`, `Downloading`, `Installed`).
- macOS se construye como `universal-apple-darwin` en `release.yml` (runner `macos-14`) con `minimumSystemVersion: 12.0`. El ONNX Runtime que trae `ort` está compilado para macOS 13.4 y su código de CoreML referencia `___isPlatformVersionAtLeast`, que no queda enlazado. Por eso falló el build de la beta 4.
- El job `build-linux` corre en Ubuntu 22.04 (glibc 2.35) para que Lumik abra en Debian 12 y Ubuntu 22.04. El ONNX Runtime estático que trae `ort` pide glibc 2.38 (`__isoc23_strtoll`) y la `libstdc++` de GCC 13, así que tampoco enlaza ahí. Windows sí compila.
- `ort` con `load-dynamic` no enlaza ONNX Runtime: lo abre con `dlopen` en `ort::environment::init_from(ruta)`.

## Goals / Non-Goals

**Goals:**
- Que el `.dmg` vuelva a compilarse y que la app abra en macOS 12.
- Búsqueda por contenido en Mac con Apple Silicon y macOS 14 o posterior, con el motor descargado y verificado como el modelo.
- Que el build de Linux vuelva a compilar en Ubuntu 22.04, sin subir el glibc que pide Lumik.
- No tocar Windows.

**Non-Goals:**
- Búsqueda por contenido en Mac Intel o en macOS 12 o 13 (habría que compilar ONNX Runtime nosotros).
- Cambiar el modelo, sus vectores o la UI de la búsqueda.

## Decisions

### D1. `load-dynamic` en macOS y Linux

En `crates/lumik-core/Cargo.toml`, `ort` queda como hoy para todas las plataformas, y además, en `[target.'cfg(any(target_os = "macos", target_os = "linux"))'.dependencies]`, `ort` con `features = ["load-dynamic"]` (misma versión, `optional = true`). Cargo solo une esas features al compilar para esas plataformas: ahí no se descarga ni se enlaza el ONNX Runtime precompilado, y desaparecen los errores de enlace de los dos jobs. Windows sigue con el estático.

En `ClipModel::load`, en macOS y Linux el entorno se crea con `ort::environment::init_from(<motor instalado>)`, y en Windows sigue `init()`. La ruta del motor llega como parámetro de `load` (`Option<&Path>`), así `lumik-core` no conoce dónde se instala.

**Alternativas descartadas:**
- `-fapple-link-rtlib`: compila, pero el código enlazado sigue siendo para 13.4, y no hay forma de probar que la app abra en macOS 12.
- `load-dynamic` también en Windows: obligaría a distribuir el motor ahí sin necesidad, porque su build funciona.
- Subir el job de Linux a Ubuntu 24.04: compila, pero Lumik entero dejaría de abrir en Debian 12 y Ubuntu 22.04, no solo la búsqueda por contenido.

### D2. El motor como un paquete descargable, igual que el modelo

- **`domain/embedding.rs`** agrega `ORT_RUNTIME_MACOS: RuntimePackage` (id, URL, tamaño, sha256 y el nombre de la librería), análogo a `ModelPackage`. El zip trae `libonnxruntime.1.31.0.dylib` de la distribución oficial `onnxruntime-osx-arm64-1.31.0`: es la versión que trae `ort` rc.15 (API 31). Solo existe para Apple Silicon: desde la 1.29, ni Microsoft ni los binarios que usa `ort` publican ONNX Runtime para Mac Intel. Su versión mínima es macOS 14.0 (`LC_BUILD_VERSION`), por eso el umbral de D3 es macOS 14 y no el 13.4 del binario estático de `ort`.
- **Linux:** `ORT_RUNTIME_LINUX` con `libonnxruntime.so.1.31.0` de la distribución oficial `onnxruntime-linux-x64-1.31.0`, que pide glibc 2.28 y `GLIBCXX_3.4.22` (revisado con `objdump -T`). Solo x86_64, la arquitectura que publica el release.
- **Publicación:** releases del repo `ort-runtime-macos-1.31.0` y `ort-runtime-linux-1.31.0`, cada uno con su zip (la librería, `LICENSE` y `ThirdPartyNotices.txt`), como `model-clip-multilingual-v1`. Se suben a mano una vez (tarea), con su sha256 en el código.
- **`ContentModel`:** en macOS, `download` instala primero el motor y luego el modelo, con un solo progreso (la suma de lo que falte). El motor va como otro paquete junto al modelo, en `{datos}/models/{id del motor}/`, con la misma descarga y verificación (`ModelStore`), y lo valida que traiga su librería en lugar de un `manifest.json`. `is_installed` exige los dos; quien ya tenía el modelo solo baja el motor. `delete` borra el modelo y el motor. Un fallo en cualquiera de los dos deja el estado en `NotInstalled` con su error, como hoy.
- Al cargar el modelo, el desktop pasa la ruta del dylib instalado a `ClipModel::load`.

**Alternativa descartada:** meter la librería dentro del `.app`. Suma ~30 MB al `.dmg` para todos, aunque no usen la búsqueda por contenido, y no evita el problema en macOS 12.

### D3. Mac Intel o macOS anterior a 14: no disponible

- `ModelState` agrega `Unsupported { requires }`, con `requires` = `"apple-silicon"` o `"macos-14"`. En una Mac Intel (la parte `x86_64` del binario universal, `cfg!(target_arch = "x86_64")`) el estado es siempre `Unsupported { requires: "apple-silicon" }`: con `load-dynamic` esa parte compila sin enlazar ONNX Runtime y nunca intenta cargarlo. En Apple Silicon, el estado se calcula primero con la versión del sistema (`sysinfo::System::os_version()`, que el desktop ya tiene como dependencia; por ejemplo `"14.2"`). La comparación es una función pura `macos_supports_runtime(&str) -> bool`. Si es menor que 14, el estado es `Unsupported { requires: "macos-14" }`, no se ofrece la descarga y no se intenta cargar el motor.
- Ajustes, en la sección de búsqueda por contenido, muestra "Requiere una Mac con Apple Silicon" o "Requiere macOS 14 o posterior" en lugar del botón (`es.json`, `en.json`). El buscador ya se comporta como sin modelo instalado.

## Risks / Trade-offs

- **[Compatibilidad de la librería con `ort`]** `load-dynamic` exige que la versión de ONNX Runtime soporte la API con que se compila `ort` rc.15. → Se fija la versión exacta del dylib en D2, y una tarea comprueba en una Mac que `init_from` carga y que una búsqueda por contenido devuelve resultados.
- **[Firma y cuarentena]** Un dylib descargado por la app no pasa por la cuarentena de Gatekeeper (no lo baja un navegador) y la app no está notarizada; `dlopen` lo carga igual. → Verificar en una Mac real, porque cambiaría si la app se notariza con hardened runtime (entonces se necesitaría `com.apple.security.cs.disable-library-validation`).
- **[No se puede probar macOS 12 en CI]** El runner es `macos-14`. → La parte de macOS 12 (la app abre y la búsqueda dice "requiere macOS 14") se verifica cuando haya acceso a una Mac con Monterey; mientras, se prueba `macos_supports_runtime` con varias versiones (test unitario).
- **[Desarrollo en Linux]** Quien corre `pnpm tauri dev` en Linux ya no tiene ONNX Runtime en la app: al descargar el modelo baja también el motor, igual que un usuario. Los tests ignorados que cargan el modelo real reciben la librería por `LUMIK_ORT_LIB`.
- **[Dependencia del change `semantic-search`]** Ese change sigue abierto. → Este se implementa sobre lo que ya está en `release/0.2.0` y se archiva después de él.

## Migration Plan

1. Publicar el release `ort-runtime-macos-<versión>` con el zip y anotar su sha256.
2. Implementar D1 a D3 y lanzar `v0.2.0-beta.5`: el job `build-macos` debe pasar.
3. Quien ya tenga el modelo instalado en macOS lo verá como no instalado (falta el motor) y lo descargará de nuevo. No hay otra migración.
