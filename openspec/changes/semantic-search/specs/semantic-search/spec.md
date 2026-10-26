# Spec Delta

## Purpose
Permite encontrar fotos por lo que muestran, describiéndolas en español o en inglés, con un modelo opcional que corre en el equipo y vectores que viajan con cada proyecto en su disco.

## ADDED Requirements

### Requirement: Modelo opcional
La búsqueda por contenido SHALL depender de un modelo que no viene con la instalación. Ajustes SHALL ofrecer descargarlo, con su tamaño, progreso y opción de cancelar. Lumik SHALL verificar el archivo antes de usarlo y desechar uno incompleto o alterado. Mientras no esté el modelo, la app SHALL funcionar como antes, sin analizar fotos ni buscar por contenido.

#### Scenario: Instalación nueva
- **WHEN** el fotógrafo instala Lumik y busca `playa`
- **THEN** la búsqueda funciona solo con tags, nombres y datos del proyecto, y no hay opción para analizar fotos en los proyectos

#### Scenario: Descargar el modelo
- **WHEN** el fotógrafo pulsa "Descargar" en Ajustes → Búsqueda y la descarga termina
- **THEN** Ajustes muestra el modelo como instalado y se activan la búsqueda por contenido y el análisis de fotos

#### Scenario: Descarga interrumpida
- **WHEN** se corta la red a la mitad de la descarga
- **THEN** Ajustes muestra el error con la opción de reintentar y el modelo sigue sin instalar

### Requirement: Borrar el modelo
Ajustes SHALL permitir borrar el modelo instalado. Al borrarlo, la app SHALL volver a funcionar como sin modelo. Los vectores ya calculados SHALL conservarse en los discos.

#### Scenario: Borrar y volver a descargar
- **WHEN** el fotógrafo borra el modelo y días después lo vuelve a descargar
- **THEN** las fotos que ya estaban analizadas se pueden buscar por contenido sin analizarlas de nuevo

### Requirement: Todo en el equipo
El análisis de las fotos y la búsqueda por contenido SHALL hacerse en el equipo. Ni las fotos, ni sus vectores, ni el texto que se busca SHALL enviarse a ningún servidor. La única conexión de la función SHALL ser la descarga del modelo.

#### Scenario: Sin red
- **WHEN** el modelo está instalado y el equipo no tiene red
- **THEN** el fotógrafo puede analizar fotos y buscar por contenido igual que con red

### Requirement: Vectores en el disco del proyecto
El vector de cada foto SHALL guardarse en el proyecto, en su disco, junto con el modelo que lo generó, para que viaje con el proyecto a cualquier equipo. Un equipo que ve un proyecto con vectores SHALL usarlos sin recalcularlos, aunque no tenga el modelo instalado al conectarlo. Un vector de otro modelo SHALL tratarse como pendiente.

#### Scenario: Proyecto analizado en otro equipo
- **WHEN** el fotógrafo analiza "Boda Martínez" en su escritorio y conecta el disco a otro equipo con el modelo instalado
- **THEN** en el otro equipo puede buscar por contenido las fotos de "Boda Martínez" sin analizarlas

#### Scenario: Proyecto bloqueado
- **WHEN** un proyecto está abierto en otro dispositivo
- **THEN** este equipo no analiza sus fotos, pero sí usa los vectores que ya tiene

### Requirement: Cuándo se analizan las fotos
Cada proyecto SHALL tener un modo de análisis: al importar o bajo demanda. Con "al importar", las fotos nuevas SHALL analizarse en segundo plano al terminar cada importación. Con "bajo demanda", SHALL analizarse solo cuando el fotógrafo pulse "Analizar fotos" en el proyecto. Los proyectos nuevos SHALL tomar el modo que indique Ajustes → Búsqueda.

#### Scenario: Al importar
- **WHEN** un proyecto está en "al importar" e importa 300 fotos
- **THEN** al terminar la importación las fotos se analizan en segundo plano y después se encuentran por contenido

#### Scenario: Bajo demanda
- **WHEN** un proyecto está en "bajo demanda" e importa fotos
- **THEN** no se analizan hasta que el fotógrafo pulsa "Analizar fotos", que procesa solo las que faltan

#### Scenario: Modo de los proyectos nuevos
- **WHEN** Ajustes → Búsqueda indica "al importar" y el fotógrafo crea un proyecto
- **THEN** el proyecto queda en "al importar" y puede cambiarse a "bajo demanda"

### Requirement: Progreso del análisis
Mientras se analizan fotos, la app SHALL mostrar el proyecto y cuántas van de cuántas, y SHALL permitir detenerlo. Lo analizado hasta ese momento SHALL conservarse. Una foto rotada SHALL volver a analizarse.

#### Scenario: Detener y seguir
- **WHEN** el fotógrafo detiene el análisis en la foto 120 de 300 y después pulsa "Analizar fotos"
- **THEN** se analizan las 180 que faltan

#### Scenario: Expulsar el disco
- **WHEN** se expulsa el disco de un proyecto que se está analizando
- **THEN** el análisis se detiene sin error y lo analizado queda guardado

### Requirement: Buscar por contenido
Con el modelo instalado, el texto libre SHALL encontrar también las fotos analizadas cuyo contenido se parece a lo escrito, en español o en inglés, y no solo las que tienen esas palabras. Los operadores SHALL seguir filtrando esos resultados. Las fotos sin analizar SHALL aparecer solo si coinciden por tags, nombre o datos del proyecto.

#### Scenario: Descripción en español
- **WHEN** el fotógrafo busca `novia en la playa` y ninguna foto tiene esos tags
- **THEN** ve las fotos analizadas de una novia en la playa, de cualquier proyecto

#### Scenario: Mismo concepto en inglés
- **WHEN** el fotógrafo busca `bride on the beach`
- **THEN** ve en esencia las mismas fotos que con `novia en la playa`

#### Scenario: Con operadores
- **WHEN** el fotógrafo busca `atardecer estrellas>=4 en:2025`
- **THEN** ve las fotos de 2025 con 4 o más estrellas que parecen un atardecer

#### Scenario: Disco desconectado
- **WHEN** el disco de una foto analizada no está conectado
- **THEN** la foto aparece igual en la búsqueda por contenido, con su miniatura y el aviso de qué disco conectar

### Requirement: Fotos sin analizar
Los resultados de una búsqueda por contenido SHALL indicar cuántas fotos de los proyectos buscados faltan por analizar, con un acceso para analizarlas.

#### Scenario: Proyectos a medias
- **WHEN** el fotógrafo busca `retrato` y hay 800 fotos sin analizar en proyectos conectados
- **THEN** los resultados avisan que 800 fotos no están analizadas y ofrecen analizarlas

### Requirement: Licencias del modelo
About SHALL listar el motor de inferencia, el tokenizer y los modelos con su licencia, y el paquete del modelo SHALL incluir los textos de sus licencias.

#### Scenario: Ver licencias
- **WHEN** el fotógrafo abre About → Licencias de código abierto
- **THEN** ve el motor de inferencia, el tokenizer y los dos modelos con su licencia y enlace
