# Spec Delta

## Purpose
Garantiza que un proyecto del disco externo lo modifique un solo dispositivo a la vez, y que el fotógrafo sepa quién lo tiene y si el disco está al día antes de tomarlo.

## ADDED Requirements

### Requirement: Candado al trabajar en un proyecto
Mientras un dispositivo trabaja en un proyecto (lo tiene abierto o corre sobre él una importación, un respaldo o un cambio de entrega), SHALL mantener en la carpeta del proyecto un candado con su identidad, su nombre legible, la hora de inicio y si el disco tiene ya todos sus cambios. Al terminar, al expulsar el disco o al cerrar la app, SHALL soltarlo. Listar el proyecto en el dashboard no SHALL tomar el candado.

#### Scenario: Abrir y salir
- **WHEN** el fotógrafo abre un proyecto libre y luego vuelve al dashboard sin operaciones en curso
- **THEN** el candado se escribe al abrir y se quita al volver

#### Scenario: Importación en curso
- **WHEN** el fotógrafo vuelve al dashboard mientras sigue una importación al proyecto
- **THEN** el candado sigue hasta que termina la importación

### Requirement: Proyecto tomado por otro dispositivo
Si el candado de un proyecto es de otro dispositivo, abrirlo SHALL mostrarlo en solo lectura con un aviso que diga qué dispositivo lo tiene y desde cuándo. En solo lectura SHALL poder verse todo y no SHALL poder cambiarse nada: ni archivos ni base de datos. El aviso SHALL ofrecer "Tomar el control".

#### Scenario: Disco que viene de la tablet
- **WHEN** el fotógrafo abre en el desktop un proyecto cuyo candado es de "Tablet de Ana" desde las 10:02
- **THEN** ve el proyecto en solo lectura con el aviso "Abierto en Tablet de Ana desde las 10:02" y el botón "Tomar el control"

#### Scenario: Intentar aprobar en solo lectura
- **WHEN** el proyecto está en solo lectura y el fotógrafo intenta aprobar una foto
- **THEN** la foto no se aprueba y el aviso sigue visible

### Requirement: Aviso según el estado del disco
Si el candado indica que el disco tiene todos los cambios del otro dispositivo, el aviso SHALL ser informativo. Si indica cambios que no llegaron al disco, el aviso SHALL advertir que tomar el control los deja fuera.

#### Scenario: Disco al día
- **WHEN** el candado de otro dispositivo marca el disco al día
- **THEN** el aviso dice que el disco está al día

#### Scenario: Cambios sin devolver
- **WHEN** el candado de otro dispositivo marca cambios pendientes
- **THEN** el aviso advierte que esos cambios no están en el disco antes de tomar el control

### Requirement: Tomar el control
"Tomar el control" SHALL reemplazar el candado por uno del dispositivo actual y salir de solo lectura. El dispositivo que perdió el candado, al volver a ver el proyecto, no SHALL escribir sobre la base del disco: SHALL tratarlo como tomado por otro.

#### Scenario: Candado huérfano
- **WHEN** el disco se desconectó de la tablet sin cerrar el proyecto y el fotógrafo pulsa "Tomar el control" en el desktop
- **THEN** el desktop trabaja en el proyecto con normalidad

#### Scenario: La tablet vuelve
- **WHEN** el desktop tomó el control y después la tablet vuelve a abrir el proyecto
- **THEN** la tablet lo muestra tomado por el desktop y no cambia la base del disco

### Requirement: Candado propio
Si el candado es del mismo dispositivo, por ejemplo tras un cierre inesperado, abrir el proyecto SHALL retomarlo sin aviso.

#### Scenario: Tras un cierre inesperado
- **WHEN** la app se cerró de golpe con un proyecto abierto y se vuelve a abrir en el mismo dispositivo
- **THEN** el proyecto se abre con normalidad, sin aviso

### Requirement: Marca en el dashboard
El dashboard SHALL marcar las tarjetas de los proyectos cuyo candado es de otro dispositivo, con su nombre. Las acciones de la tarjeta que modifican el proyecto (editar, archivar, eliminar) SHALL pedir tomar el control antes.

#### Scenario: Tarjeta tomada
- **WHEN** el dashboard lista un proyecto tomado por "iPad de Ana"
- **THEN** su tarjeta muestra "En iPad de Ana" y editarla pide tomar el control

### Requirement: Identidad de la instalación
Cada instalación de Lumik SHALL tener una identidad estable y un nombre legible, tomado del nombre del equipo o del dispositivo, que el usuario SHALL poder cambiar en Ajustes.

#### Scenario: Nombre por defecto
- **WHEN** Lumik se abre por primera vez en un equipo llamado "estudio-pc"
- **THEN** los candados de esa instalación muestran "estudio-pc"
