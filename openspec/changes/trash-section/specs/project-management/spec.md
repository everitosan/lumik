# Spec Delta

## MODIFIED Requirements

### Requirement: Descubrimiento de proyectos en dispositivos
El sistema SHALL listar los proyectos encontrados en los dispositivos montados y SHALL dejar de listar los de dispositivos desmontados o en proceso de expulsión. Cada dispositivo visto SHALL registrarse en el registro global de dispositivos conocidos. Los proyectos que estén bajo `lumik/_trash/` están en la papelera y no SHALL listarse en el dashboard, ni en el desktop ni en la tablet.

#### Scenario: Conectar un disco con proyectos
- **WHEN** se monta un disco que contiene proyectos Lumik
- **THEN** esos proyectos aparecen en el dashboard sin necesidad de importarlos

#### Scenario: Desconectar el disco
- **WHEN** el disco se desmonta o se está expulsando
- **THEN** sus proyectos dejan de aparecer en el dashboard

#### Scenario: Disco con proyectos en la papelera
- **WHEN** se conecta un disco con `lumik/_trash/2026-11-10_19-54-03/2026/09/18_Risespray/project.db`
- **THEN** "Risespray" no aparece en el dashboard y sí en la sección "Papelera"

### Requirement: Orden del dashboard
El dashboard SHALL agrupar los proyectos en secciones por el año de su fecha de sesión o, si no la tienen, de su fecha de creación. Las secciones y los proyectos de cada sección SHALL ordenarse en la dirección elegida por el usuario. Dentro de una sección, los proyectos SHALL ordenarse por esa misma fecha y desempatarse por fecha de creación en la misma dirección. Los proyectos en la papelera SHALL quedar fuera del dashboard, y los archivados SHALL mostrarse u ocultarse según la elección del usuario.

#### Scenario: Mezcla de proyectos con y sin fecha
- **WHEN** existe un proyecto sin fecha de sesión creado el 2026-05-20 y proyectos con fecha de sesión en 2026
- **THEN** el proyecto sin fecha aparece en la sección 2026, ordenado por su fecha de creación junto a los demás

#### Scenario: Misma fecha de sesión
- **WHEN** dos proyectos comparten fecha de sesión y el orden de proyectos es de más antiguo a más reciente
- **THEN** aparece primero el creado antes

#### Scenario: Archivados mostrados en orden
- **WHEN** se muestran los archivados y un proyecto archivado es del 2026-03-02, entre dos activos del 2026-01-10 y del 2026-08-10
- **THEN** dentro de la sección 2026 aparece entre ambos, en el orden elegido

## ADDED Requirements

### Requirement: Mandar un proyecto a la papelera
El menú del proyecto SHALL ofrecer "Mover a la papelera" en lugar de "Eliminar", en el desktop y en la tablet. Antes SHALL mostrarse un modal de confirmación que nombre el proyecto y diga que se podrá restaurar. Al confirmar, el proyecto SHALL salir del dashboard sin que se borre ningún archivo. La operación SHALL requerir que el disco del proyecto esté montado.

#### Scenario: Confirmar
- **WHEN** el usuario elige "Mover a la papelera" en "Risespray" y confirma
- **THEN** "Risespray" deja de verse en el dashboard y aparece en la sección "Papelera"

#### Scenario: Cancelar
- **WHEN** el usuario cierra el modal con "Cancelar"
- **THEN** el proyecto sigue en el dashboard y nada se mueve

#### Scenario: Proyecto en uso en otro dispositivo
- **WHEN** el usuario intenta mandar a la papelera un proyecto que la tablet tiene en uso
- **THEN** el proyecto no se mueve y se informa que está en uso en la tablet

### Requirement: Ubicación de un proyecto en la papelera
Mandar un proyecto a la papelera SHALL mover su carpeta completa, en el mismo disco, de `lumik/{ruta}` a `lumik/_trash/{AAAA-MM-DD_HH-MM-SS}/{ruta}`, donde la fecha y hora son las del momento en que se mandó. El nombre y el contenido de la carpeta no SHALL cambiar, incluida su propia papelera de fotos, para que mandar y restaurar sean un solo movimiento.

#### Scenario: Proyecto con estructura por fecha
- **WHEN** el 2026-11-10 a las 19:54:03 se manda a la papelera `lumik/2026/09/18_Risespray`
- **THEN** la carpeta queda en `lumik/_trash/2026-11-10_19-54-03/2026/09/18_Risespray`

