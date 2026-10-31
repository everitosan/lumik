# Spec Delta

## MODIFIED Requirements

### Requirement: Metadatos del fotógrafo
Si la firma está activa en los ajustes de la importación (por defecto, el valor global "embeber metadatos al importar", activo de fábrica), cada foto SHALL llevar el autor, el copyright y la URL de contacto elegidos para esa importación, los demás datos guardados del fotógrafo activo (email de contacto y condiciones de uso) y la descripción `{descripción}@{año}` si el proyecto tiene descripción. Los campos vacíos SHALL omitirse. En Linux, Windows y macOS se SHALL escribir dentro del archivo; en la app de tablet Android, en un sidecar `.xmp` junto a la foto.

#### Scenario: Escritorio
- **WHEN** se importa en Linux con autor y copyright en los ajustes de la importación
- **THEN** el RAW importado contiene esos valores en sus metadatos

#### Scenario: macOS
- **WHEN** se importa en macOS con autor en los ajustes de la importación
- **THEN** el RAW importado contiene el autor en sus metadatos y no se crea un `.xmp` junto a la foto

#### Scenario: Android
- **WHEN** se importa en la app de tablet Android con la firma activa y un autor en sus ajustes
- **THEN** junto a cada foto se crea un `.xmp` con el autor y la foto no se modifica

#### Scenario: Sin nada que escribir
- **WHEN** el renombrado está desactivado y no hay metadatos que escribir
- **THEN** las fotos se copian sin modificarse

#### Scenario: Firma desactivada solo esta vez
- **WHEN** la firma global está activa y el usuario la desactiva en los ajustes de la importación
- **THEN** las fotos de esa importación no llevan datos del fotógrafo

