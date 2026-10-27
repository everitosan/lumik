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
- Los dos se exportan a ONNX y se cuantizan a int8 (pesos de `MatMul`, `Gemm` y `Gather`; la convolución del parche queda en float porque ONNX Runtime no tiene `ConvInteger` en CPU): 234 MB en total.
- Alternativa: SigLIP 2 base multilingüe, de mejor calidad pero con unos 400 MB y su tokenizer de 256 000 tokens. Queda como reemplazo si la calidad no alcanza: cada vector guarda su modelo (D3), así que cambiar de modelo marca todo como pendiente sin mezclar espacios.

### D2. Paquete del modelo y descarga

- Un zip `lumik-model-clip-multilingual-v1.zip` con `image.onnx`, `text.onnx`, `tokenizer.json`, un `manifest.json` (id del modelo, dimensión, normalización de la imagen) y las licencias. Se publica como asset de un release de GitHub con tag `model-clip-multilingual-v1`. El workflow de la app solo se dispara con tags `v*`, así que este release no lo dispara.
- Un script en `tools/models/` (Python, solo para quien publica) convierte, cuantiza y empaqueta, y escribe el sha256. Lo que se publica sale de ese script.
- La app trae escritos la URL, el tamaño y el sha256. Descarga a un archivo temporal en la carpeta de datos (`{app_data}/models/`), por bloques, con progreso y cancelación, verifica el hash, descomprime en `{app_data}/models/clip-multilingual-v1/` y solo entonces lo da por instalado. Si el hash no coincide, borra el archivo.
- Borrar el modelo borra esa carpeta. Los vectores no se tocan.
- En el desktop lo hace `infrastructure::content_model` (`ModelStore` instala y borra; `ContentModel` lleva el estado de la descarga y carga el modelo la primera vez que se usa, con hasta 4 hilos). La URL, el tamaño y el sha256 viven en el núcleo (`domain::embedding::CLIP_MULTILINGUAL`). Con `LUMIK_MODEL_URL` se descarga de otra URL, `https://` o `file://`, para probar un paquete armado en local antes de publicarlo.
- Comandos: `get_content_model_state`, `download_content_model` (en segundo plano), `cancel_content_model_download` y `delete_content_model`. El progreso, como mucho cada 100 ms, y cada cambio de estado llegan con el evento `content-model-state`. Los errores viajan como `código:detalle` (`network`, `corrupt`, `io`) para que la vista los traduzca.

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
- Proyectos existentes: la migración agrega la tabla y deja `analysis_mode` en `on_demand`. El modo de los proyectos nuevos se lee de un ajuste global (`AppSettings.default_analysis_mode`, por defecto `on_demand`), que `create_project` copia al proyecto como hace con `default_delivery_mode`.

### D4. Copia al índice

`IndexProject` lee `photo_embedding` y llena `search_media.embedding` y una columna nueva `search_photo.embedding_model`. Lo hace aunque el equipo no tenga el modelo: así, al descargarlo, la búsqueda funciona de inmediato. El modelo va en `search_photo` (filas chicas) para contar las fotos sin analizar sin leer blobs. La migración del índice agrega `embedding_model` y fuerza un reindexado (`indexed_generation = -1`).

### D5. Inferencia en el núcleo

- Un puerto `EmbeddingModel` en `application/ports.rs` con `id()`, `embed_image(&Path)` y `embed_text(&str)`, que devuelven vectores normalizados. Lo implementa `embedding::ClipModel` en el núcleo, sin `cfg(target_os)`, detrás de la feature `embeddings` (la activa el desktop): así el núcleo compila sin ONNX Runtime para quien no lo necesita.
- Motor: **`ort` 2.0** (ONNX Runtime 1.31, MIT / Apache-2.0), elegido en el spike (abajo) frente a `tract`. `ort` enlaza ONNX Runtime de forma estática con los binarios precompilados que descarga al compilar, así que la app no carga bibliotecas aparte. `candle` no carga ONNX, así que obligaría a reescribir el modelo.
- Tokenizer: `tokenizers` 0.22 de Hugging Face sin features por defecto y con `fancy-regex` (Rust puro, sin `onig` ni C++), con el `tokenizer.json` del paquete.

**Resultado del spike (tarea 1.1).** Linux x86_64 de 16 hilos, paquete de `tools/models/build_clip_multilingual.py`, cinco fotos de COCO (dormitorio, niños con trofeo de tenis, béisbol, calle con coches, gatos en un sillón) contra doce frases en español e inglés:

