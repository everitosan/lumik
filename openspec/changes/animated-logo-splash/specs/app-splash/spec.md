# Spec Delta

## Purpose

Presenta la marca de Lumik al abrir la app de escritorio y la de tablet Android: una animación corta en la que el logo, un libro, se abre hasta formarse, y después deja ver la app sin hacer esperar al fotógrafo.

## ADDED Requirements

### Requirement: Splash al arrancar
El desktop y la tablet Android SHALL mostrar el splash del logo al arrancar en frío, encima de la app y sobre el fondo de la app (`#131313`). Volver a una app que ya estaba abierta, por ejemplo desde segundo plano en la tablet, NO SHALL mostrarlo.

#### Scenario: Arranque en frío del desktop
- **WHEN** el fotógrafo abre Lumik en el desktop
- **THEN** ve el splash del logo antes que la app

#### Scenario: Volver desde segundo plano
- **WHEN** el fotógrafo cambia a otra app en la tablet y regresa a Lumik sin que se haya cerrado
- **THEN** ve la app donde la dejó, sin splash

### Requirement: Secuencia del libro que se abre
La animación SHALL mostrar el libro visto desde su borde inferior. Empieza cerrado, como un rectángulo. La tapa gira 90° sobre el lomo, la esquina inferior izquierda, hasta quedar de pie. Las tres páginas de color se levantan sobre el mismo eje, una tras otra. La contratapa no se mueve. La animación SHALL terminar en el logo exacto de Lumik.

#### Scenario: Libro cerrado
- **WHEN** empieza la animación
- **THEN** se ve el libro cerrado, como un rectángulo, sin ninguna página de color asomada

#### Scenario: Termina en el logo
- **WHEN** la tapa y las páginas terminan de girar
- **THEN** la imagen coincide con el logo de Lumik: la misma forma, los mismos colores y las mismas transparencias que el ícono

### Requirement: Nombre bajo el isotipo
Bajo el libro SHALL aparecer el nombre "lumik" del logotipo: minúsculas de trazo redondeado, en un color claro sobre el fondo oscuro, con el punto de la "i" en el dorado de la marca. Las letras SHALL salir de su línea base una tras otra, con el mismo rebote leve que las páginas, empezando cuando las páginas se asientan. El punto de la "i" SHALL aparecer al final.

#### Scenario: Las letras salen después del libro
- **WHEN** la última página se asienta
- **THEN** las letras de "lumik" suben desde su línea base de izquierda a derecha, y al final aparece el punto dorado de la "i"

#### Scenario: Logotipo completo
- **WHEN** termina la animación, antes del desvanecimiento
- **THEN** se ve el logo con el nombre "lumik" centrado debajo, con las proporciones del logotipo de Lumik

### Requirement: Resplandor detrás del libro
Detrás del libro SHALL aparecer un resplandor suave antes de que se vea el libro cerrado, para que su cuerpo oscuro se distinga del fondo de la app. El resplandor SHALL desvanecerse junto con el logo al final.

#### Scenario: Libro visible sobre el fondo oscuro
- **WHEN** se muestra el libro cerrado
- **THEN** su silueta se distingue contra el resplandor, aunque el cuerpo del libro y el fondo sean de tonos oscuros parecidos

### Requirement: Duración fija y salida con transparencia
El splash SHALL durar un tiempo fijo de alrededor de 2 s, contando la animación y el desvanecimiento, sin esperar a que la app termine de cargar sus datos. Al final, el logo, el nombre y el resplandor SHALL desvanecerse con transparencia y dejar ver la app ya dibujada debajo.

#### Scenario: La app carga más rápido que el splash
- **WHEN** la app ya está lista antes de que termine la animación
- **THEN** el splash termina su animación completa y luego se desvanece

#### Scenario: La app sigue cargando
- **WHEN** termina el splash y los proyectos aún no terminan de cargar
- **THEN** el splash se desvanece igual y la app muestra su propio estado de carga

### Requirement: Reducir movimiento
Si el sistema tiene activado "reducir movimiento", el splash SHALL mostrar el logo y el nombre ya formados y quietos, con el resplandor, y SHALL desvanecerse con transparencia como en la versión animada.

#### Scenario: Movimiento reducido
- **WHEN** el fotógrafo abre Lumik con "reducir movimiento" activado en el sistema
- **THEN** ve el logo completo sin que la tapa ni las páginas se muevan, y luego la app

### Requirement: Arranque sin destellos
El paso del sistema a la app no SHALL mostrar ningún destello de otro color. En el desktop, la ventana SHALL ser del color de fondo de la app desde que aparece. En la tablet, el splash del sistema SHALL mostrar el libro cerrado sobre el fondo de la app, en el mismo lugar y tamaño donde empieza la animación.

#### Scenario: Ventana del desktop
- **WHEN** se abre la ventana de Lumik en el desktop
- **THEN** es oscura desde el primer cuadro, sin un destello blanco antes del splash

#### Scenario: Del splash del sistema a la animación en la tablet
- **WHEN** Android cambia de su splash al de Lumik
- **THEN** el libro cerrado no salta de lugar ni de tamaño, y la animación sigue desde ahí
