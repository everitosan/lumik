# Spec Delta

## MODIFIED Requirements

### Requirement: Texto libre
El texto libre SHALL buscar las fotos cuyos tags, nombre de archivo, nombre de proyecto o descripción del proyecto contengan todas sus palabras, sin distinguir mayúsculas ni acentos. Con el modelo de búsqueda por contenido instalado, SHALL encontrar además las fotos analizadas cuyo contenido se parece al texto (spec `semantic-search`).

#### Scenario: Palabra en un tag
- **WHEN** el fotógrafo escribe `calle` sin operadores
- **THEN** aparecen las fotos con el tag "calle" y las de proyectos cuyo nombre contiene "calle"

#### Scenario: Con el modelo instalado
- **WHEN** el modelo está instalado y el fotógrafo busca `calle`
- **THEN** aparecen las fotos con el tag "calle" y además las fotos analizadas que muestran una calle

### Requirement: Resultados
Los resultados SHALL agruparse por proyecto, con el nombre y la fecha del proyecto y una miniatura por foto, y SHALL ordenarse por fecha de captura de la más reciente a la más antigua. En una búsqueda por contenido, los grupos y las fotos de cada grupo SHALL ordenarse de la que más se parece a la que menos, después de las que coinciden por palabras. Los proyectos archivados SHALL quedar fuera salvo que se pida incluirlos. Sin resultados, la vista SHALL decirlo.

#### Scenario: Resultados de varios proyectos
- **WHEN** una consulta encuentra fotos en "Boda Martínez" y en "XV Ana"
- **THEN** los resultados muestran dos grupos con sus fotos

#### Scenario: Orden por parecido
- **WHEN** el fotógrafo busca `atardecer` con el modelo instalado
- **THEN** primero ve el grupo con la foto que más se parece a un atardecer y dentro de cada grupo las fotos de más a menos parecidas