| | carga | foto (inferencia) | frase |
|---|---|---|---|
| `ort`, int8, todos los hilos | 0.6 s | 17 ms | 4 ms |
| `ort`, int8, 4 hilos | | 21 ms | 4 ms |
| `ort`, int8, 1 hilo | | 57 ms | 5 ms |
| `ort`, float32 | 1.7 s | 31 ms | 11 ms |
| `tract`, float32 (1 hilo) | 1.6 s | 142 ms | 17 ms |
| `tract`, int8 | no terminó de cargar en 10 min | | |

- Las cinco fotos aciertan su frase en los dos idiomas, con los dos motores y en float e int8. La frase en español y la inglesa quedan a menos de 0.03 de parecido entre sí. El int8 conserva el vector del float (coseno 0.984 en imagen y 0.996 en texto).
- Preprocesar la foto (decodificar el JPEG de 480 px, redimensionar y normalizar) toma 8 ms.
- `ort` compila para `aarch64-linux-android` sin cambios (NDK r26, API 24): ejecutable de 37 MB con ONNX Runtime dentro. En el escritorio suma unos 30 MB al binario.
- `tract` queda descartado: no cumple los 100 ms por foto y con el modelo int8 no carga, así que habría que publicar el float32 (890 MB).
- Falta medir en un Android real (el emulador `Pixel_Tablet` es x86_64) y comprobar los binarios de `ort` para iOS. Eso queda para las pruebas en tablets.
- Preprocesado de la imagen: decodifica `.thumbs/{id}.jpg`, la redimensiona a 224 px por el lado corto con recorte central y normaliza con la media y desviación de CLIP (del `manifest.json`).

### D6. Analizar fotos

- Caso de uso `AnalyzeProject`: corre dentro de una sesión de escritura del proyecto que abre el llamador (si está bloqueado, no se analiza; al cerrarla sube la generación), busca las fotos pendientes (sin vector, de otro modelo o rotadas), calcula sus vectores de uno en uno y los escribe en lotes de 32 en una transacción. Revisa una bandera de cancelación entre fotos.
- Usa el thumbnail de `.thumbs/`. Si falta (por ejemplo, un DNG exportado por Lightroom que solo trae `JpgFromRaw`), lo genera antes. Las fotos que aun así fallan se recuerdan en la cola mientras viva la app y no cuentan en "Analizar N fotos"; un análisis completo del proyecto vuelve a intentarlas y reemplaza la lista.
- Una cola de análisis (`application::analysis_queue::AnalysisQueue`, en el núcleo) y un hilo en el desktop (`spawn_analysis_worker`): un proyecto a la vez, en el orden en que se pidió, sin duplicados. La alimentan "Analizar fotos", el final de una importación (`finish_import`, si el proyecto está en `on_import` y el modelo instalado) y "Analizar" desde el aviso de los resultados.
- El hilo abre una sesión de escritura por proyecto. Si está en otro dispositivo termina con `locked` sin tocarlo; al cerrar la sesión sube la generación y el índice copia los vectores nuevos.
- Progreso con el evento `analysis-progress` (`AnalysisEvent`: proyecto, estado `running`/`done`/`cancelled`/`locked`/`no_model`/`failed`, hechas, total; como mucho cada 250 ms) y una tarjeta en la sección de actividad de la barra lateral, con "Detener".
- Expulsar un disco detiene el análisis de sus proyectos y espera a que suelte la base (`cancel_and_wait`, hasta 10 s) antes de expulsar. Si el disco se desconecta sin expulsarlo, la escritura falla y el proyecto termina en `failed`; lo hecho hasta el último lote queda guardado.

### D7. Búsqueda

- Si el modelo está instalado y la consulta tiene texto libre, `SearchPhotos` calcula el vector del texto tal como se escribió (`SearchQuery::free_text`, con acentos y mayúsculas, sin los operadores) y, además de lo de siempre:
  1. `nearest_photos`: recorre en memoria los vectores del modelo instalado y se queda con los que llegan al umbral, hasta 3 000 candidatos, de más a menos parecidos.
  2. `content_hits`: de esos candidatos, los que pasan los operadores (el SQL de siempre sin la condición de texto, más `photo_id IN json_each(?)`), hasta 300, sin repetir los que ya salieron por palabras.
