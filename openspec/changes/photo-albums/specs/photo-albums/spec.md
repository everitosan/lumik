# Spec Delta

## Purpose
Permite agrupar y curar fotos de varios proyectos y discos en álbumes, desde antes de editarlas, para que el álbum quede listo cuando las fotos estén terminadas.

## ADDED Requirements

### Requirement: Álbumes del equipo
El fotógrafo SHALL poder crear, renombrar y borrar álbumes. Los álbumes SHALL guardarse en el equipo, no en los discos, y borrar un álbum no SHALL tocar ninguna foto ni archivo.

#### Scenario: Crear un álbum
- **WHEN** el fotógrafo crea el álbum "Convocatoria México 2026"
- **THEN** aparece en la sección Álbumes, vacío

#### Scenario: Borrar un álbum
- **WHEN** el fotógrafo borra un álbum con 24 fotos
- **THEN** el álbum desaparece y las 24 fotos siguen en sus proyectos sin cambios

### Requirement: Agregar fotos
El fotógrafo SHALL poder agregar una foto a un álbum desde el visor (también con un atajo configurable, por defecto `a`), desde el menú de la foto en el grid de un proyecto y en los resultados de búsqueda, y todas las fotos de una búsqueda de una vez. Elegir el álbum SHALL ofrecer también crear uno nuevo. Una foto que ya está en el álbum no SHALL agregarse dos veces.

#### Scenario: Desde el visor
- **WHEN** el fotógrafo pulsa `a` en el visor y elige "Convocatoria México 2026"
- **THEN** la foto queda en ese álbum

#### Scenario: Todos los resultados
- **WHEN** el fotógrafo busca `tag:calle estrellas>=4` y agrega todos los resultados a un álbum
- **THEN** el álbum contiene esas fotos, sin repetir las que ya tenía

### Requirement: Mejor versión de cada elemento
Un álbum SHALL aceptar originales y archivos de `_exported/`. Un elemento que es una foto SHALL mostrarse con su exportado ligado si lo tiene ("final") y con el original si no ("sin editar"). El álbum SHALL indicar cuántos elementos son finales y cuántos están sin editar.

#### Scenario: El álbum madura al exportar
- **WHEN** una foto se agregó al álbum antes de editarla y después aparece su JPG en `_exported/`
- **THEN** el álbum la muestra con la edición y la cuenta como final

#### Scenario: Exportado hecho fuera de Lumik
- **WHEN** con el disco fuera del equipo se deja en `_exported/` el JPG de una foto del álbum (por ejemplo desde una tablet) y luego se conecta el disco, sin abrir el proyecto
- **THEN** el álbum la muestra como final en cuanto se reindexa el proyecto, sin tener que volver a abrir el álbum

### Requirement: Reemplazar y quitar
El fotógrafo SHALL poder quitar un elemento del álbum y reemplazarlo por otra foto o por un exportado del mismo proyecto. Ninguna de las dos acciones SHALL tocar archivos.

#### Scenario: Reemplazar por una variante exportada
- **WHEN** el fotógrafo reemplaza un elemento por `IMG_1-bn.jpg` de `_exported/`
- **THEN** el álbum muestra `IMG_1-bn.jpg` en lugar del elemento anterior

### Requirement: Álbum con discos desconectados
Un álbum SHALL verse completo con miniaturas aunque los discos de sus fotos no estén conectados, y SHALL avisar qué discos faltan y cuántas fotos hay en cada uno. Al conectar o expulsar un disco con el álbum abierto, el álbum SHALL actualizarse sin recargar la vista.

#### Scenario: Dos discos faltantes
- **WHEN** el fotógrafo abre un álbum con 15 fotos en "WD 4TB" y 4 en "SSD Viajes", ninguno conectado
- **THEN** ve todas las miniaturas y el aviso de que faltan esos dos discos, con 15 y 4 fotos

#### Scenario: Expulsar un disco con el álbum abierto
- **WHEN** el fotógrafo expulsa el disco de algunas fotos del álbum que está viendo
- **THEN** esas fotos pasan a no conectadas y el aviso de discos faltantes lo incluye, sin recargar la vista

### Requirement: Fotos que ya no existen
Si la foto de un elemento se manda a la papelera o su proyecto se elimina, el elemento SHALL quedarse en el álbum con su nombre y una marca de que ya no está en su proyecto, hasta que el fotógrafo lo quite.

#### Scenario: Foto en la papelera
- **WHEN** el fotógrafo manda a la papelera una foto que estaba en un álbum
- **THEN** el álbum conserva el elemento con su nombre y la marca "ya no está en su proyecto"

### Requirement: Abrir un elemento
Con el disco conectado, abrir un elemento SHALL mostrarlo en el visor, encima del álbum. Las flechas SHALL recorrer los elementos del álbum en el orden en que se ven, aunque sean de proyectos distintos, saltando los que no se pueden abrir. Cerrar el visor SHALL volver al álbum en la misma posición. Sin el disco, SHALL indicar qué disco conectar.

#### Scenario: Siguiente elemento del álbum
- **WHEN** el fotógrafo ve en el visor un elemento y pulsa la flecha derecha
- **THEN** ve el siguiente elemento del álbum que se puede abrir, aunque sea de otro proyecto

#### Scenario: Volver al álbum
- **WHEN** el fotógrafo abre un elemento en el visor y lo cierra con Esc
- **THEN** vuelve al álbum

### Requirement: Mostrar en carpeta
"Mostrar en carpeta" SHALL abrir el explorador con el archivo del elemento seleccionado: el exportado si el elemento es final; si no, el original, o el JPG de cámara de un par. Sin el disco, SHALL indicar qué disco conectar.

#### Scenario: Mostrar en carpeta un final
- **WHEN** el fotógrafo elige "Mostrar en carpeta" en un elemento final
- **THEN** se abre el explorador en `_exported/` con ese JPG seleccionado
