# Flujo: Búsqueda por contenido

Lumik Desktop · Linux · Windows · macOS · Android (y la futura app de tablet, vía `lumik-core`)

Con el modelo instalado, el texto libre de la búsqueda trae también las fotos que **se parecen** a lo
escrito ("niños en la playa", "kids on the beach"), aunque no tengan esas palabras en sus tags.
El modelo es opcional y se descarga desde Ajustes → IA. Cada foto se analiza una vez: su vector se
guarda en el `project.db` de su disco, así que otro equipo con el mismo disco lo reusa sin volver a
analizarla. Todo corre en el equipo, sin red, salvo la descarga del modelo.

## Vista general

```mermaid
flowchart LR
    A["Ajustes → IA<br/>Descargar"]
    B["ContentModel<br/>zip verificado en<br/>{datos}/models/clip-multilingual-v1/"]
    C["Analizar fotos<br/>al importar o desde el proyecto"]
    D["AnalysisQueue + hilo content-analysis<br/>AnalyzeProject"]
    E["project.db<br/>photo_embedding"]
    F["IndexProject<br/>lumik.db · search_media.embedding"]
    G["SearchPhotos<br/>palabras + parecido"]
    H["Resultados<br/>aviso 'N fotos sin analizar'"]

    A --> B
    C --> D
    B --> D --> E --> F
    B --> G
    F --> G --> H
    H -. "Analizar" .-> D

    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef db fill:#ffcdd2,stroke:#c62828,color:#3d0d0d
    class A,C,H ui
    class D,F,G be
    class B,E db
```

## 1. Descargar el modelo

```mermaid
sequenceDiagram
    autonumber
    participant UI as AiSettingsSection
    participant CMD as download_content_model
    participant CM as ContentModel / ModelStore
    participant FS as {datos}/models/

    UI->>CMD: Descargar
    CMD->>CM: download (hilo content-model-download)
    loop cada bloque
        CM->>FS: {id}.zip.part + SHA-256 al vuelo
        CM-->>UI: content-model-state · downloading (cada ~100 ms)
    end
    alt cancelar (cancel_content_model_download)
        CM->>FS: borrar el .part
        CM-->>UI: not_installed, sin error
    else red, tamaño o hash distintos, manifest inválido o disco
        CM->>FS: borrar el .part (y {id}.extracting/)
        CM-->>UI: not_installed · error network:… / corrupt:… / io:…
    else todo coincide
        CM->>FS: descomprimir en {id}.extracting/ y validar manifest.json
        CM->>FS: renombrar a {id}/
        CM-->>UI: installed
    end
```

El paquete (`image.onnx`, `text.onnx`, `tokenizer.json`, `manifest.json` y las licencias) es el
asset del release `model-clip-multilingual-v1`; su URL, tamaño y SHA-256 están en
`domain::embedding::CLIP_MULTILINGUAL`. `LUMIK_MODEL_URL` (`https://` o `file://`) descarga otro,
p. ej. uno armado en local con `tools/models/`.

"Borrar" (`delete_content_model`) quita la carpeta del modelo, pero no los vectores de los proyectos:
siguen en cada `project.db` y sirven si se vuelve a descargar.

## 2. Analizar las fotos de un proyecto

```mermaid
sequenceDiagram
    autonumber
    participant UI as Proyecto / barra lateral / resultados
    participant IMP as finish_import
    participant Q as AnalysisQueue
    participant W as hilo content-analysis
    participant UC as AnalyzeProject
    participant P as project.db (disco)
    participant IQ as IndexQueue

    UI->>Q: analyze_project (error no_model sin modelo)
    IMP->>Q: request si el proyecto es on_import y hay modelo
    W->>Q: next() — un proyecto a la vez, en orden
    W-->>UI: analysis-progress · running
    W->>P: write_session (candado + generación)
    alt abierto en otro dispositivo
        W-->>UI: locked
    else sesión tomada
        UC->>P: pendientes: sin vector, de otro modelo o rotadas
        loop cada foto (cancel se revisa antes)
            UC->>UC: thumbnail de .thumbs/ (lo genera si falta)<br/>embed_image
            UC->>P: lote de 32 vectores en photo_embedding
            W-->>UI: running · done/total (cada ~250 ms)
        end
        W->>Q: record_failed (las que fallaron no se ofrecen más en la sesión)
        W-->>UI: done / cancelled · analizadas y fallidas
        W->>IQ: al soltar la sesión, reindexar en 2 s
    end
```

