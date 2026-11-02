# Proposal

## Why

El build de macOS de `v0.2.0-beta.4` falló al enlazar: el ONNX Runtime precompilado que trae `ort` (búsqueda por contenido, change `semantic-search`) está hecho para macOS 13.4 y pide `___isPlatformVersionAtLeast`, que no se enlaza con un mínimo de macOS 12. Lumik promete correr en macOS 12 Monterey (spec `desktop-distribution`), y la beta no tiene paquete de macOS mientras esto no se resuelva. El de Linux falló por lo mismo: ese ONNX Runtime se compiló con glibc 2.38 y GCC 13, y el job compila en Ubuntu 22.04 (glibc 2.35) a propósito, para que Lumik corra en Debian 12 y Ubuntu 22.04.

El change se llama `ort-dynamic-macos` porque empezó por macOS; cubre también Linux.

## What Changes

- **En macOS y en Linux, ONNX Runtime deja de enlazarse en la app** y se carga en tiempo de ejecución (`ort` con `load-dynamic`), solo cuando se usa la búsqueda por contenido. La app ya no depende de él para compilar ni para abrir, así que vuelve a construirse y a correr en macOS 12 y en las distros con glibc 2.35.
- **El motor se descarga junto con el modelo.** Al descargar el modelo de búsqueda por contenido en una Mac con Apple Silicon o en Linux, Lumik baja también el motor de ONNX Runtime (la librería oficial de Microsoft) desde los releases del repo, lo verifica con sha256 y lo instala junto al modelo. Borrar el modelo lo borra también. En Linux la búsqueda por contenido sigue disponible en todas las distros soportadas: el motor oficial pide glibc 2.28.
- **La búsqueda por contenido en Mac requiere Apple Silicon y macOS 14 o posterior.** ONNX Runtime ya no se publica para Mac Intel (ni Microsoft ni los binarios que usa `ort`), y el oficial de Apple Silicon requiere macOS 14. En una Mac Intel o con macOS 12 o 13, Ajustes muestra qué falta en lugar de ofrecer la descarga, y el resto de Lumik funciona igual.
- **Windows no cambia:** sigue enlazando ONNX Runtime en la app (su build pasó), sin descarga aparte.
- **Fuera de alcance:** compilar ONNX Runtime para Mac Intel o para macOS 12; la tablet, que no tiene búsqueda por contenido; y la clave de subida de Android (el otro job que falló en la beta 4).

## Capabilities

### New Capabilities

(ninguna)

### Modified Capabilities

- `desktop-distribution`: en macOS y Linux, la búsqueda por contenido usa un motor que se descarga con el modelo; en macOS requiere Apple Silicon y macOS 14, y la app sigue corriendo en Mac Intel y en macOS 12.

## Impact

- **`crates/lumik-core`:**
  - `Cargo.toml`: `ort` con `load-dynamic` en macOS y Linux;
  - `embedding.rs`: en macOS y Linux, inicializar `ort` con la ruta de la librería instalada;
  - `domain/embedding.rs`: los paquetes del motor de macOS y de Linux (URL, tamaño y sha256).
- **`apps/desktop`:**
  - `infrastructure/content_model.rs`: descargar, verificar, instalar y borrar el motor junto con el modelo en macOS;
  - el estado del modelo con "requiere Apple Silicon" o "requiere macOS 14" y su texto en Ajustes (`es.json` y `en.json`).
- **Releases del repo:** `ort-runtime-macos-1.31.0` y `ort-runtime-linux-1.31.0`, con la librería de cada uno, como el release del modelo.
- **CI:** los jobs `build-macos` y `build-linux` de `release.yml` vuelven a compilar sin cambios en el workflow.
- **Desarrollo en Linux:** `pnpm tauri dev` también baja el motor al descargar el modelo.
- **Relación:** depende del change `semantic-search`, que sigue abierto y aporta el modelo, su descarga y Ajustes.
