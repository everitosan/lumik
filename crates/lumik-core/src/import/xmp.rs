// Este módulo solo se usa en Android (sin exiftool): ahí se escriben sidecars
// .xmp a mano. En desktop exiftool hace este trabajo, por lo que estas funciones
// aparecen como "dead code" al compilar para Linux/Windows.
#![allow(dead_code)]

use crate::db::models::PhotographerMetadata;
use std::path::Path;

/// Write an XMP sidecar file alongside a RAW photo.
/// The sidecar path is `file_path` with the extension replaced by `.xmp`.
/// Writes only if there is at least one non-empty field.
pub fn write_xmp_sidecar(
    file_path: &Path,
    metadata: Option<&PhotographerMetadata>,
    image_description: Option<&str>,
) -> Result<(), String> {
    let mut description_attrs = String::new();
    let mut description_children = String::new();

    // dc:description (project description@year)
    if let Some(desc) = image_description {
        if !desc.is_empty() {
            description_children.push_str(&format!(
                "      <dc:description><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:description>\n",
                xml_escape(desc)
            ));
        }
    }

    if let Some(meta) = metadata {
        // dc:creator (Artist)
        if let Some(ref v) = meta.artist {
            if !v.is_empty() {
                description_children.push_str(&format!(
                    "      <dc:creator><rdf:Seq><rdf:li>{}</rdf:li></rdf:Seq></dc:creator>\n",
                    xml_escape(v)
                ));
            }
        }

        // dc:rights (Copyright)
        if let Some(ref v) = meta.copyright {
            if !v.is_empty() {
                description_children.push_str(&format!(
                    "      <dc:rights><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:rights>\n",
                    xml_escape(v)
                ));
            }
        }

        // xmpRights:UsageTerms
        if let Some(ref v) = meta.usage_terms {
            if !v.is_empty() {
                description_children.push_str(&format!(
                    "      <xmpRights:UsageTerms><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></xmpRights:UsageTerms>\n",
                    xml_escape(v)
                ));
            }
        }

        // Iptc4xmpCore contact info
        let url = meta.contact_url.as_deref().unwrap_or("").trim().to_string();
        let email = meta.contact_email.as_deref().unwrap_or("").trim().to_string();
        if !url.is_empty() || !email.is_empty() {
            description_attrs.push_str(" xmlns:Iptc4xmpCore=\"http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/\"");
            let mut contact = String::new();
            if !url.is_empty() {
                contact.push_str(&format!(
                    "          <Iptc4xmpCore:CiUrlWork>{}</Iptc4xmpCore:CiUrlWork>\n",
                    xml_escape(&url)
                ));
            }
            if !email.is_empty() {
                contact.push_str(&format!(
                    "          <Iptc4xmpCore:CiEmailWork>{}</Iptc4xmpCore:CiEmailWork>\n",
                    xml_escape(&email)
                ));
            }
            description_children.push_str(&format!(
                "      <Iptc4xmpCore:CreatorContactInfo rdf:parseType=\"Resource\">\n{}\
                 \n      </Iptc4xmpCore:CreatorContactInfo>\n",
                contact.trim_end()
            ));
        }
    }

    if description_children.is_empty() {
        return Ok(());
    }

    let content = format!(
        "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"Lumik\">\n  \
         <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n    \
         <rdf:Description rdf:about=\"\"\n      \
         xmlns:dc=\"http://purl.org/dc/elements/1.1/\"\n      \
         xmlns:xmpRights=\"http://ns.adobe.com/xap/1.0/rights/\"{attrs}>\n\
         {children}\
         \n    </rdf:Description>\n  \
         </rdf:RDF>\n\
         </x:xmpmeta>\n\
         <?xpacket end=\"w\"?>",
        attrs = description_attrs,
        children = description_children,
    );

    let xmp_path = file_path.with_extension("xmp");
    std::fs::write(&xmp_path, content)
        .map_err(|e| format!("Failed to write XMP sidecar {:?}: {}", xmp_path, e))
}

/// Update (or create) the XMP sidecar with the new EXIF orientation.
/// Used on Android where exiftool is not available to write to the RAW file.
/// If a sidecar already exists, updates the tiff:Orientation element in-place.
/// If no sidecar exists, writes a minimal XMP with just the orientation.
pub fn update_xmp_orientation(file_path: &Path, rotation: i32) -> Result<(), String> {
    let xmp_path = file_path.with_extension("xmp");
    let orientation = match rotation { 90 => 6, 180 => 3, 270 => 8, _ => 1 };
    let tag_value = format!("<tiff:Orientation>{}</tiff:Orientation>", orientation);

    if xmp_path.exists() {
        let content = std::fs::read_to_string(&xmp_path)
            .map_err(|e| format!("Read XMP: {}", e))?;

        let updated = if content.contains("<tiff:Orientation>") {
            let start = content.find("<tiff:Orientation>").unwrap();
            let end = content.find("</tiff:Orientation>").unwrap() + "</tiff:Orientation>".len();
            format!("{}{}{}", &content[..start], tag_value, &content[end..])
        } else {
            let mut c = if !content.contains("xmlns:tiff") {
                content.replace(
                    "rdf:about=\"\"",
                    "rdf:about=\"\"\n      xmlns:tiff=\"http://ns.adobe.com/tiff/1.0/\"",
                )
            } else {
                content
            };
            c = c.replace(
                "\n    </rdf:Description>",
                &format!("\n      {}\n    </rdf:Description>", tag_value),
            );
            c
        };

        std::fs::write(&xmp_path, updated)
            .map_err(|e| format!("Write XMP: {}", e))
    } else {
        let content = format!(
            "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
             <x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"Lumik\">\n  \
             <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n    \
             <rdf:Description rdf:about=\"\"\n      \
             xmlns:tiff=\"http://ns.adobe.com/tiff/1.0/\">\n\
             \n      {}\n\
             \n    </rdf:Description>\n  \
             </rdf:RDF>\n\
             </x:xmpmeta>\n\
             <?xpacket end=\"w\"?>",
            tag_value
        );
        std::fs::write(&xmp_path, content)
            .map_err(|e| format!("Write XMP orientation: {}", e))
    }
}

