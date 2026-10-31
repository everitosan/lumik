# Spec Delta

## MODIFIED Requirements

### Requirement: Orientación escrita en el archivo con espera
La orientación final SHALL escribirse en el archivo unos 600 ms después del último cambio, sin bloquear la interfaz. Si en ese intervalo llegan más rotaciones de la misma foto, SHALL hacerse una sola escritura con el valor final. En Linux y Windows se SHALL escribir dentro del archivo; en la app de tablet Android, en su sidecar `.xmp`, creándolo si no existe.

#### Scenario: Rotaciones seguidas
- **WHEN** el usuario pulsa `]` tres veces seguidas
- **THEN** el archivo recibe una sola escritura con orientación de 270°

#### Scenario: Android
- **WHEN** se rota una foto en la app de tablet Android
- **THEN** la orientación se guarda en el `.xmp` de la foto y el RAW no se modifica
