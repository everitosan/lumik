# Spec Delta

## ADDED Requirements

### Requirement: Reintentar archivos fallidos
Cuando el resultado del import tiene archivos fallidos que se pueden reintentar, el paso de confirmación SHALL ofrecer un botón para reintentarlos todos a la vez, en el mismo proyecto y con los mismos ajustes de la importación. El reintento SHALL procesar solo esos archivos, y SHALL estar disponible otra vez si alguno vuelve a fallar.

#### Scenario: Fallo pasajero de metadatos
- **WHEN** en una importación de 40 fotos 3 fallan por metadatos y el usuario pulsa "Reintentar"
- **THEN** se procesan solo esas 3, con el mismo renombrado y la misma firma, y las que entran quedan en el proyecto como cualquier otra foto importada

#### Scenario: Sin fallidos
- **WHEN** todos los archivos se importaron
- **THEN** no se muestra el botón de reintentar

#### Scenario: Vuelve a fallar
- **WHEN** un archivo reintentado vuelve a fallar
- **THEN** sigue en la lista de fallidos con su motivo y el botón de reintentar sigue disponible

### Requirement: Reintento según el motivo del fallo
Un archivo que falló al copiarse, prepararse o firmarse SHALL reintentarse desde su archivo de origen. Un archivo que falló por falta de espacio SHALL reintentarse solo si el disco del proyecto tiene ahora espacio suficiente. Un archivo que llegó a `_media/` pero no se pudo registrar SHALL reintentarse registrándolo desde `_media/`, sin volver a copiarlo, sin borrarlo y sin duplicarlo.

#### Scenario: Origen que no se pudo leer
- **WHEN** una foto falló porque no se pudo leer del origen y el origen vuelve a estar disponible
- **THEN** el reintento la copia desde el origen y la importa

#### Scenario: Sigue sin espacio
- **WHEN** hay fallidos por falta de espacio y el disco del proyecto sigue sin espacio para ellos
- **THEN** el reintento no se inicia y se indica el espacio que falta

#### Scenario: Fallo al registrar
- **WHEN** las fotos se movieron a `_media/` pero falló su registro en el proyecto, y el usuario reintenta
- **THEN** se registran los archivos que ya están en `_media/`, con sus miniaturas, sin crear copias nuevas ni sufijos `-001`

#### Scenario: Archivo ya registrado o desaparecido
- **WHEN** al reintentar el registro un archivo ya está registrado en el proyecto o ya no está en `_media/`
- **THEN** no se registra dos veces; el ya registrado cuenta como importado y el desaparecido sigue como fallido

### Requirement: Resultado acumulado tras reintentar
Tras un reintento, el resultado SHALL mostrar el total de la sesión: las fotos importadas y los videos copiados en todos los intentos, los archivos que siguen fallidos con su motivo y los ignorados. El registro de actividad SHALL conservar las entradas de los intentos anteriores y agregar las del reintento.

#### Scenario: Reintento exitoso
- **WHEN** se importaron 37 de 40 fotos y el reintento importa las 3 restantes
- **THEN** el resultado indica 40 fotos importadas, sin fallidos, y el registro conserva las entradas del primer intento

#### Scenario: Reintento parcial
- **WHEN** se importaron 37 de 40 fotos y el reintento importa 2 de las 3
- **THEN** el resultado indica 39 importadas y una fallida con su motivo