/// Actualiza (o quita, con `None`) `dc:description` en el sidecar `.xmp` de la
/// foto. Crea el sidecar si no existe y hay algo que escribir.
pub fn update_xmp_description(file_path: &Path, description: Option<&str>) -> Result<(), String> {
    let xmp_path = file_path.with_extension("xmp");
    let description = description.filter(|d| !d.is_empty());
    let tag_value = description.map(|d| {
        format!(
            "<dc:description><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:description>",
            xml_escape(d)
        )
    });

    if !xmp_path.exists() {
        let Some(tag_value) = tag_value else {
            return Ok(());
        };
        let content = format!(
            "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
             <x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"Lumik\">\n  \
             <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n    \
             <rdf:Description rdf:about=\"\"\n      \
             xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n\
             \n      {}\n\
             \n    </rdf:Description>\n  \
             </rdf:RDF>\n\
             </x:xmpmeta>\n\
             <?xpacket end=\"w\"?>",
            tag_value
        );
        return std::fs::write(&xmp_path, content).map_err(|e| format!("Write XMP description: {}", e));
    }

    let content = std::fs::read_to_string(&xmp_path).map_err(|e| format!("Read XMP: {}", e))?;

    // Quita la descripción anterior (con su línea) si la hay.
    let mut updated = match (content.find("<dc:description>"), content.find("</dc:description>")) {
        (Some(start), Some(end)) => {
            let end = end + "</dc:description>".len();
            let line_start = content[..start].rfind('\n').map(|i| i + 1).unwrap_or(start);
            let line_end = if content[end..].starts_with('\n') { end + 1 } else { end };
            format!("{}{}", &content[..line_start], &content[line_end..])
        }
        _ => content,
    };

    if let Some(tag_value) = tag_value {
        if !updated.contains("xmlns:dc") {
            updated = updated.replacen(
                "rdf:about=\"\"",
                "rdf:about=\"\"\n      xmlns:dc=\"http://purl.org/dc/elements/1.1/\"",
                1,
            );
        }
        updated = updated.replacen(
            "\n    </rdf:Description>",
            &format!("\n      {}\n    </rdf:Description>", tag_value),
            1,
        );
    }

    std::fs::write(&xmp_path, updated).map_err(|e| format!("Write XMP: {}", e))
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_photo(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lumik_xmp_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("IMG_0001.CR3")
    }

    #[test]
    fn description_is_created_replaced_and_removed() {
        let photo = temp_photo("desc");
        let xmp = photo.with_extension("xmp");

        update_xmp_description(&photo, Some("Boda@2024")).unwrap();
        let c = std::fs::read_to_string(&xmp).unwrap();
        assert!(c.contains("xmlns:dc="));
        assert!(c.contains(">Boda@2024<"));

        update_xmp_description(&photo, Some("Playa & sol@2025")).unwrap();
        let c = std::fs::read_to_string(&xmp).unwrap();
        assert!(c.contains(">Playa &amp; sol@2025<"));
        assert!(!c.contains("Boda@2024"));
        assert_eq!(c.matches("<dc:description>").count(), 1);

        update_xmp_description(&photo, None).unwrap();
        let c = std::fs::read_to_string(&xmp).unwrap();
        assert!(!c.contains("<dc:description>"));
        assert!(c.contains("</rdf:Description>"));
    }

    #[test]
    fn description_keeps_other_fields_of_existing_sidecar() {
        let photo = temp_photo("keep");
        write_xmp_sidecar(&photo, None, Some("Viejo@2023")).unwrap();
        update_xmp_orientation(&photo, 90).unwrap();

        update_xmp_description(&photo, Some("Nuevo@2024")).unwrap();
        let c = std::fs::read_to_string(photo.with_extension("xmp")).unwrap();
        assert!(c.contains(">Nuevo@2024<"));
        assert!(!c.contains("Viejo@2023"));
        assert!(c.contains("<tiff:Orientation>6</tiff:Orientation>"));
    }

    #[test]
    fn no_sidecar_is_created_to_remove_nothing() {
        let photo = temp_photo("none");
        update_xmp_description(&photo, None).unwrap();
        assert!(!photo.with_extension("xmp").exists());
    }
}
