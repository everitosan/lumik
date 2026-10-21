# Spec Delta

## ADDED Requirements

### Requirement: Identidad portable del proyecto
Un proyecto SHALL identificarse por su `project.id`, sin importar en qué disco ni en qué plataforma esté. Ni el proyecto ni sus fotos SHALL depender del UUID del disco donde viven: las fotos nuevas no SHALL guardar ese UUID, y los valores ya guardados no SHALL usarse. Mover la carpeta de un proyecto a otro disco, dentro de `lumik/`, SHALL bastar para que se liste desde el nuevo disco.

#### Scenario: Mover un proyecto a otro disco
- **WHEN** el usuario copia `lumik/2025/03/29_Chepe/` del disco A al disco B, conecta solo el disco B y abre Lumik
- **THEN** el proyecto aparece en el dashboard con el mismo id, sus fotos y su estado de respaldo, desde el disco B

#### Scenario: Proyecto abierto en otra plataforma
- **WHEN** un proyecto importado en Linux se abre en macOS, donde el disco tiene otro UUID
- **THEN** el proyecto y sus fotos se muestran y se editan igual que en Linux

#### Scenario: Importar fotos
- **WHEN** el usuario importa fotos en un proyecto
- **THEN** las fotos nuevas se guardan sin el UUID del disco

### Requirement: Proyecto duplicado en dos discos
Si el mismo proyecto (mismo `project.id`) está en dos discos montados a la vez, el sistema SHALL listarlo una sola vez, desde el disco que descubra primero, y no SHALL avisar del duplicado. Esta es una limitación conocida: qué copia se lista no está garantizado.

#### Scenario: Copia en dos discos conectados
- **WHEN** los discos A y B contienen la misma carpeta de proyecto y ambos están montados
- **THEN** el dashboard muestra el proyecto una sola vez, desde uno de los dos discos
