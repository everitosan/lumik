# Spec Delta

## Purpose

La sección "Papelera" reúne todo lo que el fotógrafo mandó a la papelera en los discos conectados (proyectos y fotos sueltas), y es el único lugar desde donde se restaura o se elimina definitivamente.

## ADDED Requirements

### Requirement: Sección "Papelera" en la navegación
El menú izquierdo del desktop y el rail de la tablet SHALL ofrecer la entrada "Papelera" inmediatamente antes de la de configuración ("Configuración" en el desktop, "Ajustes" en la tablet), sin importar qué otras entradas haya. La entrada SHALL indicar cuántos elementos hay en la papelera de los discos conectados.

#### Scenario: Abrir la papelera en el desktop
- **WHEN** el usuario elige "Papelera" en el menú izquierdo
- **THEN** se muestra la sección con lo que hay en la papelera de los discos conectados

#### Scenario: Posición en la tablet
- **WHEN** la tablet tiene algo en la papelera de un disco conectado
- **THEN** la entrada "Papelera" aparece justo antes de "Ajustes"

#### Scenario: Conteo en la entrada
- **WHEN** hay un proyecto y 3 fotos sueltas en la papelera de los discos conectados
- **THEN** la entrada "Papelera" indica 4

### Requirement: La entrada solo existe si hay algo en la papelera
La entrada "Papelera" SHALL mostrarse solo si algún disco conectado tiene algo en la papelera, y SHALL desaparecer en cuanto ya no haya nada. Si desaparece mientras el usuario está en la sección, la app SHALL llevarlo a la sección de proyectos.

#### Scenario: Nada en la papelera
- **WHEN** ningún disco conectado tiene nada en la papelera
- **THEN** el menú no muestra la entrada "Papelera"

#### Scenario: Primer elemento
- **WHEN** el usuario manda a la papelera una foto y no había nada en la papelera
- **THEN** la entrada "Papelera" aparece en el menú

#### Scenario: Vaciar lo último
- **WHEN** el usuario está en la sección y elimina o restaura el último elemento
- **THEN** la entrada desaparece y la app muestra la sección de proyectos

#### Scenario: Expulsar el único disco con papelera
- **WHEN** el usuario expulsa el único disco conectado que tenía algo en la papelera
- **THEN** la entrada "Papelera" desaparece

### Requirement: Contenido agrupado por disco conectado
La sección SHALL agrupar su contenido por disco, con un grupo por cada disco conectado que tenga algo en la papelera. Un disco sin nada en la papelera no SHALL tener grupo. Dentro de cada grupo SHALL listar primero los proyectos en la papelera y después las fotos sueltas. Lo que esté en la papelera de un disco desconectado no SHALL mostrarse.

#### Scenario: Dos discos conectados
- **WHEN** KINGSTON tiene un proyecto en la papelera y SEAGATE tiene 2 fotos sueltas
- **THEN** la sección muestra el grupo KINGSTON con el proyecto y el grupo SEAGATE con las 2 fotos

#### Scenario: Disco desconectado
- **WHEN** el usuario expulsa KINGSTON
- **THEN** el grupo KINGSTON desaparece de la sección y su papelera no se toca

#### Scenario: Disco sin papelera
- **WHEN** KINGSTON tiene un proyecto en la papelera y SEAGATE no tiene nada en la papelera
- **THEN** la sección muestra solo el grupo KINGSTON

### Requirement: Qué muestra cada elemento
Cada proyecto en la papelera SHALL mostrarse con su portada, su nombre, su fecha de sesión, su número de fotos y la fecha en que se mandó a la papelera. Cada foto suelta SHALL mostrarse con su miniatura, su nombre de archivo y el nombre de su proyecto. Las fotos de un proyecto que está en la papelera no SHALL listarse como fotos sueltas.

#### Scenario: Foto de un proyecto en la papelera
- **WHEN** un proyecto con 2 fotos en su propia papelera se manda a la papelera
- **THEN** la sección lo muestra como un proyecto y no lista esas 2 fotos aparte

#### Scenario: Foto suelta
- **WHEN** se manda a la papelera `IMG_0001.cr2` del proyecto "Boda"
- **THEN** la sección la muestra con su miniatura, `IMG_0001.cr2` y "Boda"

### Requirement: Restaurar desde la sección
Cada elemento SHALL ofrecer "Restaurar", sin confirmación. Restaurar un proyecto SHALL devolverlo al dashboard. Restaurar una foto SHALL devolverla a su proyecto. En ambos casos el elemento SHALL salir de la sección.