#### Scenario: Proyecto con ruta plana
- **WHEN** se manda a la papelera `lumik/Emma Arenas`
- **THEN** la carpeta queda en `lumik/_trash/{fecha}/Emma Arenas`

#### Scenario: Mismo proyecto dos veces
- **WHEN** se manda a la papelera un proyecto, se crea otro con el mismo nombre y fecha de sesión y también se manda a la papelera
- **THEN** los dos quedan en `lumik/_trash/` bajo carpetas de fecha distintas, sin sobrescribirse

### Requirement: Proyectos en la papelera con el formato anterior
Las carpetas del formato anterior, `lumik/_trash/{ruta}__{AAAA-MM-DD_HH-MM-SS}`, SHALL seguir listándose en la sección "Papelera" con su fecha, y restaurarse a `lumik/{ruta}`.

#### Scenario: Restaurar del formato anterior
- **WHEN** el disco tiene `lumik/_trash/2026/09/18_Boda__2026-11-10_08-00-00` y se restaura
- **THEN** la sección "Papelera" lo lista mandado el 2026-11-10 a las 08:00 y vuelve a `lumik/2026/09/18_Boda`

### Requirement: Avance al mover un proyecto en la tablet
En la tablet, mientras la carpeta de un proyecto se mueve a la papelera o de vuelta, SHALL mostrarse un modal con una animación de avance (una carpeta va de un ícono al otro) y los segundos transcurridos. El modal no SHALL poder cerrarse hasta que termine, y al terminar SHALL mostrar una marca de hecho. Mandar y restaurar SHALL usar el mismo modal; restaurar lo abre ya moviendo, sin confirmación.

#### Scenario: Mandar un proyecto grande
- **WHEN** en la tablet el usuario confirma mandar a la papelera un proyecto con más de 1 000 fotos
- **THEN** el modal muestra la animación y los segundos transcurridos hasta que termina, y luego la marca de hecho

#### Scenario: Restaurar
- **WHEN** el usuario toca "Restaurar" en un proyecto de la sección "Papelera"
- **THEN** se abre el mismo modal, con la carpeta yendo de la papelera al proyecto, hasta que termina

### Requirement: Restaurar un proyecto
Restaurar un proyecto SHALL devolver su carpeta a la ruta que tenía antes de la papelera, sin la carpeta de la fecha. Si esa ruta está ocupada y el usuario elige otro nombre en el modal de colisión, el proyecto SHALL restaurarse con ese nombre: su carpeta y los archivos de sus fotos SHALL nombrarse como al editar el nombre del proyecto.

#### Scenario: Ruta libre
- **WHEN** se restaura `lumik/_trash/2026-11-10_19-54-03/2026/09/18_Risespray` y `lumik/2026/09/18_Risespray` no existe
- **THEN** la carpeta vuelve a `lumik/2026/09/18_Risespray` y el proyecto se lista en el dashboard

#### Scenario: Restaurar con otro nombre
- **WHEN** la ruta original está ocupada y el usuario restaura con el nombre "Risespray 2"
- **THEN** el proyecto queda en `lumik/2026/09/18_Risespray 2`, se llama "Risespray 2" y sus fotos se renombran como al editar el nombre

### Requirement: Eliminar definitivamente un proyecto
Eliminar definitivamente un proyecto en la papelera SHALL borrar su carpeta y todo su contenido del disco, y quitarlo del índice de búsqueda. No SHALL tocar su respaldo en la nube. Solo SHALL poder hacerse desde la sección "Papelera".

#### Scenario: Eliminación confirmada
- **WHEN** el usuario confirma eliminar definitivamente "Risespray" desde la papelera
- **THEN** su carpeta desaparece del disco, deja de listarse en la papelera y su respaldo en la nube sigue intacto

#### Scenario: Carpetas vacías
- **WHEN** se elimina el único proyecto de `lumik/_trash/2026/09/`
- **THEN** las carpetas de mes y año que quedan vacías bajo `lumik/_trash/` también se quitan

## REMOVED Requirements

### Requirement: Eliminar proyecto
**Reason**: Borraba la carpeta del proyecto para siempre en un solo paso, contra la regla de que Lumik no borra originales sin un segundo paso explícito.
**Migration**: El menú del proyecto ofrece "Mover a la papelera" (requisito "Mandar un proyecto a la papelera"); el borrado permanente se hace desde la sección "Papelera" (requisito "Eliminar definitivamente un proyecto").
