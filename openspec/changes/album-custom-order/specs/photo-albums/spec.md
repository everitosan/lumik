# Spec Delta

## ADDED Requirements

### Requirement: Orden personalizado
Cada álbum SHALL tener un orden personalizado, que es su orden propio. Al crearse este requisito, el orden de cada álbum existente SHALL ser el orden en que se agregaron sus elementos. Un elemento agregado SHALL quedar al final; un elemento reemplazado SHALL conservar su lugar; quitar un elemento no SHALL cambiar el orden de los demás.

#### Scenario: Álbum existente al actualizar
- **WHEN** el fotógrafo abre, después de actualizar, un álbum que armó antes
- **THEN** ve sus fotos en el orden en que las agregó

#### Scenario: Agregar al final
- **WHEN** el fotógrafo agrega una foto a un álbum de 20 elementos ordenado a su gusto
- **THEN** la foto queda en el lugar 21 y las otras 20 conservan su orden

#### Scenario: Reemplazar en su lugar
- **WHEN** el fotógrafo reemplaza el elemento 5 por otra versión
- **THEN** la nueva versión queda en el lugar 5

### Requirement: Reordenar arrastrando
En el orden personalizado, el fotógrafo SHALL poder arrastrar una foto del álbum a otro lugar del grid. Mientras arrastra, la foto SHALL seguir al cursor y las demás SHALL apartarse para mostrar dónde caerá. Al soltarla SHALL insertarse en ese lugar, recorriendo a las demás. Si el destino está fuera de la vista, el grid SHALL desplazarse al acercar la foto a su borde. El nuevo orden SHALL guardarse al soltar.

#### Scenario: Insertar entre dos fotos
- **WHEN** en un álbum con `a b c d e` el fotógrafo arrastra `e` y la suelta entre `a` y `b`
- **THEN** el álbum queda `a e b c d`, y se ve igual al volver a abrirlo

#### Scenario: Soltar fuera del grid
- **WHEN** el fotógrafo arrastra una foto y la suelta fuera del grid o pulsa Esc
- **THEN** el orden no cambia

#### Scenario: Destino fuera de la vista
- **WHEN** el fotógrafo arrastra una foto hacia el borde inferior del grid
- **THEN** el grid se desplaza hacia abajo hasta que suelte la foto o la aleje del borde

### Requirement: Mover al principio o al final
En el orden personalizado, el menú de cada foto del álbum SHALL ofrecer "Mover al principio" y "Mover al final". La foto SHALL pasar a ser la primera o la última y las demás SHALL conservar su orden.

#### Scenario: Mover al principio
- **WHEN** el fotógrafo elige "Mover al principio" en la foto 280 de un álbum de 300
- **THEN** esa foto queda primera y las demás conservan su orden detrás de ella

### Requirement: Vista por fecha de captura
"Ordenar por" SHALL ofrecer "Personalizado" y "Fecha de captura". "Fecha de captura" SHALL mostrar el álbum de la más reciente a la más antigua sin cambiar su orden personalizado, y en esa vista no SHALL poderse arrastrar fotos ni mostrarse "Mover al principio" ni "Mover al final".

#### Scenario: Ver por fecha y volver
- **WHEN** el fotógrafo cambia a "Fecha de captura" y luego vuelve a "Personalizado"
- **THEN** el álbum vuelve a su orden personalizado, sin cambios

#### Scenario: Sin arrastre por fecha
- **WHEN** el fotógrafo intenta arrastrar una foto con el álbum ordenado por fecha de captura
- **THEN** la foto no se mueve

### Requirement: Modo de orden por álbum
El modo de orden que el fotógrafo elige SHALL guardarse en cada álbum y SHALL usarse al volver a abrirlo. Un álbum nuevo SHALL abrirse en "Personalizado".

#### Scenario: Recordar el modo
- **WHEN** el fotógrafo pone un álbum en "Fecha de captura", sale y lo vuelve a abrir
- **THEN** el álbum se abre ordenado por fecha de captura
