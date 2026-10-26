# Flujo: Álbumes

Lumik Desktop · Linux · Windows · macOS · Android (y la futura app de tablet, vía `lumik-core`)

Un álbum junta fotos de varios proyectos y discos para curarlas juntas (una convocatoria, un
portafolio). Vive en la base del equipo (`lumik.db`), no en los discos, y sus elementos se
referencian por id: una **foto** (`photo.id`) o un **exportado** de `_exported/`
(`uuid5(project_id/nombre)`, el mismo del respaldo y del portal). Cada elemento muestra su mejor
versión, que se resuelve al leer el álbum contra el índice de búsqueda: un álbum armado con
originales pasa solo a mostrar las ediciones cuando se exportan.

## Vista general

```mermaid
flowchart LR
    A["Visor (tecla A),<br/>menú de una foto<br/>o 'Agregar todas'"]
    B["AddToAlbumDialog<br/>elegir o crear álbum"]
    C["album_item<br/>(kind, ref_id + nombres copiados)"]
    D["ReadAlbum<br/>mejor versión + discos"]
    E["Detalle del álbum<br/>final / sin editar / ya no está"]
    F["abrir en el visor<br/>o mostrar en carpeta"]

    A --> B --> C --> D --> E --> F

    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    class A,B,E,F ui
    class D be
    class C db
```

## 1. Mejor versión de cada elemento

```mermaid
flowchart TB
    I{"kind del elemento"}
    P{"¿hay un exportado<br/>ligado a la foto?<br/>(search_export.photo_id)"}
    S{"¿la foto sigue<br/>en el índice?"}
    X{"¿el exportado sigue<br/>en el índice?"}
    F["final<br/>miniatura de la edición"]
    U["sin editar<br/>miniatura del original"]
    M["ya no está<br/>nombre copiado al agregar"]

    I -- "photo" --> P
    P -- "sí" --> F
    P -- "no" --> S
    S -- "sí" --> U
    S -- "no" --> M
    I -- "export" --> X
    X -- "sí" --> F
    X -- "no" --> M

    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3000
    classDef ok fill:#d5e8d4,stroke:#82b366,color:#102a12
    classDef err fill:#f8cecc,stroke:#b85450,color:#3d0d0d
    class I,P,S,X dec
    class F,U ok
    class M err
```

Un disco desconectado no hace que un elemento "ya no esté": el índice conserva las filas de sus
proyectos. El aviso de discos faltantes cuenta, por disco, los elementos cuyo proyecto no está
abierto.

El índice incluye los archivos de `_exported/` (`search_export`), ligados a su foto por
`photo.jpg_path` o, si todavía no hay enlace, por nombre con la misma regla que `sync_exported`
(stem sin extensión, sin distinguir mayúsculas). La miniatura de la edición solo se rehace si
cambia el tamaño o la fecha del archivo.

## 1b. Exportado hecho fuera de Lumik

```mermaid
sequenceDiagram
    autonumber
    actor U as Fotógrafo
    participant T as Tablet
    participant W as device_watch / scan
    participant IP as IndexProject
    participant AD as AlbumDetail

    U->>T: revela DSCF0042.RAF y deja DSCF0042.jpg en _exported/
    U->>W: conecta el disco
    W->>IP: reindexar los proyectos abiertos
    IP->>IP: firma de _exported/ distinta a la guardada<br/>(aunque la generación sea la misma)
    IP->>IP: liga DSCF0042.jpg a su foto por nombre
    IP-->>AD: search-index-updated
    AD-->>U: el elemento pasa a final
    Note over IP: al abrir el proyecto, sync_exported<br/>guarda el mismo enlace en project.db
```

## 2. Abrir y mostrar en carpeta

```mermaid
sequenceDiagram
    autonumber
    actor U as Fotógrafo
    participant AD as AlbumDetail
    participant V as CollectionViewer
    participant BE as reveal_album_item
    participant OS as Explorador

    U->>AD: abrir un elemento
    alt disco conectado y con foto
        AD->>V: elementos que se pueden abrir, desde este
        V-->>U: visor encima del álbum
        U->>V: flechas
        V-->>U: siguiente elemento del álbum (aunque sea de otro proyecto)
        U->>V: Esc
        V-->>AD: el álbum, en la misma posición
    else sin disco
        AD-->>U: "conecta WD 4TB"
    end
    U->>AD: Mostrar en carpeta
    AD->>BE: item_id
    alt final
        BE->>OS: _exported/{archivo}
    else sin editar
        BE->>OS: original (o el JPG de cámara de un par)
    else sin disco
        BE-->>AD: disk_missing:<disco>
    end
```

## Código

| Pieza | Archivo |
|-------|---------|
| Id y extensiones de los exportados | `crates/lumik-core/src/domain/exported.rs` |
| Tablas `album`, `album_item`, `search_export` | `crates/lumik-core/src/db/global_schema.sql` |
| Álbumes: operaciones y mejor versión | `crates/lumik-core/src/db/albums.rs` |
| Indexar `_exported/` | `crates/lumik-core/src/application/use_cases/index_project.rs` |
| Leer un álbum, agregar resultados | `crates/lumik-core/src/application/use_cases/albums.rs` |
| Comandos y mostrar en carpeta | `src-tauri/src/commands/albums.rs` |
| Lista, detalle, agregar y reemplazar | `src/pages/Albums/` |
| Visor que recorre el álbum o los resultados | `src/pages/Collection/CollectionViewer.tsx` |
