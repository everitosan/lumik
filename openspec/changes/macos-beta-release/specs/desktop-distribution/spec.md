# Spec Delta

## Purpose

Define cómo se entrega la app de escritorio en cada plataforma: qué paquetes publica el release de cada versión, qué sistemas soporta, cómo va firmada y qué incluye para que la app instalada funcione sin que el usuario instale nada más.

## ADDED Requirements

### Requirement: Paquetes del release
Cada tag de versión SHALL publicar en su release de GitHub un paquete por plataforma de escritorio: `.deb` y `.AppImage` para Linux, un instalador `.exe` para Windows y un `.dmg` para macOS. La versión de cada paquete SHALL ser la del tag.

#### Scenario: Tag de versión
- **WHEN** se publica el tag `v0.1.4-beta`
- **THEN** su release contiene los paquetes de Linux, Windows y macOS, y la app de cada uno se reporta como versión `0.1.4-beta`

### Requirement: Paquete de macOS universal
El `.dmg` de macOS SHALL contener una sola app que corra de forma nativa en Mac con procesador Intel y con Apple Silicon, en macOS 12 Monterey o posterior. En versiones anteriores macOS SHALL impedir abrirla.

#### Scenario: Mac con Apple Silicon
- **WHEN** se abre la app del `.dmg` en una Mac con Apple Silicon
- **THEN** corre de forma nativa, sin Rosetta

#### Scenario: Mac Intel con Monterey
- **WHEN** se abre la app del `.dmg` en una Mac Intel con macOS 12
- **THEN** la app arranca y lista los discos montados

### Requirement: Herramientas incluidas en el paquete
La app instalada SHALL leer y escribir metadatos de las fotos (previews embebidos de RAW, EXIF y la firma del fotógrafo) sin que el usuario instale herramientas aparte y sin depender del PATH con que se abre. En Windows y macOS la herramienta de metadatos SHALL venir dentro del paquete. Si la incluida no puede ejecutarse, la app SHALL intentar con una instalada en el sistema.

#### Scenario: App abierta desde Finder
- **WHEN** en una Mac sin exiftool instalado se abre Lumik desde Finder y se abre un proyecto con fotos RAW
- **THEN** el grid muestra las miniaturas de los RAW y el visor muestra su preview

#### Scenario: Firma al importar en macOS
- **WHEN** en la app instalada en macOS se importa un RAW con la firma activa
- **THEN** el archivo importado contiene el autor y el copyright de la importación

### Requirement: Firma del paquete de macOS en beta
Mientras la app esté en beta, el paquete de macOS SHALL ir firmado ad hoc, sin Developer ID ni notarización. Las notas de cada release SHALL explicar cómo abrir la app la primera vez pese al aviso de Gatekeeper: quitando la cuarentena con `xattr` o desde "Abrir igualmente" en Privacidad y seguridad.

#### Scenario: Primera apertura tras descargar
- **WHEN** un tester descarga el `.dmg`, copia Lumik a Aplicaciones y la abre
- **THEN** macOS la bloquea por ser de un desarrollador no verificado, y siguiendo las instrucciones de las notas del release la app abre y las siguientes aperturas no muestran el aviso

### Requirement: Acceso a discos externos en macOS
La primera vez que la app instalada lee un disco externo, macOS SHALL pedir permiso con un texto de Lumik que explique que accede a los discos para abrir e importar proyectos. Si el usuario lo niega, los discos SHALL seguir listándose, pero sus proyectos no se pueden abrir hasta conceder el permiso en Ajustes del Sistema.

#### Scenario: Conceder el permiso
- **WHEN** con un disco externo conectado se abre Lumik instalado por primera vez y el usuario permite el acceso
- **THEN** el dashboard muestra los proyectos de ese disco

#### Scenario: Negar el permiso
- **WHEN** el usuario niega el acceso a volúmenes extraíbles
- **THEN** el disco aparece en la barra lateral y sus proyectos no se muestran
