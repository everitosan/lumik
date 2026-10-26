# Proposal

## Why

La búsqueda de hoy solo encuentra lo que el fotógrafo escribió: tags, nombres de archivo y datos del proyecto. Para dar con "la novia en la playa" o "retratos de noche" en años de sesiones sin etiquetar hace falta buscar por lo que hay en la foto, en español o en inglés y sin mandar fotos fuera del equipo.

## What Changes

- Búsqueda por contenido: el texto libre también encuentra fotos cuyo contenido se parece a lo escrito, en español o inglés, con un modelo de imagen y texto que corre en el equipo. Nada sale del equipo.
- Función opcional: Lumik se instala igual que hoy. En Ajustes → IA se descarga el modelo (unos 235 MB, verificado); hasta entonces no hay búsqueda por contenido ni análisis de fotos. Se puede borrar y vuelve a como estaba.
- Vectores de las fotos guardados en el disco, en el `project.db`, con el modelo que los generó: se calculan una vez y viajan con el proyecto a cualquier equipo o tablet. El índice de búsqueda los copia.
- Cada proyecto elige cuándo se analizan sus fotos: al importar o bajo demanda ("Analizar fotos"). Los proyectos nuevos toman el modo que diga Ajustes.
- El modelo se publica como asset de un release propio en GitHub, ya convertido y cuantizado, con sus licencias.
- About lista el motor de inferencia, el tokenizer y los dos modelos con sus licencias.
- Fuera de alcance: autotags (etiquetas sugeridas por contenido), elegir entre varios modelos, buscar con una foto de ejemplo y promocionarlo en Android.

## Capabilities

### New Capabilities
- `semantic-search`: búsqueda por contenido de las fotos: el modelo opcional (descarga, verificación, borrado), el análisis de las fotos de un proyecto (al importar o bajo demanda) y la búsqueda con texto en español o inglés.

### Modified Capabilities
- `photo-search`: el texto libre también busca por contenido cuando el modelo está instalado, y los resultados se ordenan por parecido en ese caso.
- `app-settings`: Ajustes gana la pestaña IA, que agrupa las funciones con modelos en el equipo (hoy, la búsqueda por contenido).

## Impact

- Núcleo (`crates/lumik-core`): motor de inferencia y tokenizer en Rust (dependencias nuevas), tabla de vectores en `project.db` y modo de análisis en `project_settings`, `search_media.embedding` con su modelo en `lumik.db`, caso de uso para analizar fotos y similitud en la búsqueda. Lo hereda la app de tablet.
- Desktop: descarga y gestión del modelo, pestaña Ajustes → IA, "Analizar fotos" y modo en el proyecto, análisis tras importar, avisos de progreso, About, textos `es`/`en`.
- Repo: script para convertir y cuantizar el modelo y un release `model-clip-multilingual-v1` (tag que no empieza con `v`, no dispara el workflow de la app).
- Tamaño: unos 2 KB por foto en `project.db` y en `lumik.db`.
