# Design

## Context

- Cada proyecto guarda sus fotos en su `project.db` del disco: estrellas, color, tags (texto separado por comas, en minúsculas, `normalize_tags`), aprobada, exportada, par, fecha de captura, cámara, lente. Hoy solo se filtran dentro del proyecto abierto (`photo-filtering`).
- La base global (`lumik.db`) es del equipo y ya registra los discos vistos (`device`).
- `project.generation` (de `project-lock`) sube con cada escritura del proyecto.
- El núcleo vive en `crates/lumik-core`, compartido con la futura app de tablet.
- Ver proposal.md para la motivación.

## Goals / Non-Goals

**Goals:**
- Buscar en todos los proyectos del equipo con operadores y texto libre, en menos de 200 ms para 70 000 fotos.
- Resultados útiles sin el disco conectado.
- Dejar listo el lugar del vector de cada foto para `semantic-search`.

**Non-Goals:**
- Búsqueda semántica, modelo y autotags (`semantic-search`).
- Albums.
- Compartir el índice entre equipos o con el portal web.
- Escribir tags en los archivos (XMP `dc:subject`).

## Decisions

### D1. El índice vive en `lumik.db` y es desechable

Tablas nuevas en la base global:

```
search_project(project_id PK, name, description, session_date, archived,
               device_uuid, device_name, indexed_generation, indexed_at)
search_photo(photo_id PK, project_id FK, file_name, capture_date, stars,
             colors, tags, culled, delivered, is_pair, camera, lens, *_norm)
search_media(photo_id PK FK, rotation, thumb BLOB, embedding BLOB NULL)
```

La miniatura y el vector van en `search_media`, aparte. Medido con 72 000 fotos: con la miniatura en la misma fila, cada búsqueda recorre ~300 MB de páginas (`estrellas>=3` tarda 80 ms y un texto libre amplio 186 ms); separada, 11 ms y 96 ms. Con el vector de `semantic-search` (~2 KB más por foto) la diferencia crecería. Una base con el esquema anterior se borra y se reconstruye al abrirla, porque el índice es desechable.

- `device_uuid` y `device_name` son del último disco donde se vio el proyecto, solo para el aviso de "conecta…". La identidad sigue siendo `project.id`.
- `embedding` (en `search_media`) queda vacío en este change.
- Borrar las dos tablas no pierde nada: se reconstruyen al conectar los discos.

Se descartó guardar el índice en cada disco, porque buscar exigiría tenerlos todos conectados.

### D2. Refresco por generación

- **Al descubrir proyectos** (arranque y cada escaneo de discos): por cada proyecto abierto, si `project.generation` difiere de `indexed_generation`, se reindexa ese proyecto completo. Es una lectura SQL del `project.db` y una escritura en una transacción.
- **Tras cada escritura:** al cerrar una `WriteSession` se marca el proyecto para reindexar en segundo plano, con debounce, para no reindexar en cada estrella.
- **Proyectos eliminados:** salen del índice.
- **Proyectos de discos desconectados:** se conservan.

Así no hay que tocar cada caso de uso, y el índice nunca queda más de unos segundos atrás de lo que el equipo escribió. Lo que otro equipo cambió en el disco se recoge al conectarlo, porque su generación es distinta.

### D3. Miniatura del índice

Una miniatura de 128 px en el lado mayor, JPEG de calidad 70 (unos 3–5 KB), generada a partir del thumb de 320 px que ya existe en `.thumbs/`. Para 70 000 fotos son unos 300 MB en `lumik.db`.

Se descartó no guardar miniatura: los resultados sin disco serían solo nombres de archivo, que no sirven para elegir fotos. Si el tamaño molesta, se baja a 96 px sin cambiar nada más.

### D4. Consulta: operadores, texto libre y SQL

`domain::search` interpreta la consulta en una estructura (`SearchQuery`) y la convierte de vuelta a texto. Así el cuadro y el panel son dos vistas de lo mismo, y se puede probar sin base de datos.

- Los operadores se traducen a condiciones SQL sobre `search_photo` unida a `search_project`.
- El texto libre se separa en palabras. Cada una debe aparecer en tags, nombre de archivo o nombre o descripción del proyecto. Se compara sin mayúsculas ni acentos, con columnas normalizadas al indexar.
- El orden es por fecha de captura descendente, y los resultados se agrupan por proyecto en el frontend.
- Un operador mal escrito (`estrellas>=x`) se trata como texto libre.
- Nombres de color: rojo, amarillo, verde, azul y púrpura (1 a 5, los del filtro por color), también en inglés.
- Los operadores se leen en español y en inglés; el panel los escribe en el idioma de la app.

Con `semantic-search`, el texto libre pasará a ordenar por parecido entre las candidatas que dejen los operadores, sin cambiar esta interfaz.

### D5. Interfaz

- Un solo buscador, el del dashboard: al escribir filtra proyectos por nombre como antes; Enter o el panel avanzado cambian el grid de proyectos por los resultados de fotos, con "Volver a proyectos". Mientras el texto parece llevar operadores (`palabra:` o `palabra>=`) no se filtran proyectos; cuáles son válidos lo decide el núcleo al buscar. Se descartó una sección "Buscar" aparte: dos buscadores confundían.
- La búsqueda en curso vive en `App` (`SearchSnapshot`), así que abrir una foto y cerrar el visor vuelve al dashboard con los mismos resultados y la misma posición.
- La ida y vuelta entre el cuadro y el panel la hace el núcleo (`parse_search_query` / `format_search_query`), para no duplicar en TypeScript la interpretación de la consulta.
- Los resultados llegan sin miniatura; cada tarjeta visible la pide aparte (`get_search_thumbnail`), como el grid del proyecto.
- El panel avanzado se despliega desde el cuadro, como en Gmail.
- Resultados con virtualización, como el grid del proyecto.
- Abrir un resultado con disco conectado navega al visor de su proyecto; sin disco, muestra la miniatura y el aviso.

## Risks / Trade-offs

- [`lumik.db` crece varios cientos de MB con bibliotecas grandes] → Miniatura de 3–5 KB; se puede bajar el tamaño o quitar el índice de proyectos que no se ven hace mucho, más adelante.
- [Primera indexación de un disco con decenas de miles de fotos] → Corre en segundo plano, proyecto por proyecto, solo leyendo `project.db` y los thumbs. Se ve el avance y se reanuda en el siguiente escaneo si se desconecta el disco.
- [La miniatura del índice queda vieja tras rotar] → Rotar sube la generación y el reindexado regenera la miniatura.
- [Texto libre sin semántica encuentra menos de lo esperado] → Es el paso intermedio declarado. El cuadro se queda igual cuando llegue `semantic-search`.

## Migration Plan

Las tablas se crean con `CREATE TABLE IF NOT EXISTS` en la base global. El primer arranque indexa los proyectos abiertos en segundo plano. No hay cambios en `project.db`.
