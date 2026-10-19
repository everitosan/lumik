# Casos de uso — Lumik

Diagramas de flujo de la suite. **Mermaid es la única fuente**: se renderizan directo en GitHub
y en el preview de Markdown de VSCode, sin herramienta externa.

Los diagramas grandes están partidos en sub-diagramas por fase, uno por sección (`## 1.`, `## 2.`, …),
precedidos de una **vista general** que cabe en una pantalla. Cada sección se puede leer sola.

## Flujos implementados

| Flujo | Qué cubre | Sub-diagramas |
|-------|-----------|---------------|
| [Crear proyecto](flujo-crear-proyecto.md) | Modal de alta, detección del dispositivo destino, creación de `project.db` | 3 |
| [Importar imágenes](flujo-importar-imagenes.md) | Wizard de 3 pasos, pipeline de copia, metadatos, EXIF, miniaturas | 6 |
| [Culling](flujo-culling.md) | Toggle `culled`, movimiento de archivo entre `_media/` y `_culled/`, rollback | 3 |
| [Rating de estrellas](flujo-rating-estrellas.md) | Estrellas, color labels, tags y el sidecar AppleDouble para iPad | 2 |
| [Rotación](flujo-rotacion.md) | Rotación optimista en canvas, regeneración de miniatura, escritura EXIF/XMP | 3 |

## WIP — respaldo en la nube (SaaS)

| Flujo | Qué cubre |
|-------|-----------|
| [Panorama de respaldo](wip-backup-panorama.md) | Mapa de componentes y rutas numeradas de todo el subsistema |
| [Login / cuenta](wip-flujo-login-cuenta.md) | Autenticación del fotógrafo, JWT, identidad única `creator_id` |
| [Conectar proveedor](wip-flujo-conectar-proveedor.md) | OAuth por fotógrafo contra Google Drive / Mega / Nextcloud |
| [`ensure_catalog_synced`](wip-flujo-sync-catalogo.md) | Publicación o reconciliación del catálogo al abrir un proyecto |
| [Respaldar `_exported`](wip-flujo-respaldar-exported.md) | Subida desde desktop con fan-out a varios remotes |
| [Respaldo desde iPad/web](wip-flujo-respaldo-ipad.md) | Subida desde campo y reconciliación posterior en desktop |

## Convenciones

Todos los diagramas usan la misma paleta, declarada con `classDef` en cada bloque:

| Clase | Capa | Relleno |
|-------|------|---------|
| `term` | Inicio / fin del flujo | verde `#d5e8d4` |
| `ui` | Frontend — React / TypeScript | azul `#dae8fc` |
| `ipc` | Puente Tauri IPC | verde `#c8e6c9` |
| `be` | Backend — Rust / Tauri | naranja `#ffe0b2` |
| `db` | Persistencia — SQLite / sistema de archivos | rojo `#ffcdd2` |
| `plat` | Rama específica por plataforma | violeta `#e1d5e7` |
| `dec` | Decisión | amarillo `#fff2cc` |
| `err` | Error / rollback | rosa `#f8cecc` |
| `note` | Nota al margen (enlace punteado) | amarillo claro `#fff9c4` |

Reglas de escritura, para que todo renderice igual en GitHub, VSCode y cualquier otro visor:

- Etiquetas siempre entre comillas, saltos de línea con `<br/>`, **sin** `<b>` ni `<i>`
  (dependen de `htmlLabels`, que no está garantizado en todos los visores).
- `end` es palabra reservada en Mermaid: el nodo final se llama `fin`.
- Los flujos con carriles (lanes) y paso de mensajes usan `sequenceDiagram`; los dominados por
  ramificación usan `flowchart`.
- Las referencias a archivos de código van en una tabla al final, no como nodos del diagrama.
