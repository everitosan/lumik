# photo-search Specification

## Purpose
Permite al fotógrafo encontrar fotos en todos sus proyectos y discos desde un solo lugar, con filtros combinables y texto libre, incluso cuando el disco de esas fotos no está conectado.

## Requirements

### Requirement: Índice de fotos en el equipo
Cada equipo SHALL mantener un índice de las fotos de todos los proyectos que ha visto, en su base global. El índice SHALL refrescarse cuando se conecta un disco y cuando cambia un proyecto, y SHALL conservar las fotos de discos desconectados. Las fotos en la papelera no SHALL estar en el índice, y los proyectos eliminados SHALL salir de él.

#### Scenario: Disco desconectado
- **WHEN** el fotógrafo desconecta el disco donde está "Boda Martínez"
- **THEN** las fotos de "Boda Martínez" siguen apareciendo en la búsqueda

#### Scenario: Cambio en un proyecto
- **WHEN** el fotógrafo califica una foto con 5 estrellas
- **THEN** una búsqueda `estrellas>=5` la incluye sin reconectar el disco

### Requirement: Operadores de búsqueda
El cuadro de búsqueda SHALL entender estos operadores, combinables con Y: `tag:x`, `-tag:x`, `estrellas` comparada con un número (`=`, `>`, `>=`, `<`, `<=`, o `:` como `=`), `color:nombre`, `camara:texto`, `lente:texto`, `en:AAAA` o `en:AAAA-MM`, `antes:AAAA-MM-DD`, `despues:AAAA-MM-DD`, `proyecto:texto`, `es:aprobada|entregada|par` e `incluir:archivados`.

#### Scenario: Comparar estrellas
- **WHEN** el fotógrafo busca `estrellas>4`
- **THEN** ve las fotos con 5 estrellas, igual que con `estrellas=5` o `estrellas>=5`

#### Scenario: Consulta combinada
- **WHEN** el fotógrafo busca `tag:calle -tag:boda estrellas>=4 en:2025`
- **THEN** ve las fotos de 2025 con el tag "calle", sin el tag "boda" y con 4 o más estrellas

### Requirement: Lectura de la consulta
Los operadores SHALL aceptarse también en inglés (`stars>=`, `camera:`, `is:approved`…). Los valores con espacios SHALL ir entre comillas. Lo que no sea un operador, o tenga un valor inválido, SHALL tratarse como texto libre.

#### Scenario: Operadores en inglés
- **WHEN** el fotógrafo busca `stars>=4 is:approved`
- **THEN** ve las fotos aprobadas con 4 o más estrellas

#### Scenario: Valor con espacios
- **WHEN** el fotógrafo busca `camara:"X-T5"`
- **THEN** ve las fotos tomadas con esa cámara

#### Scenario: Valor inválido
- **WHEN** el fotógrafo busca `estrellas>=9`
- **THEN** se busca como texto libre y no como filtro de estrellas

### Requirement: Buscador del dashboard
La búsqueda SHALL vivir en el buscador del dashboard. Al escribir texto sin operadores, el buscador SHALL filtrar los proyectos por nombre como hasta ahora. Pulsar Enter, o buscar desde el panel avanzado, SHALL mostrar en lugar de los proyectos los resultados de fotos de todos los proyectos, con una forma de volver a los proyectos. Mientras el texto lleve operadores, la lista de proyectos no SHALL filtrarse.

#### Scenario: Filtrar proyectos al escribir
- **WHEN** el fotógrafo escribe `boda` en el buscador sin pulsar Enter
- **THEN** ve solo los proyectos cuyo nombre contiene "boda"

#### Scenario: Buscar una palabra en las fotos
- **WHEN** el fotógrafo escribe `calle` y pulsa Enter
- **THEN** ve las fotos de todos los proyectos que coinciden con "calle", agrupadas por proyecto

#### Scenario: Volver a los proyectos
- **WHEN** el fotógrafo está viendo resultados de fotos y pulsa "Volver a proyectos" o borra el buscador
- **THEN** vuelve a ver el grid de proyectos

### Requirement: Texto libre
El texto libre SHALL buscar las fotos cuyos tags, nombre de archivo, nombre de proyecto o descripción del proyecto contengan todas sus palabras, sin distinguir mayúsculas ni acentos.

#### Scenario: Palabra en un tag
- **WHEN** el fotógrafo escribe `calle` sin operadores
- **THEN** aparecen las fotos con el tag "calle" y las de proyectos cuyo nombre contiene "calle"

### Requirement: Búsqueda avanzada
Un panel de búsqueda avanzada SHALL ofrecer los campos texto libre, tags, sin tags, estrellas, color, cámara, lente, fecha de captura (un año, un mes o un rango), proyecto, estado (aprobada, entregada, par RAW+JPG) e incluir archivados. Buscar desde el panel SHALL escribir en el cuadro los operadores equivalentes, en el idioma de la app, y abrir el panel con una consulta escrita SHALL precargar sus campos.

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
Abrir una foto de los resultados SHALL abrirla en el visor, encima de los resultados, si su disco está conectado. Si no, SHALL indicar qué disco conectar. En el visor, las flechas SHALL recorrer los resultados en el orden en que se ven, aunque sean de proyectos distintos, saltando los que no tienen su disco conectado. Cerrar el visor SHALL volver a los resultados, con la misma consulta y en la misma posición.

#### Scenario: Disco conectado
- **WHEN** el fotógrafo abre un resultado cuyo disco está conectado
- **THEN** se abre esa foto en el visor, sin pasar por el grid de su proyecto

#### Scenario: Siguiente resultado
- **WHEN** el fotógrafo ve en el visor un resultado de "Boda" y el siguiente resultado es de "XV Ana"
- **THEN** la flecha derecha muestra esa foto de "XV Ana"

#### Scenario: Volver a los resultados
- **WHEN** el fotógrafo abrió una foto desde los resultados y cierra el visor con Esc
- **THEN** vuelve al dashboard con los resultados de su búsqueda, en la misma posición

#### Scenario: Disco no conectado
- **WHEN** el fotógrafo abre un resultado cuyo disco no está conectado
- **THEN** ve la miniatura y un aviso con el nombre del disco que debe conectar
