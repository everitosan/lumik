# Spec Delta

## ADDED Requirements

### Requirement: Mensaje principal
El hero de la landing SHALL presentar a Lumik como gratis y con el proyecto viviendo en el disco del fotógrafo, en inglés y en español. Su botón principal SHALL llevar a la página de descargas e indicar que la descarga es gratis. Mientras el repositorio no tenga licencia, la landing no SHALL describir a Lumik como open source.

#### Scenario: Hero en español
- **WHEN** se abre `/es/`
- **THEN** el título del hero es "Tu disco es tu estudio", el subtítulo dice que es gratis y que el proyecto vive en el disco, y el botón principal dice "Descargar gratis" y lleva a `/es/downloads`

#### Scenario: Sin licencia
- **WHEN** se abre cualquier página de la landing y el repositorio no tiene archivo de licencia
- **THEN** ningún texto dice "open source" ni "código abierto"

### Requirement: Pruebas en el hero
Dentro del hero, debajo del mensaje y del clip, la landing SHALL mostrar tres pruebas cortas, de modo que en escritorio se vean en la primera pantalla junto con el resto del hero: que es gratis y sin suscripción, que corre en Linux, Windows y macOS, y un promedio de importación de unos 150 RAW por minuto, aclarando que depende del medio de almacenamiento y del equipo.

#### Scenario: Tira de pruebas
- **WHEN** se abre la landing en una pantalla de escritorio
- **THEN** ve las tres pruebas en la primera pantalla, dentro del hero y antes de la sección de cómo funciona

### Requirement: Secciones de pilar
La landing SHALL tener una sección para cada uno de estos pilares, en este orden: "Tu disco es tu estudio", "Importa sin miedo" y "Las manos no dejan el teclado". Cada sección SHALL tener un título, un texto de no más de tres frases y un espacio para su clip. Los textos SHALL describir solo lo que se puede usar en un paquete publicado.

#### Scenario: Importa sin miedo
- **WHEN** se abre la sección "Importa sin miedo"
- **THEN** su texto menciona la firma y el renombrado al importar, que un archivo con fallo no detiene el resto y que se puede reintentar

#### Scenario: Funciones no publicadas
- **WHEN** se leen las secciones de pilar
- **THEN** no mencionan selección con IA, Android, respaldo en la nube ni entrega al cliente

### Requirement: Clips en bucle
Cada clip de la landing SHALL reproducirse sin audio, en bucle y sin controles, y SHALL mostrar una imagen de portada mientras no se reproduce. El clip SHALL cargarse solo cuando su sección se acerca a la pantalla. Un clip acelerado SHALL indicar su factor de aceleración en la imagen.

#### Scenario: Llegar a una sección
- **WHEN** el visitante baja hasta una sección con clip
- **THEN** el clip empieza a reproducirse sin sonido y en bucle

#### Scenario: Carga de la página
- **WHEN** se abre la landing y el visitante no baja de la primera pantalla
- **THEN** solo se descarga el clip del hero, y los de las secciones de pilar todavía no

#### Scenario: Pantalla angosta
- **WHEN** la landing se abre a 375 px de ancho
- **THEN** los clips ocupan el ancho disponible sin scroll horizontal

### Requirement: Clips con menos movimiento
Para el visitante que pide menos movimiento, cada clip SHALL quedar en pausa en su portada con un botón para reproducirlo, que funcione con el ratón y con el teclado.

#### Scenario: Menos movimiento
- **WHEN** el sistema del visitante tiene activado reducir el movimiento
- **THEN** cada clip muestra su portada quieta y un botón de play, y solo se reproduce si el visitante lo pulsa o lo activa con Enter o Espacio

### Requirement: Espacio de clip sin video
Mientras un espacio de clip no tenga video, SHALL mostrar su captura de respaldo en el mismo marco que tendrá el clip. El build de producción no SHALL mostrar ningún texto que anuncie que falta el video.

#### Scenario: Espacio sin clip
- **WHEN** la sección "Importa sin miedo" todavía no tiene clip
- **THEN** muestra su captura de respaldo con el mismo tamaño que tendrá el clip, y el build de producción no muestra ninguna marca de "video pendiente"

### Requirement: Medición de descargas
Cada botón de la landing que lleva a la página de descargas SHALL registrar en la analítica del sitio un evento que indique desde qué parte de la página se pulsó.

#### Scenario: Botón del hero
- **WHEN** el visitante pulsa "Descargar gratis" en el hero
- **THEN** la analítica registra un evento de descarga con la ubicación del hero
