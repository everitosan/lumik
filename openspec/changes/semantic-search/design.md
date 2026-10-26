# Design

## Context

- El índice de búsqueda (`photo-search`) vive en `lumik.db`: `search_photo` (filas chicas que filtra el SQL) y `search_media` (miniatura de 128 px y una columna `embedding` que hoy siempre está en NULL). `IndexProject` lo rehace por proyecto cuando cambia su generación o `_exported/`.
- Cada foto tiene en el disco `.thumbs/{id}.jpg` (480 px, con la rotación aplicada).
- `crates/lumik-core` no depende de Tauri ni de `cfg(target_os)` y lo va a heredar la app de tablet (Expo + uniffi), así que la inferencia tiene que compilar para Linux, Windows, macOS, Android e iOS.
- Escribir en `project.db` requiere una sesión de escritura (`ProjectSessions`, candado `.lumik-lock`).
- Ver proposal.md para la motivación.

## Goals / Non-Goals

**Goals:**
- Buscar por contenido con texto en español o inglés, sin red y sin cambiar nada para quien no descarga el modelo.
- Calcular cada vector una sola vez por foto y modelo, sin importar en qué equipo se calcule.

**Non-Goals:**
- Autotags (change aparte, cuando la búsqueda esté validada con fotos reales).
- Elegir entre varios modelos o tener dos instalados a la vez.
- Buscar con una foto de ejemplo ("más como esta"), aunque los vectores lo permitirán.
- Un índice vectorial aproximado (HNSW o similar): con decenas de miles de fotos basta recorrerlas todas.

## Decisions

### D1. Modelo: CLIP ViT-B/32 con codificador de texto multilingüe

- Imagen: el codificador de CLIP ViT-B/32 (OpenAI, MIT). Texto: `sentence-transformers/clip-ViT-B-32-multilingual-v1` (Apache-2.0), un codificador destilado para que su vector caiga en el mismo espacio que el de la imagen en más de 50 idiomas, español entre ellos. Vectores de 512 dimensiones.
- Los dos se exportan a ONNX y se cuantizan a int8: unos 220 MB en total.
- Alternativa: SigLIP 2 base multilingüe, de mejor calidad pero con unos 400 MB y su tokenizer de 256 000 tokens. Queda como reemplazo si la calidad no alcanza: cada vector guarda su modelo (D3), así que cambiar de modelo marca todo como pendiente sin mezclar espacios.

### D2. Paquete del modelo y descarga

- Un zip `lumik-model-clip-multilingual-v1.zip` con `image.onnx`, `text.onnx`, `tokenizer.json`, un `manifest.json` (id del modelo, dimensión, normalización de la imagen) y las licencias. Se publica como asset de un release de GitHub con tag `model-clip-multilingual-v1`. El workflow de la app solo se dispara con tags `v*`, así que este release no lo dispara.
- Un script en `tools/models/` (Python, solo para quien publica) convierte, cuantiza y empaqueta, y escribe el sha256. Lo que se publica sale de ese script.
- La app trae escritos la URL, el tamaño y el sha256. Descarga a un archivo temporal en la carpeta de datos (`{app_data}/models/`), por bloques, con progreso y cancelación, verifica el hash, descomprime en `{app_data}/models/clip-multilingual-v1/` y solo entonces lo da por instalado. Si el hash no coincide, borra el archivo.
- Borrar el modelo borra esa carpeta. Los vectores no se tocan.

### D3. Vectores en `project.db`

```
photo_embedding(photo_id PK REFERENCES photo ON DELETE CASCADE,
                model TEXT, rotation INTEGER, vector BLOB)
project_settings.analysis_mode TEXT NOT NULL DEFAULT 'on_demand'
                               CHECK(analysis_mode IN ('on_import', 'on_demand'))
```

- `vector`: 512 `f32` little-endian, normalizado (2 KB). `rotation` es la de la foto cuando se analizó: si cambia, la foto vuelve a estar pendiente, como su miniatura. Un `model` distinto al instalado también cuenta como pendiente.
- Escribir vectores sube la generación, lo que dispara el reindexado normal y los copia al índice.
- Alternativa descartada: guardarlos solo en `lumik.db`. Cada equipo, y sobre todo la tablet, tendría que recalcular lo que otro ya calculó.
- Proyectos existentes: la migración agrega la tabla y deja `analysis_mode` en `on_demand`. El modo de los proyectos nuevos se lee de un ajuste global (`search.analysis_mode`, por defecto `on_demand`).

### D4. Copia al índice

`IndexProject` lee `photo_embedding` y llena `search_media.embedding` y una columna nueva `search_media.embedding_model`. Lo hace aunque el equipo no tenga el modelo: así, al descargarlo, la búsqueda funciona de inmediato. La migración del índice agrega `embedding_model` y fuerza un reindexado (`indexed_generation = -1`).

