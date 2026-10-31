# Spec Delta

## Purpose

Cómo se publica la app de tablet Android: en Google Play, firmada, sin permisos que Play no aprueba para Lumik y con versiones que corresponden a los tags del monorepo.

## ADDED Requirements

### Requirement: Publicación en Google Play
La app de tablet SHALL publicarse en Google Play con el id `com.lumik.tablet`, como un Android App Bundle firmado con la clave de subida y con la firma de apps de Google Play. Cada beta SHALL publicarse primero en la pista de prueba interna.

#### Scenario: Nueva beta
- **WHEN** se publica una beta de la app de tablet
- **THEN** su AAB llega a la pista de prueba interna de Google Play y los probadores la instalan desde Play

### Requirement: Permisos mínimos
La app no SHALL declarar `MANAGE_EXTERNAL_STORAGE` ni permisos de lectura o escritura amplia del almacenamiento: SHALL llegar a los discos y a la tarjeta solo por SAF.

#### Scenario: Revisión del manifiesto
- **WHEN** se inspecciona el manifiesto del AAB publicado
- **THEN** no aparece `MANAGE_EXTERNAL_STORAGE` ni `READ_EXTERNAL_STORAGE` ni `WRITE_EXTERNAL_STORAGE`

### Requirement: Versión ligada a los tags
La versión visible de la app SHALL coincidir con la del tag del monorepo del que se construye (por ejemplo `0.2.0-beta.3` del tag `v0.2.0-beta.3`), y el código de versión de Android SHALL crecer con cada AAB subido.

#### Scenario: Dos betas seguidas
- **WHEN** se construyen los AAB de `v0.2.0-beta.3` y `v0.2.0-beta.4`
- **THEN** muestran esas versiones y el segundo tiene un código de versión mayor que el primero

### Requirement: Solo build de release
El AAB publicado SHALL ser un build de release con el núcleo de Rust compilado en modo optimizado para `arm64-v8a`.

#### Scenario: AAB de una beta
- **WHEN** se construye el AAB de una beta
- **THEN** contiene la librería del núcleo para `arm64-v8a` compilada en release y no incluye el servidor de desarrollo de JavaScript
