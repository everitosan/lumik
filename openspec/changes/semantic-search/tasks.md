# Tasks

## 1. Modelo e inferencia (núcleo)

- [x] 1.1 Spike del motor: script en `tools/models/` que exporta los dos codificadores a ONNX, cuantiza a int8 y empaqueta el zip con `manifest.json`, licencias y sha256; probar `ort` y `tract` con ese paquete (compilación para `aarch64-linux-android` y tiempo por foto en el escritorio) y elegir según D5. Verificar con una nota en design.md con los números y el motor elegido.
- [ ] 1.2 Puerto `EmbeddingModel` y su adaptador (carga del paquete, preprocesado de la imagen, tokenizer). Verificar con tests: vectores de 512 normalizados, una foto de playa más parecida a "playa" y a "beach" que a "nieve", y error claro con un paquete incompleto.
- [ ] 1.3 Publicar el release `model-clip-multilingual-v1` con el zip y escribir en el núcleo la URL, el tamaño y el sha256. Verificar descargando el asset y comprobando el hash.

## 2. Vectores en el proyecto (núcleo)

- [ ] 2.1 Tabla `photo_embedding` y `project_settings.analysis_mode` con su migración; ajuste global `search.analysis_mode` para proyectos nuevos. Verificar con tests de esquema y de migración sobre una base sin las columnas.
- [ ] 2.2 Caso de uso `AnalyzeProject`: pendientes (sin vector, otro modelo, rotadas), lotes de 32, cancelación entre fotos, proyecto bloqueado sin escribir, generación que sube. Verificar con tests de fakes de cada caso.
- [ ] 2.3 `IndexProject` copia los vectores a `search_media.embedding` y `embedding_model`, con migración y reindexado. Verificar con un test: un proyecto con vectores llega al índice sin el modelo instalado.

## 3. Búsqueda (núcleo)

- [ ] 3.1 `search_photos` con texto libre y modelo: candidatos que pasan los operadores, producto punto, umbral, hasta 300, orden (primero por palabras y luego por parecido) y conteo de fotos sin analizar. Verificar con tests con vectores fijos y una medición con 70 000 vectores simulados (< 30 ms).

## 4. Desktop: modelo y análisis

- [ ] 4.1 Descarga del modelo: comandos de estado, descargar (progreso, cancelar, verificación del hash, descompresión) y borrar. Verificar con tests de infraestructura con un servidor local y un hash erróneo.
- [ ] 4.2 Cola de análisis con su hilo, evento `analysis-progress`, cancelación al expulsar y análisis al terminar una importación en proyectos `on_import`. Verificar con `cargo test` y en `dev`.
- [ ] 4.3 Comandos del proyecto: leer y cambiar el modo, analizar, pendientes y detener. Verificar con la compilación y en `dev`.

## 5. Desktop: interfaz

- [ ] 5.1 Pestaña Ajustes → Búsqueda (estado del modelo, descargar, cancelar, borrar, modo de los proyectos nuevos). Verificar en `dev` los escenarios de "Modelo opcional" y "Borrar el modelo".
- [ ] 5.2 En el proyecto: modo de análisis y "Analizar fotos (N pendientes)" en su menú; tarjeta de progreso en la barra lateral con "Detener". Verificar en `dev` los escenarios de "Cuándo se analizan las fotos" y "Progreso del análisis".
- [ ] 5.3 Resultados: orden por parecido y aviso "N fotos sin analizar" con "Analizar". Verificar en `dev`.
- [ ] 5.4 About con el motor, `tokenizers` y los dos modelos; textos `es`/`en`. Verificar con `tsc` y `jq empty` de los JSON.

## 6. Documentación y verificación

- [ ] 6.1 Diagrama en `docs/useCases/` (descarga del modelo, análisis, búsqueda) y comandos en `apps/desktop/CLAUDE.md`. Verificar que el Mermaid parsea.
- [ ] 6.2 Prueba en `dev` con fotos reales: buscar 10 conceptos en español y en inglés, ajustar el umbral, analizar en un equipo y buscar en otro con el mismo disco, y buscar sin red. Correr los tests del núcleo y del desktop y `openspec validate semantic-search --strict`.
