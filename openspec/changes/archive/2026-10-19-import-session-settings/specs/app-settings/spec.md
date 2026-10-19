# Spec Delta

## MODIFIED Requirements

### Requirement: Metadatos del fotógrafo
El usuario SHALL poder editar los datos que se escriben en las fotos al importar: autor, copyright y URL de contacto. Estos datos SHALL guardarse en la base global de la instalación, no en los discos, y SHALL ser los valores iniciales de cada importación, que puede cambiarlos solo para esa importación sin modificarlos.

#### Scenario: Editar copyright
- **WHEN** el usuario cambia el copyright a "© 2026 Estudio Lumik"
- **THEN** los imports siguientes muestran y escriben ese copyright en las fotos, salvo que se cambie en el paso de ajustes de una importación

#### Scenario: Cambio en una importación
- **WHEN** el usuario cambia el autor en el paso de ajustes de una importación
- **THEN** la página de Ajustes sigue mostrando el autor anterior
