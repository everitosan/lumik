# Spec Delta

## Purpose

Cómo la app de tablet Android llega a los discos y a los proyectos sin permisos amplios de almacenamiento: acceso por SAF a cada disco, copia de trabajo de `project.db` que vuelve al disco, y escritura segura de los archivos de Lumik en el disco.

## ADDED Requirements

### Requirement: Acceso a un disco por SAF
La app SHALL pedir acceso a cada disco una sola vez con el selector de carpetas del sistema, abierto en la raíz del disco, y SHALL conservar el permiso tras cerrar la app, reiniciar la tablet o actualizar la app. Un disco conectado sin permiso SHALL mostrarse con la acción "Dar acceso". La app no SHALL pedir el permiso de acceso a todos los archivos.

#### Scenario: Primer acceso
- **WHEN** el fotógrafo conecta un disco nuevo y toca "Dar acceso"
- **THEN** se abre el selector del sistema en la raíz de ese disco y, al aceptar, sus proyectos aparecen en el inicio

#### Scenario: Después de reiniciar
- **WHEN** el fotógrafo reinicia la tablet y vuelve a conectar un disco al que ya dio acceso
- **THEN** sus proyectos aparecen sin volver a pedir permiso

### Requirement: Lista de discos al día
La app SHALL volver a leer los discos conectados cuando Android avise que se montó o se desmontó un volumen, y al volver a primer plano. El inicio SHALL listar los proyectos de todos los discos conectados con acceso; el almacenamiento interno y las carpetas de la tarjeta de la cámara no SHALL listarse como discos.

#### Scenario: Conectar con la app abierta
- **WHEN** el fotógrafo conecta un disco con acceso mientras la app está abierta
- **THEN** sus proyectos aparecen en el inicio sin reiniciar la app

### Requirement: Copia de trabajo del proyecto
Al abrir un proyecto, la app SHALL trabajar sobre una copia de `project.db` en su almacenamiento interno, y SHALL devolverla al disco unos 3 segundos después del último cambio, al pasar a segundo plano, al salir del proyecto y al expulsar el disco. Mientras haya cambios sin devolver, el candado del proyecto SHALL indicarlo (`pending`).

#### Scenario: Calificar varias fotos
- **WHEN** el fotógrafo califica cinco fotos seguidas y deja de tocar la pantalla
- **THEN** `project.db` vuelve al disco una sola vez, unos 3 segundos después de la última calificación, y el candado deja de marcar cambios pendientes

#### Scenario: Quitar el disco sin expulsar
- **WHEN** el fotógrafo quita el disco un segundo después de calificar una foto
- **THEN** el cambio queda en la tablet y el candado del disco sigue marcando cambios pendientes

### Requirement: Devolución segura de la base
La base SHALL volver al disco escribiendo primero un temporal completo y reemplazando después la anterior, de modo que en el disco quede siempre una base íntegra. Si una devolución se interrumpe y solo queda el temporal, la app SHALL recuperarlo como `project.db` al buscar proyectos.

#### Scenario: Devolución interrumpida
- **WHEN** al buscar proyectos la carpeta de uno tiene `project.db.lumik-tmp` y no tiene `project.db`
- **THEN** el temporal pasa a ser `project.db` y el proyecto se lista

### Requirement: Archivos de Lumik sin sobrescribir
La app no SHALL sobrescribir en su lugar un archivo existente del disco. Para cambiar `.lumik-lock` o `lumik/.disk.json` SHALL escribir un archivo nuevo, borrar el anterior y renombrar el nuevo. Los archivos que crea SHALL crearse con tipo binario para que Android no les agregue una extensión.

#### Scenario: Cambiar el nombre del disco y desconectarlo
- **WHEN** el fotógrafo cambia el nombre de un disco a uno más corto y lo desconecta sin expulsar
- **THEN** al volver a conectarlo el disco se muestra con el nombre nuevo

### Requirement: Expulsar un disco
"Expulsar" SHALL cerrar lo que esté abierto en ese disco, devolver lo pendiente y soltar el candado, y SHALL ocultar el disco hasta que se desconecte o el fotógrafo lo vuelva a usar. Como Android no permite a una app desmontar un disco, SHALL avisar que ya se puede desconectar y ofrecer abrir los ajustes de almacenamiento del sistema.

#### Scenario: Expulsar con un proyecto abierto
- **WHEN** el fotógrafo expulsa el disco del proyecto que tiene abierto con cambios sin devolver
- **THEN** los cambios llegan al disco, el candado se quita y la app avisa que ya puede desconectarlo
