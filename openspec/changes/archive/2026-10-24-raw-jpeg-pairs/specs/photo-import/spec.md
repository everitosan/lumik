# Spec Delta

## MODIFIED Requirements

### Requirement: Ajustes de la importación
El paso de ajustes SHALL mostrar, precargados con la configuración global, si se renombran las fotos (con un ejemplo del nombre resultante para el proyecto destino) y si se firman con los datos del fotógrafo, junto con su autor, copyright y URL editables. Lo elegido SHALL aplicarse solo a esa importación, sin modificar la configuración global. El usuario SHALL poder restablecer los valores globales. En un bloque aparte, identificado como del proyecto, el paso SHALL mostrar y permitir cambiar la opción de entrega del proyecto destino; ese cambio SHALL guardarse en el proyecto.

#### Scenario: Valores iniciales
- **WHEN** la configuración global tiene el renombrado activo y el autor "Ana Pérez"
- **THEN** el paso de ajustes muestra el renombrado activo y el autor "Ana Pérez"

#### Scenario: Cambiar el copyright para un cliente
- **WHEN** el usuario cambia el copyright a "© 2026 Cliente X" en el paso de ajustes e importa
- **THEN** las fotos de esa importación llevan "© 2026 Cliente X" y la configuración global conserva su copyright anterior

#### Scenario: Siguiente importación
- **WHEN** después de esa importación el usuario abre otra
- **THEN** el paso de ajustes vuelve a mostrar los valores de la configuración global

#### Scenario: Desactivar el renombrado solo esta vez
- **WHEN** el renombrado global está activo y el usuario lo desactiva en el paso de ajustes
- **THEN** las fotos de esa importación conservan su nombre original y el ajuste global sigue activo

#### Scenario: Restablecer
- **WHEN** el usuario cambió varios valores y pulsa restablecer
- **THEN** todos los valores del paso vuelven a los de la configuración global

#### Scenario: Resumen antes de importar
- **WHEN** el usuario llega al paso de confirmación
- **THEN** ve si las fotos se renombrarán y con qué autor se firmarán, o que no se firmarán

#### Scenario: Cambiar la entrega del proyecto desde el import
- **WHEN** el usuario cambia en el paso de ajustes la entrega del proyecto a *el JPG de los pares RAW+JPG*
- **THEN** el proyecto guarda ese valor, la siguiente importación al mismo proyecto lo muestra, y restablecer no lo cambia

### Requirement: Renombrado por fecha de captura
Si el renombrado está activo en los ajustes de la importación (por defecto, el valor global "renombrar al importar", activo de fábrica), cada foto SHALL renombrarse a `{AAAA-MM-DD}__{HH-MM-SS}__{proyecto}.{ext}` a partir de su fecha de captura, con el nombre del proyecto saneado (caracteres no alfanuméricos salvo `-` y `_` pasan a `_`). Si varias fotos comparten fecha, a partir de la segunda se SHALL añadir un sufijo `-001`, `-002`… Los dos archivos de un par SHALL recibir el mismo nombre, cada uno con su extensión.

#### Scenario: Nombres únicos
- **WHEN** se importa una foto capturada el 2024-06-15 a las 14:30:22 al proyecto "Boda Ana"
- **THEN** se guarda como `2024-06-15__14-30-22__Boda_Ana.cr2`

#### Scenario: Ráfaga en el mismo segundo
- **WHEN** dos fotos tienen la misma fecha y hora de captura
- **THEN** una conserva el nombre limpio y la otra recibe el sufijo `-001`

#### Scenario: TIFF de escáner sin fecha de captura
- **WHEN** en escritorio una foto no tiene fecha de captura
- **THEN** se usa su fecha de modificación para el nombre

#### Scenario: Renombrado desactivado
- **WHEN** el renombrado está desactivado en los ajustes de la importación
- **THEN** las fotos conservan su nombre original

#### Scenario: Ráfaga de pares
- **WHEN** se importan tres pares RAW+JPG capturados en el mismo segundo
- **THEN** cada par recibe un nombre distinto (limpio, `-001`, `-002`) y su RAW y su JPG comparten ese nombre

#### Scenario: JPG que se une a un RAW ya renombrado
- **WHEN** se importa un JPG que se une a `2024-06-15__14-30-22__Boda_Ana-001.raf` del proyecto
- **THEN** el JPG se guarda como `2024-06-15__14-30-22__Boda_Ana-001.jpg`

## ADDED Requirements

### Requirement: El import cuenta tomas
El paso de origen, el resumen y el resultado del import SHALL contar fotos por toma: un par cuenta como una foto. El paso de origen y el resumen SHALL desglosar cuántas fotos son pares RAW+JPG, RAW sueltos y JPG sueltos. El registro de actividad SHALL seguir listando cada archivo.

#### Scenario: Origen con pares
- **WHEN** el usuario selecciona 300 pares, 3 RAW sin JPG y 1 JPG sin RAW
- **THEN** el paso de origen muestra 304 fotos: 300 RAW+JPG, 3 solo RAW y 1 solo JPG

#### Scenario: Resultado
- **WHEN** esa importación termina sin fallos
- **THEN** el resultado dice "304 fotografías importadas" y el registro lista los 604 archivos
