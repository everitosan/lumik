# Spec Delta

## Purpose

Mantiene en el servidor un catálogo de los proyectos y fotos de cada fotógrafo, publicado desde el desktop, para que el portal web y el iPad puedan listarlos y respaldar contra ellos aunque el disco no esté conectado.

## ADDED Requirements

### Requirement: Fuentes de verdad
La existencia y los metadatos de proyectos y fotos (nombre, fecha, slug, archivo) SHALL mandar desde el desktop. El estado de respaldo de cada foto (estado, ruta remota, fecha, hash, tamaño) SHALL mandar desde el servidor, y una publicación del desktop no SHALL sobrescribirlo.

#### Scenario: Republicar fotos ya respaldadas
- **WHEN** el desktop vuelve a publicar las fotos de un proyecto cuyas fotos ya figuran como subidas
- **THEN** los metadatos se actualizan y el estado de respaldo sigue siendo `uploaded`

### Requirement: Publicación al abrir un proyecto
Al abrir un proyecto con sesión iniciada, el desktop SHALL comprobar si el proyecto está en el catálogo. Si no está, SHALL publicar el proyecto, sus fotos y su portada. Si ya está, no SHALL republicarlo. Sin sesión o sin conexión, el proyecto SHALL abrirse con normalidad en modo solo local.

#### Scenario: Primer apertura con sesión
- **WHEN** el fotógrafo abre un proyecto que el catálogo no conoce
- **THEN** el proyecto aparece en `GET /v1/projects` con su portada

#### Scenario: Sin conexión
- **WHEN** el fotógrafo abre un proyecto sin red
- **THEN** el proyecto se abre sin errores y no se publica nada

#### Scenario: Token caducado
- **WHEN** al publicar el servidor responde que el token de acceso caducó
- **THEN** el desktop lo renueva y reintenta una vez

### Requirement: Propiedad derivada del token
Los proyectos publicados SHALL atribuirse al fotógrafo del token de acceso. El `creator_id` local no SHALL enviarse ni igualarse a la identidad del servidor. Un proyecto existente de otro fotógrafo no SHALL poder sobrescribirse publicando el mismo id.

#### Scenario: Id ajeno
- **WHEN** el fotógrafo A publica un proyecto con un id que ya pertenece al fotógrafo B
- **THEN** el proyecto de B no se modifica

### Requirement: Consulta del catálogo
El servidor SHALL exponer los proyectos del fotógrafo, de más reciente a más antiguo, con número de fotos, número de fotos respaldadas, estado de flujo y proveedores usados. SHALL exponer también la existencia de un proyecto (200 o 404) y sus fotos, filtrables por estado de respaldo.

#### Scenario: Fotos pendientes
- **WHEN** se piden las fotos de un proyecto con `?backup=pending`
- **THEN** solo se devuelven las fotos con estado `pending`

### Requirement: Portada en el catálogo
El desktop SHALL subir la miniatura de portada al publicar un proyecto y cada vez que cambie la portada. El servidor SHALL devolverla solo al dueño del proyecto, o 404 si no tiene.

#### Scenario: Cambiar la portada
- **WHEN** el fotógrafo elige otra portada en el desktop con sesión iniciada
- **THEN** el portal web muestra la nueva portada

### Requirement: Enlace de `_exported` con las fotos originales
Al abrir un proyecto, el desktop SHALL buscar en `_exported/` archivos de imagen no ocultos (`jpg`, `jpeg`, `png`, `tif`, `tiff`, `webp`) y emparejarlos con las fotos del proyecto por nombre sin extensión, sin distinguir mayúsculas. Cada foto emparejada SHALL enlazarse a su exportado y pasar a estado `edited`, y los cambios SHALL publicarse en el catálogo si hay sesión.

#### Scenario: Export con el mismo nombre
- **WHEN** existe `_exported/IMG_0001.JPG` y en el proyecto hay `IMG_0001.cr2`
- **THEN** la foto queda enlazada a ese JPG y en estado `edited`

#### Scenario: Export renombrado
- **WHEN** el editor exportó `retrato-final.jpg`, que no coincide con ningún original
- **THEN** el archivo queda sin emparejar y aun así se puede respaldar

### Requirement: Identidad determinista de los exportados
Cada archivo de `_exported/` SHALL identificarse con un UUID v5 calculado a partir de `{project_id}/{nombre_de_archivo}` (espacio de nombres URL). El desktop y el portal web SHALL calcular el mismo id, para que un mismo archivo subido desde los dos lados no se duplique en el catálogo.

#### Scenario: Subir desde web lo que ya subió el desktop
- **WHEN** el portal web sube `foto1.jpg` a un proyecto en el que el desktop ya la respaldó
- **THEN** el catálogo conserva una sola entrada para `foto1.jpg`