### D5. Inferencia en el núcleo

- Un puerto `EmbeddingModel` en `application/ports.rs` con `embed_image(&Path) -> Vec<f32>` y `embed_text(&str) -> Vec<f32>`. Lo implementa un adaptador en el núcleo, sin `cfg(target_os)`.
- Motor: se decide en la tarea 1.1 entre `ort` (ONNX Runtime, el más rápido y probado; trae una biblioteca nativa por plataforma) y `tract` (Rust puro, compila igual en todas las plataformas, más lento). Criterio: que compile para Android (aarch64) y que analizar una foto de 480 px en el escritorio tome menos de 100 ms. Si los dos cumplen, gana `tract` por simplicidad de compilación. `candle` no carga ONNX, así que obligaría a reescribir el modelo.
- Tokenizer: `tokenizers` de Hugging Face con el `tokenizer.json` del paquete.
- Preprocesado de la imagen: decodifica `.thumbs/{id}.jpg`, la redimensiona a 224 px por el lado corto con recorte central y normaliza con la media y desviación de CLIP (del `manifest.json`).

### D6. Analizar fotos

- Caso de uso `AnalyzeProject`: toma una sesión de escritura del proyecto (si está bloqueado, no hace nada), busca las fotos pendientes (sin vector, de otro modelo o rotadas), calcula sus vectores de uno en uno y los escribe en lotes de 32 en una transacción. Revisa una bandera de cancelación entre fotos.
- Una cola de análisis en el desktop, con un hilo, como `IndexQueue`: un proyecto a la vez, en el orden en que se pidió. La alimentan "Analizar fotos", el final de una importación (si el proyecto está en `on_import` y el modelo instalado) y "Analizar" desde el aviso de los resultados.
- Progreso con el evento `analysis-progress` (proyecto, hechas, total) y una tarjeta en la sección de actividad de la barra lateral, con "Detener". Expulsar el disco cancela su análisis.

### D7. Búsqueda

- Si el modelo está instalado y la consulta tiene texto libre, `search_photos` calcula el vector del texto (unos milisegundos) y, además de lo de siempre, recorre los vectores de `search_media` del modelo instalado que pasan los operadores (el SQL de siempre sin la condición de texto), calcula el producto punto y se queda con los que superan un umbral, hasta 300.
- Resultado: primero las fotos que coinciden por palabras y luego las que se parecen, de más a menos parecidas. Los grupos siguen el orden de su primera foto.
- El umbral es una constante del núcleo, que se ajusta con fotos reales en la prueba manual (tarea 6.2).
- Costo: recorrer 70 000 vectores de 512 dimensiones son unas 36 millones de multiplicaciones, menos de 30 ms. Se leen solo `photo_id, embedding` de las filas que pasan los filtros.
- Los resultados indican cuántas fotos de los proyectos buscados faltan por analizar (sin vector del modelo instalado), para el aviso.

### D8. Interfaz

- Ajustes → Búsqueda: estado del modelo (no instalado con su tamaño y "Descargar", descargando con progreso y "Cancelar", instalado con "Borrar") y el modo de análisis para proyectos nuevos, desactivado sin modelo.
- Proyecto: en su menú, el modo de análisis y "Analizar fotos (N pendientes)". Sin modelo, no aparecen.
- Resultados: el aviso "N fotos sin analizar" con "Analizar", solo en búsquedas con texto libre y modelo instalado.
- About: rawler, exiftool, Tauri, React y rusqlite, más el motor elegido, `tokenizers`, CLIP ViT-B/32 (MIT) y clip-ViT-B-32-multilingual-v1 (Apache-2.0), estos dos con la nota "si se descarga".

## Risks / Trade-offs

- [La calidad en español puede quedar por debajo de la del inglés] → Se prueba con fotos reales en los dos idiomas (6.2). Si no alcanza, el reemplazo es SigLIP 2 (D1), sin cambiar el resto.
- [Analizar es lento en Android y tablets] → Es opcional y bajo demanda por defecto. Los vectores calculados en el escritorio viajan en el disco.
- [El umbral fijo deja pasar fotos que no tienen que ver o se pierde algunas que sí] → Se ajusta en la prueba y los resultados van ordenados por parecido, así que lo dudoso queda al final.
- [`project.db` crece unos 2 MB por cada 1 000 fotos] → Aceptado. `f16` lo reduciría a la mitad si molesta.
- [Un release de GitHub puede borrarse por error] → La app verifica el hash, y el script reproduce el paquete.

## Migration Plan

- `project.db`: tabla `photo_embedding` y columna `analysis_mode`, idempotentes. Una versión anterior de Lumik las ignora.
- `lumik.db`: columna `search_media.embedding_model` y un reindexado completo, que reutiliza las miniaturas.
- Sin modelo no cambia nada para el usuario.