- Los vectores se cargan en memoria (`VectorCache`, contiguos) la primera vez que se busca por contenido y se recargan cuando cambia el índice (`index_version`, que sube tras escribir o quitar un proyecto). Leerlos de `search_media` en cada búsqueda obligaría a leer también las miniaturas, que van antes en la fila.
- Resultado: primero las fotos que coinciden por palabras y luego las que se parecen, de más a menos parecidas, con su parecido (`similarity`). Los grupos siguen el orden de su primera foto.
- El umbral (`CONTENT_MIN_SIMILARITY`, 0.24) es una constante del núcleo, que se ajusta con fotos reales en la prueba manual (tarea 6.2).
- Costo medido con 70 000 vectores de 512 (release): recorrido 9 ms, filtros 9 ms, conteo de sin analizar 9 ms. Cargar los vectores tras un cambio del índice toma unos 320 ms, una vez. En memoria son 2 KB por foto (140 MB con 70 000).
- Los resultados indican, por proyecto, cuántas fotos que pasan los operadores faltan por analizar (sin vector del modelo instalado), para el aviso.

### D8. Interfaz

- Ajustes → IA (`AiSettingsSection`), pestaña pensada para agrupar las funciones con modelos que lleguen después. El título de la búsqueda por contenido va arriba; debajo, dos columnas que pasan a una en pantallas angostas (`grid` con `auto-fit`): la explicación y la nota de privacidad, y el bloque del modelo (no instalado con su tamaño y "Descargar", descargando con progreso y "Cancelar", instalado con "Borrar"). Abajo, el modo de análisis de los proyectos nuevos, que solo aparece con el modelo instalado.
- Proyecto: el modo de análisis va en "Editar proyecto", junto a la entrega al aprobar, donde ya viven los ajustes de cada proyecto. "Analizar N fotos" es un botón de la barra superior del proyecto, junto a Respaldar; mientras se analiza muestra "Analizando 120/300" y al pulsarlo se detiene, y con todo analizado no aparece. Sin modelo, no aparece ninguno de los dos.
- Barra lateral: sección "Análisis", como la de Respaldos, mientras haya proyectos en espera, en curso o terminados sin quitar. Cada tarjeta muestra el avance con "Detener", o cómo terminó (listo, detenido, abierto en otro dispositivo, error) con una × para quitarla. El backend avisa también cuando un proyecto entra a la cola (`queued`) y cuando empieza (`running`), para que aparezca de inmediato.
- Resultados: el aviso "N fotos sin analizar" con "Analizar", solo en búsquedas con texto libre y modelo instalado.
- About: rawler, exiftool, Tauri, React y rusqlite, más `ort` y ONNX Runtime (MIT), `tokenizers` (Apache-2.0), CLIP ViT-B/32 (MIT) y clip-ViT-B-32-multilingual-v1 (Apache-2.0), estos dos con la nota "si se descarga".

## Risks / Trade-offs

- [La calidad en español puede quedar por debajo de la del inglés] → Se prueba con fotos reales en los dos idiomas (6.2). Si no alcanza, el reemplazo es SigLIP 2 (D1), sin cambiar el resto.
- [Analizar es lento en Android y tablets] → Es opcional y bajo demanda por defecto. Los vectores calculados en el escritorio viajan en el disco. Con un hilo, el escritorio tarda 57 ms por foto; un teléfono puede tardar varias veces eso.
- [`ort` descarga ONNX Runtime precompilado al compilar] → El CI necesita red al compilar, como con las demás dependencias. Si algún día molesta, `ort` acepta una ruta local a la biblioteca.
- [El umbral fijo deja pasar fotos que no tienen que ver o se pierde algunas que sí] → Se ajusta en la prueba y los resultados van ordenados por parecido, así que lo dudoso queda al final.
- [`project.db` crece unos 2 MB por cada 1 000 fotos, y la caché en memoria igual] → Aceptado. `f16` lo reduciría a la mitad si molesta.
- [Tras cada cambio del índice (calificar una foto, por ejemplo) la siguiente búsqueda por contenido recarga todos los vectores] → 320 ms con 70 000 fotos, solo en esa búsqueda. Si molesta, la recarga puede hacerse por proyecto.
- [Un release de GitHub puede borrarse por error] → La app verifica el hash, y el script reproduce el paquete.

## Migration Plan

- `project.db`: tabla `photo_embedding` y columna `analysis_mode`, idempotentes. Una versión anterior de Lumik las ignora.
- `lumik.db`: columna `search_media.embedding_model` y un reindexado completo, que reutiliza las miniaturas.
- Sin modelo no cambia nada para el usuario.
