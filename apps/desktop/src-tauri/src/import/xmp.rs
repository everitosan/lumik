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

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
