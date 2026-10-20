# Spec Delta

## ADDED Requirements

### Requirement: Respaldar sin sesión
Si el fotógrafo intenta respaldar un proyecto sin sesión de respaldo iniciada, el desktop SHALL mostrar un aviso que explique que hace falta una cuenta, con una acción que lleve a Ajustes → Respaldo para iniciar sesión o crear la cuenta. No SHALL mostrarse el modal de respaldo ni lanzarse ninguna subida.

#### Scenario: Pulsar Respaldar sin sesión
- **WHEN** el fotógrafo sin sesión pulsa "Respaldar" en un proyecto
- **THEN** ve un aviso de que necesita una cuenta de respaldo, sin la lista de archivos ni los proveedores

#### Scenario: Ir a iniciar sesión desde el aviso
- **WHEN** el fotógrafo pulsa la acción del aviso
- **THEN** el aviso se cierra y la app muestra Ajustes en la pestaña Respaldo, con el formulario de login

## MODIFIED Requirements

### Requirement: Subida en segundo plano con progreso
El respaldo SHALL ejecutarse en segundo plano con hasta 6 subidas en paralelo, sin congelar la interfaz. El fotógrafo SHALL poder cerrar el modal, seguir usando la app y volver a abrirlo para ver el progreso de cada archivo, desde el proyecto o desde la tarjeta del respaldo en la barra lateral. Un archivo que ya se está subiendo no SHALL lanzarse otra vez.

#### Scenario: Cerrar y reabrir el modal
- **WHEN** el fotógrafo cierra el modal a mitad de la subida y lo vuelve a abrir
- **THEN** ve el estado de cada archivo (subiendo, subido o fallido) y la subida no se reinicia

#### Scenario: Reabrir desde la barra lateral
- **WHEN** el fotógrafo cierra el modal, vuelve al dashboard y pulsa la tarjeta del respaldo en la barra lateral
- **THEN** ve el mismo modal con el estado de cada archivo y la subida no se reinicia

### Requirement: Cola offline-first
Al iniciar un respaldo, el desktop SHALL guardar la intención (proyecto, proveedores, calidad) en una cola persistente y SHALL quitarla al terminar. Cada respaldo en cola SHALL reanudarse con los archivos que falten en cuanto haya sesión de respaldo: al arrancar la app si ya la hay, o al iniciar sesión durante esa ejecución. Sin sesión, si el disco no está montado o si no hay red, el respaldo SHALL seguir en cola hasta la siguiente oportunidad.

#### Scenario: Cerrar la app a mitad de respaldo
- **WHEN** el fotógrafo cierra Lumik con un respaldo a medias y la vuelve a abrir con el disco conectado
- **THEN** el respaldo se reanuda con los archivos que faltaban

#### Scenario: Disco ausente al arrancar
- **WHEN** la app arranca con un respaldo en cola y sin el disco del proyecto
- **THEN** el respaldo sigue en cola y no se muestra ningún error

#### Scenario: Arrancar sin sesión
- **WHEN** la app arranca con un respaldo en cola y sin sesión de respaldo
- **THEN** no se intenta ninguna subida, el respaldo sigue en cola y la sección "Respaldos" no aparece

#### Scenario: Iniciar sesión con respaldos en cola
- **WHEN** el fotógrafo arrancó sin sesión con un respaldo en cola e inicia sesión en Ajustes → Respaldo con el disco conectado
- **THEN** el respaldo se reanuda con los archivos que faltaban y aparece su tarjeta en la barra lateral
