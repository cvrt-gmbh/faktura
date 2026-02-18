use lopdf::{Document, Object, Stream, dictionary};

use super::FACTURX_FILENAME;
use super::profile::ZugferdProfile;
use super::xmp;
use crate::core::RechnungError;

/// Embed a Factur-X/ZUGFeRD XML into a PDF, producing a PDF/A-3 compliant document.
///
/// Takes existing PDF bytes and the XML string to embed.
/// Returns the modified PDF bytes with the XML attached as `factur-x.xml`.
pub fn embed_in_pdf(
    pdf_bytes: &[u8],
    xml: &str,
    profile: ZugferdProfile,
) -> Result<Vec<u8>, RechnungError> {
    let mut doc = Document::load_mem(pdf_bytes)
        .map_err(|e| RechnungError::Builder(format!("failed to load PDF: {e}")))?;

    embed_xml_into_document(&mut doc, xml.as_bytes(), profile)?;

    let mut output = Vec::new();
    doc.save_to(&mut output)
        .map_err(|e| RechnungError::Builder(format!("failed to save PDF: {e}")))?;

    Ok(output)
}

fn embed_xml_into_document(
    doc: &mut Document,
    xml_bytes: &[u8],
    profile: ZugferdProfile,
) -> Result<(), RechnungError> {
    // 1. Create the EmbeddedFile stream
    let ef_stream = Stream::new(
        dictionary! {
            "Type" => "EmbeddedFile",
            "Subtype" => Object::Name(b"text#2Fxml".to_vec()),
            "Params" => dictionary! {
                "Size" => Object::Integer(xml_bytes.len() as i64),
            },
        },
        xml_bytes.to_vec(),
    );
    let ef_stream_id = doc.add_object(ef_stream);

    // 2. Create the FileSpec dictionary
    let af_rel = profile.af_relationship();
    let filespec = dictionary! {
        "Type" => "Filespec",
        "F" => Object::string_literal(FACTURX_FILENAME),
        "UF" => Object::string_literal(FACTURX_FILENAME),
        "Desc" => Object::string_literal("Factur-X XML invoice"),
        "AFRelationship" => Object::Name(af_rel.as_bytes().to_vec()),
        "EF" => dictionary! {
            "F" => Object::Reference(ef_stream_id),
            "UF" => Object::Reference(ef_stream_id),
        },
    };
    let filespec_id = doc.add_object(filespec);

    // 3. Create the EmbeddedFiles name tree
    let ef_name_tree = dictionary! {
        "Names" => Object::Array(vec![
            Object::string_literal(FACTURX_FILENAME),
            Object::Reference(filespec_id),
        ]),
    };
    let ef_name_tree_id = doc.add_object(ef_name_tree);

    // 4. Create or update the Names dictionary
    let names_dict = dictionary! {
        "EmbeddedFiles" => Object::Reference(ef_name_tree_id),
    };
    let names_id = doc.add_object(names_dict);

    // 5. Create XMP metadata stream
    let xmp_str = xmp::build_xmp(profile);
    let xmp_bytes = xmp_str.into_bytes();
    let metadata_stream = Stream::new(
        dictionary! {
            "Type" => "Metadata",
            "Subtype" => "XML",
        },
        xmp_bytes,
    )
    .with_compression(false); // XMP must not be compressed per PDF/A
    let metadata_id = doc.add_object(metadata_stream);

    // 6. Update the Catalog
    let catalog = doc
        .catalog_mut()
        .map_err(|e| RechnungError::Builder(format!("failed to get catalog: {e}")))?;

    catalog.set("AF", Object::Array(vec![Object::Reference(filespec_id)]));
    catalog.set("Names", Object::Reference(names_id));
    catalog.set("Metadata", Object::Reference(metadata_id));
    // Mark as PDF/A-3
    catalog.set(
        "MarkInfo",
        dictionary! { "Marked" => Object::Boolean(true) },
    );

    Ok(())
}
