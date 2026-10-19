# Spec Delta

## Purpose

Respaldar en las nubes del fotógrafo las fotos finales de cada proyecto (la carpeta `_exported/`), en segundo plano, de forma verificable y sin duplicar archivos ni carpetas.

## ADDED Requirements

### Requirement: Unidad de respaldo
Lo que se respalda SHALL ser cada archivo de imagen de `_exported/` del proyecto. Los archivos ocultos SHALL ignorarse. Los RAW, `_media/` y `_culled/` no SHALL respaldarse.

#### Scenario: Carpeta con archivos ocultos
- **WHEN** `_exported/` contiene `a.jpg`, `b.tif` y `.DS_Store`
- **THEN** se ofrecen para respaldar `a.jpg` y `b.tif`

### Requirement: Ruta en la nube
Cada archivo SHALL subirse a `Lumik/{año}/{mes}/{día}_{slug}/_exported/{archivo}` en cada proveedor, con la fecha de sesión (o la de creación si no hay). La ruta de un proyecto SHALL fijarse en su primer respaldo y reutilizarse en los siguientes. Antes de subir en paralelo, la carpeta destino SHALL crearse una sola vez por proveedor, para que proveedores que admiten nombres repetidos, como Mega, no la dupliquen.

#### Scenario: Primer respaldo
- **WHEN** se respalda el proyecto "Boda" del 2024-06-15
- **THEN** los archivos quedan en `Lumik/2024/06/15_Boda/_exported/` en cada proveedor elegido

#### Scenario: Subida paralela a Mega
- **WHEN** se suben 20 archivos en paralelo a Mega
- **THEN** existe una sola carpeta `_exported` para el proyecto

### Requirement: Modal de respaldo
El desktop SHALL ofrecer un modal por proyecto que liste los archivos de `_exported/` aún no subidos y deje elegir entre los proveedores conectados (todos por defecto) y la calidad: original, alta (lado mayor de 3840 px, JPEG 90), media (2048 px, 82) o baja (1280 px, 75). Sin proveedores conectados, el servidor SHALL rechazar la subida.

#### Scenario: Elegir calidad media
- **WHEN** el fotógrafo respalda con calidad media un JPEG de 6000 px
- **THEN** en la nube queda un JPEG de 2048 px en su lado mayor

#### Scenario: Calidad original
- **WHEN** el fotógrafo respalda con calidad original
- **THEN** el archivo sube sin recomprimir

#### Scenario: Sin proveedores
- **WHEN** el fotógrafo no tiene proveedores conectados e intenta respaldar
- **THEN** la subida se rechaza indicando que no hay proveedores conectados

### Requirement: Subida en segundo plano con progreso
El respaldo SHALL ejecutarse en segundo plano con hasta 6 subidas en paralelo, sin congelar la interfaz. El fotógrafo SHALL poder cerrar el modal, seguir usando la app y volver a abrirlo para ver el progreso de cada archivo. Un archivo que ya se está subiendo no SHALL lanzarse otra vez.

#### Scenario: Cerrar y reabrir el modal
- **WHEN** el fotógrafo cierra el modal a mitad de la subida y lo vuelve a abrir
- **THEN** ve el estado de cada archivo (subiendo, subido o fallido) y la subida no se reinicia

### Requirement: Fan-out a varios proveedores
Cada archivo SHALL subirse a todos los proveedores elegidos. El archivo SHALL quedar como `uploaded` solo si llegó a todos; si falló en alguno, SHALL quedar como `failed` y SHALL incrementarse su contador de reintentos.

#### Scenario: Falla un proveedor
- **WHEN** un archivo sube a Nextcloud pero falla en Mega
- **THEN** el archivo queda como `failed` y el resultado indica qué proveedor falló

### Requirement: Integridad verificable
Por cada archivo subido, el servidor SHALL registrar el SHA-256 y el tamaño de los bytes que realmente se subieron, ya recomprimidos si aplica. La copia a la nube SHALL verificar el hash entre origen y destino.

#### Scenario: Consultar integridad
- **WHEN** un archivo queda como `uploaded`
- **THEN** el catálogo guarda su SHA-256 y su tamaño en bytes

### Requirement: Re-subidas sin duplicados
Volver a subir un archivo ya respaldado SHALL sobrescribirlo en la misma ruta, sin crear copias.

#### Scenario: Reintentar un respaldo
- **WHEN** se vuelve a respaldar un archivo que ya estaba subido
- **THEN** en la nube sigue habiendo un solo archivo con ese nombre

### Requirement: Cola offline-first
Al iniciar un respaldo, el desktop SHALL guardar la intención (proyecto, proveedores, calidad) en una cola persistente y SHALL quitarla al terminar. Al arrancar la app, SHALL reanudar cada respaldo en cola con los archivos que falten. Si el disco no está montado o no hay red, el respaldo SHALL seguir en cola hasta el siguiente arranque.

#### Scenario: Cerrar la app a mitad de respaldo
- **WHEN** el fotógrafo cierra Lumik con un respaldo a medias y la vuelve a abrir con el disco conectado
- **THEN** el respaldo se reanuda con los archivos que faltaban

#### Scenario: Disco ausente al arrancar
- **WHEN** la app arranca con un respaldo en cola y sin el disco del proyecto
- **THEN** el respaldo sigue en cola y no se muestra ningún error

### Requirement: Estado del proyecto tras respaldar
Tras un respaldo correcto, el proyecto SHALL quedar como `delivered` ("Respaldado") si `_exported/` tiene al menos tantas fotos como `_culled/`, y como `edited` ("Editando") si tiene menos. El estado y los proveedores usados SHALL guardarse y publicarse en el catálogo, y cada tarjeta de proyecto SHALL mostrar los iconos de esos proveedores.

#### Scenario: Todas las aprobadas exportadas
- **WHEN** un proyecto tiene 40 fotos aprobadas y 40 exportadas, y el respaldo termina bien
- **THEN** el proyecto se muestra como "Respaldado" con los iconos de sus proveedores

#### Scenario: Exportación parcial
- **WHEN** un proyecto tiene 40 aprobadas y 25 exportadas, y el respaldo termina bien
- **THEN** el proyecto se muestra como "Editando"

### Requirement: Renombrar mueve la carpeta en la nube
Al renombrar un proyecto, el desktop SHALL pedir en segundo plano que su carpeta en la nube se mueva a la ruta del nuevo nombre en todos los proveedores, actualizando las rutas guardadas de sus fotos. Si el proyecto aún no tiene respaldo o la ruta no cambia, no SHALL hacerse nada. Un fallo no SHALL impedir el renombrado local.

#### Scenario: Renombrar un proyecto respaldado
- **WHEN** el proyecto "Boda" ya respaldado se renombra a "Boda Ana"
- **THEN** en la nube la carpeta pasa a `Lumik/2024/06/15_Boda Ana/_exported/` sin partirse en dos

### Requirement: Abrir la carpeta en el proveedor
Desde el icono de un proveedor en la tarjeta, se SHALL generar un enlace a la carpeta del proyecto que caduca en 7 días, pero solo en proveedores que admiten enlaces con caducidad (Dropbox, OneDrive). En los demás SHALL abrirse la web del proveedor. Nunca SHALL crearse un enlace público permanente, ni guardarse o escribirse en logs.

#### Scenario: Proveedor sin enlaces con caducidad
- **WHEN** el fotógrafo pulsa el icono de Mega
- **THEN** se abre la web de Mega y no se crea ningún enlace público
