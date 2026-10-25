# Spec Delta

## Purpose
Permite al fotógrafo encontrar fotos en todos sus proyectos y discos desde un solo lugar, con filtros combinables y texto libre, incluso cuando el disco de esas fotos no está conectado.

## ADDED Requirements

### Requirement: Índice de fotos en el equipo
Cada equipo SHALL mantener un índice de las fotos de todos los proyectos que ha visto, en su base global. El índice SHALL refrescarse cuando se conecta un disco y cuando cambia un proyecto, y SHALL conservar las fotos de discos desconectados. Las fotos en la papelera no SHALL estar en el índice, y los proyectos eliminados SHALL salir de él.

#### Scenario: Disco desconectado
- **WHEN** el fotógrafo desconecta el disco donde está "Boda Martínez"
- **THEN** las fotos de "Boda Martínez" siguen apareciendo en la búsqueda

#### Scenario: Cambio en un proyecto
- **WHEN** el fotógrafo califica una foto con 5 estrellas
- **THEN** una búsqueda `estrellas>=5` la incluye sin reconectar el disco

### Requirement: Operadores de búsqueda
El cuadro de búsqueda SHALL entender estos operadores, combinables con Y: `tag:x` (tiene el tag), `-tag:x` (no lo tiene), `estrellas>=n`, `estrellas:n`, `color:nombre`, `camara:texto`, `lente:texto`, `en:AAAA` o `en:AAAA-MM`, `antes:AAAA-MM-DD`, `despues:AAAA-MM-DD`, `proyecto:texto` y `es:aprobada|entregada|par`. Los valores con espacios SHALL ir entre comillas. Lo que no sea un operador SHALL tratarse como texto libre.

#### Scenario: Consulta combinada
- **WHEN** el fotógrafo busca `tag:calle -tag:boda estrellas>=4 en:2025`
- **THEN** ve las fotos de 2025 con el tag "calle", sin el tag "boda" y con 4 o más estrellas

#### Scenario: Valor con espacios
- **WHEN** el fotógrafo busca `camara:"X-T5"`
- **THEN** ve las fotos tomadas con esa cámara

### Requirement: Texto libre
El texto libre SHALL buscar las fotos cuyos tags, nombre de archivo, nombre de proyecto o descripción del proyecto contengan todas sus palabras, sin distinguir mayúsculas ni acentos.

#### Scenario: Palabra en un tag
- **WHEN** el fotógrafo escribe `calle` sin operadores
- **THEN** aparecen las fotos con el tag "calle" y las de proyectos cuyo nombre contiene "calle"

### Requirement: Búsqueda avanzada
Un panel de búsqueda avanzada SHALL ofrecer los campos texto libre, tags, sin tags, estrellas, color, cámara, lente, fecha de captura (un año, un mes o un rango), proyecto, estado (aprobada, entregada, par RAW+JPG) e incluir archivados. Buscar desde el panel SHALL escribir en el cuadro los operadores equivalentes, y abrir el panel con una consulta escrita SHALL precargar sus campos.

#### Scenario: Del panel al cuadro
- **WHEN** el fotógrafo elige en el panel el tag "retrato" y "4 o más estrellas" y pulsa buscar
- **THEN** el cuadro muestra `tag:retrato estrellas>=4` y los resultados de esa consulta

#### Scenario: Del cuadro al panel
- **WHEN** el cuadro tiene `es:aprobada en:2025` y el fotógrafo abre el panel
- **THEN** el panel muestra marcada "aprobada" y la fecha "en 2025"

### Requirement: Resultados
Los resultados SHALL agruparse por proyecto, con el nombre y la fecha del proyecto y una miniatura por foto, y SHALL ordenarse por fecha de captura de la más reciente a la más antigua. Los proyectos archivados SHALL quedar fuera salvo que se pida incluirlos. Sin resultados, la vista SHALL decirlo.

#### Scenario: Resultados de varios proyectos
- **WHEN** una consulta encuentra fotos en "Boda Martínez" y en "XV Ana"
- **THEN** los resultados muestran dos grupos con sus fotos

### Requirement: Abrir un resultado
Abrir una foto de los resultados SHALL abrirla en el visor de su proyecto si su disco está conectado. Si no, SHALL indicar qué disco conectar.

#### Scenario: Disco conectado
- **WHEN** el fotógrafo abre un resultado cuyo disco está conectado
- **THEN** se abre esa foto en el visor de su proyecto

#### Scenario: Disco no conectado
- **WHEN** el fotógrafo abre un resultado cuyo disco no está conectado
- **THEN** ve la miniatura y un aviso con el nombre del disco que debe conectar
