# Spec Delta

## Purpose

El import de la tarjeta en la app de tablet Android: mismo resultado en el disco que el import del desktop (`photo-import`), adaptado a SAF, a la lectura por fd y a la firma en sidecar `.xmp`.

## ADDED Requirements

### Requirement: Import desde un proyecto en tres pasos
El import SHALL abrirse solo desde dentro de un proyecto, que es su destino, en tres pasos: archivos, ajustes e importar. Los archivos SHALL elegirse con el selector del sistema con selección múltiple, en una lista a la que se pueden agregar más y de la que se pueden quitar. Un proyecto vacío SHALL abrir el import una vez al abrirse.

#### Scenario: Agregar y quitar archivos
- **WHEN** el fotógrafo elige 50 fotos, agrega 20 más y quita 5
- **THEN** el import copia las 65 que quedaron en la lista

#### Scenario: Proyecto recién creado
- **WHEN** el fotógrafo abre un proyecto sin fotos por primera vez
- **THEN** se abre el import de ese proyecto

### Requirement: Ajustes de la importación
El paso de ajustes SHALL mostrar, precargados con los ajustes de la tablet, si se renombran las fotos (con un ejemplo del nombre resultante) y si se firman, y SHALL permitir cambiarlos solo para esa importación sin modificar los ajustes de la tablet. En un bloque aparte, del proyecto, SHALL mostrar la opción "Al aprobar, entregar directamente" y permitir cambiarla con las mismas reglas que el desktop (`camera-jpeg-delivery`), incluida la pregunta por las fotos ya aprobadas.

#### Scenario: Sin renombrar solo esta vez
- **WHEN** el fotógrafo desactiva el renombrado en el paso de ajustes e importa `DSCF0001.RAF`
- **THEN** la foto queda como `_media/DSCF0001.RAF` y la siguiente importación vuelve a mostrar el renombrado activo

#### Scenario: Cambiar la entrega desde el import
- **WHEN** el fotógrafo cambia en el paso de ajustes la entrega del proyecto a "el JPG de los pares"
- **THEN** el proyecto queda con esa opción y los pares que se aprueben después entregan su JPG a `_exported/`

### Requirement: Espacio libre antes de importar
El import no SHALL poder iniciarse si el espacio libre del disco del proyecto es menor que el tamaño total de los archivos elegidos. El paso de importar SHALL mostrar el tamaño total y el espacio libre.

#### Scenario: Disco casi lleno
- **WHEN** el fotógrafo elige 8 GB de fotos y el disco del proyecto tiene 5 GB libres
- **THEN** el botón de importar está desactivado y se indica que falta espacio en el disco

### Requirement: Mismo resultado que el desktop
Las fotos importadas en la tablet SHALL quedar en el disco igual que con el import del desktop: en `_media/`, renombradas por fecha de captura si el renombrado está activo, con los pares RAW+JPEG con el mismo nombre, registradas en `project.db` y con miniatura. Ningún archivo de `_media/` SHALL sobrescribirse.

#### Scenario: Par RAW+JPEG
- **WHEN** se importan `DSCF0001.RAF` y `DSCF0001.JPG` con el renombrado activo
- **THEN** en `_media/` quedan los dos con el mismo nombre por fecha de captura y el proyecto los cuenta como una foto

### Requirement: Una sola lectura de la tarjeta
Cada archivo de la tarjeta SHALL leerse una sola vez: lo que se lee SHALL escribirse a la vez en la copia de trabajo de la tablet y en el staging del disco, y al terminar los originales SHALL moverse y renombrarse dentro del disco, sin volver a copiarse. Al terminar no SHALL quedar staging en el disco ni copias de los originales en la tablet.

#### Scenario: Import completo
- **WHEN** termina un import de 100 fotos sin errores
- **THEN** el disco no tiene `.import-staging/` y la tablet no conserva copias de esos originales

### Requirement: Firma del fotógrafo en sidecar
Si la firma está activa en los ajustes de la importación, cada foto SHALL llevar en un sidecar `.xmp` junto a ella el autor, el copyright y la URL de contacto del fotógrafo, y la descripción `{descripción}@{año}` si el proyecto tiene descripción. El archivo original no SHALL modificarse.

#### Scenario: Con autor
- **WHEN** se importa con la firma activa y el autor "Ana López" en los ajustes
- **THEN** junto a cada foto queda un `.xmp` con ese autor y la foto conserva su contenido original

### Requirement: Progreso y errores por etapa
El import SHALL mostrar su avance (archivo actual, total y tiempo restante) mientras copia, procesa y ordena en el disco. Un archivo que falle SHALL contarse como fallido sin detener a los demás, y su error SHALL decir en qué etapa falló: al leer la tarjeta, en la tablet o en el disco.

#### Scenario: Tarjeta con un archivo dañado
- **WHEN** un archivo de la tarjeta no se puede leer a mitad de la copia
- **THEN** el resultado lo cuenta como fallido con un error que indica que falló la lectura de la tarjeta, y el resto de las fotos se importa

### Requirement: Import y candado
El import SHALL tomar el candado del proyecto mientras dura y no SHALL empezar si el proyecto lo tiene otro dispositivo; en ese caso SHALL pedir abrir el proyecto y tomar el control antes de importar.

#### Scenario: Proyecto tomado
- **WHEN** el fotógrafo intenta importar en un proyecto que tiene abierto el desktop "estudio-pc"
- **THEN** el import no empieza y se indica que el proyecto está abierto en "estudio-pc"
