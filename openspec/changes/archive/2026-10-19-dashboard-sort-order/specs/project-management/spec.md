# Spec Delta

## ADDED Requirements

### Requirement: Orden configurable del dashboard
El usuario SHALL poder elegir desde el dashboard si las secciones de año se muestran de la más reciente a la más antigua o al revés, y lo mismo para los proyectos dentro de cada año. La elección SHALL aplicarse al momento y guardarse en la configuración de la instalación, no en los discos, de modo que se conserve entre reinicios. En una instalación nueva, los años SHALL ir de más reciente a más antiguo y los proyectos de más antiguo a más reciente.

#### Scenario: Instalación nueva
- **WHEN** el usuario abre el dashboard por primera vez
- **THEN** ve los años de más reciente a más antiguo y, dentro de cada año, los proyectos de más antiguo a más reciente

#### Scenario: Años de más antiguo a más reciente
- **WHEN** hay proyectos de 2024, 2025 y 2026 y el usuario elige ver primero los años más antiguos
- **THEN** las secciones aparecen en el orden 2024, 2025, 2026

#### Scenario: Proyectos más recientes primero
- **WHEN** un año tiene proyectos del 2026-03-02 y del 2026-08-10 y el usuario elige ver primero los proyectos más recientes
- **THEN** dentro de la sección 2026 aparece primero el del 2026-08-10

#### Scenario: Se conserva tras reiniciar
- **WHEN** el usuario cambia el orden y reinicia la app
- **THEN** el dashboard se muestra con el orden elegido

#### Scenario: Independiente de los discos
- **WHEN** el usuario cambia el orden y después conecta otro disco con proyectos
- **THEN** los proyectos de ese disco se muestran con el mismo orden

## MODIFIED Requirements

### Requirement: Orden del dashboard
El dashboard SHALL agrupar los proyectos en secciones por el año de su fecha de sesión o, si no la tienen, de su fecha de creación. Las secciones y los proyectos de cada sección SHALL ordenarse en la dirección elegida por el usuario. Dentro de una sección, los proyectos SHALL ordenarse por esa misma fecha y desempatarse por fecha de creación en la misma dirección. Los proyectos archivados o eliminados lógicamente SHALL quedar fuera del dashboard.

#### Scenario: Mezcla de proyectos con y sin fecha
- **WHEN** existe un proyecto sin fecha de sesión creado el 2026-05-20 y proyectos con fecha de sesión en 2026
- **THEN** el proyecto sin fecha aparece en la sección 2026, ordenado por su fecha de creación junto a los demás

#### Scenario: Misma fecha de sesión
- **WHEN** dos proyectos comparten fecha de sesión y el orden de proyectos es de más antiguo a más reciente
- **THEN** aparece primero el creado antes