"Detener" (`stop_project_analysis`) levanta la bandera de cancelación: lo hecho se queda y el resto
sigue pendiente. Expulsar el disco (`eject_device`) detiene igual el análisis de sus proyectos y
espera hasta 10 s a que suelte la base.

El modo de cada proyecto (`on_import` u `on_demand`) va en "Editar proyecto"; los proyectos nuevos
toman el de Ajustes → IA (`default_analysis_mode`).

## 3. Buscar por contenido

```mermaid
flowchart TB
    A["'playa estrellas>=3'"]
    B["SearchQuery::parse<br/>operadores + texto libre"]
    C["Por palabras<br/>como siempre, por fecha"]
    D{"¿modelo instalado<br/>y texto libre?"}
    E["embed_text('playa')"]
    F["nearest_photos<br/>producto punto con los vectores de lumik.db<br/>(en caché, se recargan si cambia el índice)<br/>parecido ≥ 0.24, hasta 3000"]
    G["content_hits<br/>las que pasan los operadores<br/>sin repetir las de palabras, hasta 300"]
    H["unanalyzed_photos<br/>por proyecto: pasan los operadores, sin vector<br/>sin las que fallaron · connected"]
    I["Resultados<br/>primero palabras, luego de más a menos parecidas"]
    J["UnanalyzedNotice<br/>'N fotos sin analizar' · 'N en discos no conectados'<br/>Analizar → analyze_project de los conectados"]

    A --> B --> C --> D
    D -- no --> I
    D -- sí --> E --> F --> G --> I
    D -- sí --> H --> J

    classDef ui fill:#dae8fc,stroke:#6c8ebf,color:#10243d
    classDef be fill:#ffe0b2,stroke:#f57c00,color:#3d2600
    classDef dec fill:#fff2cc,stroke:#d6b656,color:#3d3000
    class A,I,J ui
    class B,C,E,F,G,H be
    class D dec
```

Los vectores viven en `lumik.db`, así que también salen las fotos de discos no conectados (con
"conecta…", como en la búsqueda por palabras). Sin modelo, o con solo operadores, la búsqueda es la
de siempre.

## Código

| Pieza | Archivo |
|-------|---------|
| Paquete del modelo (URL, tamaño, hash), modo de análisis | `crates/lumik-core/src/domain/embedding.rs` |
| Carga del modelo, preprocesado y tokenizer (`ort`, feature `embeddings`) | `crates/lumik-core/src/embedding.rs` |
| Analizar un proyecto | `crates/lumik-core/src/application/use_cases/analyze_project.rs` |
| Cola de análisis y fotos fallidas | `crates/lumik-core/src/application/analysis_queue.rs` |
| Vectores al índice | `crates/lumik-core/src/application/use_cases/index_project.rs` |
| Búsqueda por parecido y sin analizar | `crates/lumik-core/src/application/use_cases/search_photos.rs`, `db/search_index.rs` |
| Descarga, verificación e instalación | `src-tauri/src/infrastructure/content_model.rs` |
| Comandos del modelo y del análisis | `src-tauri/src/commands/content_model.rs`, `commands/analysis.rs` |
| Hilo de análisis, análisis tras importar, al expulsar | `src-tauri/src/commands.rs` (`spawn_analysis_worker`), `commands/import_cmd.rs`, `commands/device.rs` |
| Ajustes → IA y modo | `src/pages/AiSettingsSection.tsx`, `src/components/AnalysisModeSelect.tsx` |
| Proyecto, barra lateral y resultados | `ProjectDetailHeader.tsx`, `EditProjectModal.tsx`, `src/components/Sidebar.tsx`, `src/pages/Search/PhotoSearchResults.tsx` |
| Empaquetar el modelo | `tools/models/build_clip_multilingual.py` |
