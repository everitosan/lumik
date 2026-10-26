# Flujo: Búsqueda de fotos

Lumik Desktop · Linux · Windows · macOS · Android (y la futura app de tablet, vía `lumik-core`)

La búsqueda cruza todos los proyectos que ha visto el equipo, aunque su disco no esté conectado.
Para eso cada equipo guarda un **índice** en su base global (`lumik.db`): una copia de las fotos de
cada proyecto (tags, estrellas, color, fechas, cámara, lente y estado) y, en una tabla aparte
(`search_media`) para que las búsquedas recorran filas chicas, una miniatura de 128 px y el vector
de la foto.
La fuente de verdad sigue siendo cada `project.db`; el índice se puede borrar y se reconstruye al
conectar los discos.

## Vista general

```mermaid
flowchart LR
    A["Disco conectado<br/>o escritura en un proyecto"]
    B["IndexQueue<br/>ya / 2 s tras escribir"]
    C["IndexProject<br/>si cambió la generación"]
    D["lumik.db<br/>search_project + search_photo<br/>+ search_media"]
    E["Cuadro o panel<br/>operadores + texto libre"]
    F["SearchPhotos<br/>SQL + conectado o no"]
    G["Resultados por proyecto<br/>abrir o 'conecta…'"]

    A --> B --> C --> D
    E --> F --> D
    F --> G

    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    class E,G ui
    class B,C,F be
    class A,D db
```

## 1. Mantener el índice al día

```mermaid
sequenceDiagram
    autonumber
    participant SC as scan_connected_devices / arranque
    participant WS as WriteSession (drop)
    participant Q as IndexQueue
    participant W as hilo search-index
    participant P as project.db (disco)
    participant G as lumik.db (equipo)

    SC->>Q: request(cada proyecto abierto, ya)
    WS->>Q: request(proyecto, 2 s)  — otra escritura reinicia la espera
    W->>Q: next_due()
    Q-->>W: proyectos listos
    W->>P: generation
    W->>G: indexed_generation
    alt igual
        W-->>W: nada que hacer
    else distinta
        W->>P: proyecto + fotos fuera de la papelera
        W->>G: miniatura y vector ya indexados
        W->>W: miniatura nueva solo si no hay o la foto rotó
        W->>G: reemplazar proyecto y fotos (una transacción)
    end
```

Lo que otro equipo cambió en el disco se recoge al conectarlo, porque su generación es distinta.
Eliminar un proyecto lo quita del índice; desconectar su disco no.

## 2. Buscar

```mermaid
flowchart TB
    A["'tag:calle -tag:boda estrellas>=4 en:2025 ninos'"]
    B["SearchQuery::parse<br/>(operadores en español o inglés;<br/>lo inválido es texto libre)"]
    C["where_clause<br/>tags_norm LIKE ',calle,'<br/>stars >= 4, capture_date en 2025<br/>texto: foto o proyecto contiene 'ninos'"]
    D["ORDER BY capture_date DESC<br/>LIMIT 5000"]
    E["connected = el proyecto está abierto"]
    F["grupos por proyecto<br/>miniatura pedida por tarjeta visible"]

    A --> B --> C --> D --> E --> F

    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    class A,F ui
    class B,C,D,E be
```

La búsqueda vive en el buscador del dashboard (`SearchBox`): al escribir filtra proyectos por nombre;
Enter o el panel avanzado cambian el grid por los resultados de fotos (`PhotoSearchResults`), con
"Volver a proyectos". Mientras el texto parece llevar operadores no se filtran proyectos.

El panel de búsqueda avanzada no tiene lógica propia: al abrirse pide `parse_search_query` del
cuadro, y al buscar pide `format_search_query` para escribir los operadores en el idioma de la app.

Abrir un resultado con su disco conectado muestra el visor encima de los resultados
(`CollectionViewer`): las flechas recorren los resultados con disco conectado, aunque crucen
proyectos, y Esc vuelve a los resultados en la misma posición. Sin disco, se muestra la miniatura y
qué disco conectar.

El texto libre busca hoy por coincidencia. Con `semantic-search` pasará a ordenar por parecido entre
las candidatas que dejen los operadores, usando la columna `embedding` que ya existe.

## Código

| Pieza | Archivo |
|-------|---------|
| Consulta (operadores, ida y vuelta) | `crates/lumik-core/src/domain/search.rs` |
| Tablas del índice | `crates/lumik-core/src/db/global_schema.sql` |
| Escritura y consulta del índice | `crates/lumik-core/src/db/search_index.rs` |
| Indexar un proyecto | `crates/lumik-core/src/application/use_cases/index_project.rs` |
| Buscar | `crates/lumik-core/src/application/use_cases/search_photos.rs` |
| Cola de reindexado | `crates/lumik-core/src/application/index_queue.rs` |
| Hilo, pedidos al escanear y tras escribir | `src-tauri/src/commands.rs` |
| Miniatura de 128 px | `src-tauri/src/infrastructure/imaging.rs` (`index_thumbnail`) |
| Comandos | `src-tauri/src/commands/search.rs` |
| Buscador, resultados y panel | `src/pages/Search/SearchBox.tsx`, `PhotoSearchResults.tsx`, `AdvancedSearchPanel.tsx` |
