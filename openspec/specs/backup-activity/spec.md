# backup-activity Specification

## Purpose

Permite seguir desde cualquier pantalla del desktop los respaldos que se están ejecutando y actuar sobre los que fallaron, para no perder de vista ni cortar sin querer una subida en segundo plano.

## Requirements

### Requirement: Sección de respaldos en la barra lateral
La barra lateral del desktop SHALL mostrar una sección "Respaldos" mientras haya algún respaldo en curso o terminado con errores, desde cualquier pantalla de la app. Sin ningún respaldo que mostrar, la sección SHALL ocultarse por completo. La visibilidad SHALL depender solo de los respaldos, no de la sesión.

#### Scenario: Fuera del proyecto
- **WHEN** el fotógrafo lanza un respaldo y vuelve al dashboard
- **THEN** la sección "Respaldos" de la barra lateral muestra ese respaldo en curso

#### Scenario: Sin respaldos
- **WHEN** no hay ningún respaldo en curso ni con errores
- **THEN** la barra lateral no muestra la sección "Respaldos"

#### Scenario: Termina el último respaldo
- **WHEN** el único respaldo visible termina con todos sus archivos subidos
- **THEN** su tarjeta desaparece y la sección "Respaldos" se oculta

#### Scenario: Cerrar sesión a mitad de un respaldo
- **WHEN** el fotógrafo cierra sesión en Ajustes mientras "Boda Ana" se está respaldando
- **THEN** la sección "Respaldos" sigue visible con la tarjeta de "Boda Ana"

### Requirement: Tarjeta por respaldo en curso
Cada respaldo en curso SHALL mostrarse en una tarjeta con el nombre de su proyecto y su avance como archivos subidos sobre el total, también cuando el respaldo se reanudó al arrancar la app. La sección SHALL mostrar solo las tarjetas, sin datos de la cuenta.

#### Scenario: Dos respaldos a la vez
- **WHEN** se respaldan "Boda Ana" (1 de 20 archivos subidos) y "Risespray" (3 de 23)
- **THEN** la sección muestra dos tarjetas: "Boda Ana 1/20" y "Risespray 3/23"

#### Scenario: Respaldo reanudado al arrancar
- **WHEN** la app arranca y reanuda un respaldo en cola del proyecto "Boda Ana"
- **THEN** su tarjeta muestra el nombre "Boda Ana" y su avance

### Requirement: Detalle desde la tarjeta
Pulsar una tarjeta SHALL abrir el modal de progreso de ese respaldo, con el estado de cada archivo, sin salir de la pantalla actual.

#### Scenario: Abrir el detalle desde Ajustes
- **WHEN** el fotógrafo está en Ajustes y pulsa la tarjeta de "Boda Ana"
- **THEN** se abre el modal con cada archivo de ese respaldo y su estado, y al cerrarlo sigue en Ajustes

### Requirement: Fin de un respaldo
Un respaldo que termina con todos sus archivos subidos SHALL desaparecer de la sección. Uno que termina con algún archivo fallido SHALL quedarse visible, indicando cuántos archivos fallaron.

#### Scenario: Termina sin errores
- **WHEN** termina el respaldo de "Boda Ana" con 20 de 20 archivos subidos
- **THEN** su tarjeta desaparece de la sección

#### Scenario: Termina con errores
- **WHEN** termina el respaldo de "Risespray" con 21 de 23 archivos subidos
- **THEN** su tarjeta sigue visible e indica que 2 archivos fallaron

### Requirement: Reintentar un respaldo con errores
Una tarjeta con errores y el modal de su detalle SHALL ofrecer "Reintentar", que vuelve a subir en ese momento solo los archivos fallidos, con los mismos proveedores y calidad. Si el disco del proyecto no está montado, SHALL indicarlo y no SHALL intentar la subida.

#### Scenario: Reintentar con el disco conectado
- **WHEN** el fotógrafo pulsa "Reintentar" en la tarjeta de "Risespray" con 2 archivos fallidos
- **THEN** se suben de nuevo solo esos 2 archivos y la tarjeta muestra su avance

#### Scenario: Reintentar sin el disco
- **WHEN** el fotógrafo pulsa "Reintentar" y el disco de "Risespray" no está conectado
- **THEN** se indica que hay que conectar ese disco y no se sube nada

### Requirement: Descartar un respaldo con errores
Una tarjeta con errores SHALL ofrecer "Descartar", que la quita de la sección sin cancelar el respaldo: el proyecto SHALL seguir en la cola persistente y reintentarse en el siguiente arranque.

#### Scenario: Descartar y reiniciar
- **WHEN** el fotógrafo descarta la tarjeta de "Risespray" y reinicia la app con el disco conectado
- **THEN** el respaldo de "Risespray" se reanuda con los archivos que faltaban y vuelve a aparecer su tarjeta

### Requirement: Confirmación al cerrar con respaldos en curso
Si hay algún respaldo subiendo archivos, cerrar la ventana de la app SHALL pedir confirmación, indicando cuántos respaldos están en curso. Si el fotógrafo cancela, la app SHALL seguir abierta y las subidas SHALL continuar. Sin respaldos en curso, la app SHALL cerrarse sin preguntar.

#### Scenario: Cerrar a mitad de un respaldo
- **WHEN** el fotógrafo cierra la ventana mientras "Boda Ana" se está respaldando y cancela la confirmación
- **THEN** la app sigue abierta y la subida continúa

#### Scenario: Confirmar el cierre
- **WHEN** el fotógrafo cierra la ventana con un respaldo en curso y confirma
- **THEN** la app se cierra y el respaldo queda en la cola para reanudarse al arrancar

#### Scenario: Sin respaldos en curso
- **WHEN** solo queda una tarjeta con errores y el fotógrafo cierra la ventana
- **THEN** la app se cierra sin pedir confirmación