#### Scenario: Restaurar un proyecto
- **WHEN** el usuario restaura el proyecto "Risespray"
- **THEN** "Risespray" vuelve a listarse en el dashboard y deja de verse en la papelera

#### Scenario: Restaurar una foto
- **WHEN** el usuario restaura `IMG_0001.cr2` de "Boda"
- **THEN** la foto vuelve al grid de "Boda" y deja de verse en la papelera

### Requirement: Colisión al restaurar
Si el lugar al que vuelve un elemento ya está ocupado, restaurar no SHALL mover nada: SHALL mostrarse un modal que explique qué ocupa ese lugar y pida otro nombre (de proyecto o de archivo), con las acciones "Cancelar" y "Restaurar". El modal no SHALL aceptar un nombre vacío ni uno que también esté ocupado. Cancelar no SHALL cambiar nada.

#### Scenario: Proyecto con el lugar ocupado
- **WHEN** se restaura "Risespray" y ya existe `lumik/2026/09/18_Risespray`
- **THEN** se muestra el modal de colisión y nada se mueve hasta que el usuario elige otro nombre

#### Scenario: Restaurar con otro nombre
- **WHEN** en el modal de colisión el usuario escribe "Risespray 2" y confirma
- **THEN** el proyecto se restaura como "Risespray 2", en su carpeta correspondiente, y sale de la papelera

#### Scenario: Nombre también ocupado
- **WHEN** en el modal el usuario escribe un nombre cuyo destino también existe
- **THEN** el modal lo indica y no permite confirmar

#### Scenario: Cancelar la colisión
- **WHEN** el usuario cierra el modal de colisión con "Cancelar"
- **THEN** el elemento sigue en la papelera sin cambios

### Requirement: Eliminar definitivamente un elemento
Cada elemento SHALL ofrecer "Eliminar definitivamente". Antes SHALL mostrarse un modal de confirmación que nombre el elemento, diga que no se puede deshacer y diga que el respaldo en la nube no se toca y se borra desde el portal web. Al confirmar, el elemento SHALL borrarse del disco y de los registros sin dejar rastro, y salir de la sección.

#### Scenario: Eliminar un proyecto
- **WHEN** el usuario confirma eliminar definitivamente el proyecto "Risespray"
- **THEN** su carpeta desaparece del disco y el proyecto deja de verse en la papelera

#### Scenario: Cancelar
- **WHEN** el usuario cierra el modal de eliminar con "Cancelar"
- **THEN** el elemento sigue en la papelera

#### Scenario: Aviso sobre el respaldo
- **WHEN** se abre el modal de eliminar definitivamente
- **THEN** el texto dice que el respaldo en la nube se conserva y que se borra desde el portal web

### Requirement: Vaciar la papelera de un disco
Cada grupo SHALL ofrecer "Vaciar papelera de {disco}", con un modal de confirmación que diga cuántos proyectos y fotos se eliminarán, que no se puede deshacer y que el respaldo en la nube no se toca. Al confirmar SHALL eliminarse definitivamente todo lo que esa sección muestra para ese disco, y nada de otros discos.

#### Scenario: Vaciar un disco
- **WHEN** el usuario vacía la papelera de KINGSTON, que tiene 1 proyecto y 3 fotos
- **THEN** se eliminan el proyecto y las 3 fotos, y la papelera de SEAGATE no cambia

#### Scenario: Elemento que no se pudo eliminar
- **WHEN** al vaciar la papelera de un disco una foto no se puede eliminar
- **THEN** se eliminan los demás elementos, esa foto sigue en la papelera y se informa cuál falló y por qué

### Requirement: Proyectos que otro dispositivo tiene abiertos
Restaurar o eliminar definitivamente una foto de un proyecto que otro dispositivo tiene en uso SHALL rechazarse con un aviso de quién lo tiene, como cualquier otra edición de ese proyecto. Al vaciar la papelera, esas fotos SHALL quedarse en la papelera y reportarse como no eliminadas.

#### Scenario: Foto de un proyecto en uso en la tablet
- **WHEN** el desktop intenta eliminar definitivamente una foto de un proyecto que la tablet tiene abierto
- **THEN** la foto no se elimina y se informa que el proyecto está en uso en la tablet

### Requirement: La papelera no se vacía sola
El sistema no SHALL eliminar definitivamente nada de la papelera sin una acción explícita del usuario en esta sección, sin importar cuánto tiempo lleve ahí.

#### Scenario: Elemento antiguo
- **WHEN** una foto lleva un año en la papelera
- **THEN** sigue en la papelera hasta que el usuario la restaure o la elimine
