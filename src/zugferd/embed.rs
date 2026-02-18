use lopdf::{Document, Object, Stream, dictionary};

use super::FACTURX_FILENAME;
use super::profile::ZugferdProfile;
use super::xmp;
use crate::core::RechnungError;

/// Embed a Factur-X/ZUGFeRD XML into a PDF, producing a PDF/A-3 compliant document.
///
/// Takes existing PDF bytes and the XML string to embed.
/// Returns the modified PDF bytes with the XML attached as `factur-x.xml`.
///
/// Adds the required PDF/A-3 structures:
/// - Embedded file stream with `factur-x.xml`
/// - XMP metadata with Factur-X extension schema
/// - OutputIntent with sRGB ICC profile
/// - MarkInfo tagged-PDF flag
pub fn embed_in_pdf(
    pdf_bytes: &[u8],
    xml: &str,
    profile: ZugferdProfile,
) -> Result<Vec<u8>, RechnungError> {
    let mut doc = Document::load_mem(pdf_bytes)
        .map_err(|e| RechnungError::Builder(format!("failed to load PDF: {e}")))?;

    embed_xml_into_document(&mut doc, xml.as_bytes(), profile)?;

    // PDF/A-3 requires a document ID in the trailer
    if !doc.trailer.has(b"ID") {
        let id_bytes = Object::string_literal(format!(
            "faktura-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        doc.trailer
            .set("ID", Object::Array(vec![id_bytes.clone(), id_bytes]));
    }

    // PDF/A-3 requires version 1.7 and a binary header comment (ISO 19005-3, 6.1.2).
    // lopdf writes "%PDF-{version}\n" as the header. By embedding the binary
    // comment bytes directly into the version string, all xref offsets computed by
    // lopdf will correctly account for the extra bytes.
    //
    // The version field is written via `write!()` so it goes through UTF-8.
    // We need 4 bytes > 127 that are also valid UTF-8. Two-byte UTF-8 sequences
    // (0xC2-0xDF followed by 0x80-0xBF) give us bytes > 127.
    // We use 4 two-byte UTF-8 chars: each first byte > 0xC0 (>127), satisfying PDF/A.
    // veraPDF checks the first 4 bytes AFTER the '%' character on line 2.
    let binary_comment = "\n%\u{00e2}\u{00e3}\u{00cf}\u{00d3}";
    doc.version = format!("1.7{binary_comment}");

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
            "Subtype" => Object::Name(b"text/xml".to_vec()),
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

    // 6. Create sRGB ICC profile stream and OutputIntent (required for PDF/A-3)
    let icc_bytes = build_srgb_icc_profile();
    let icc_stream = Stream::new(
        dictionary! {
            "N" => Object::Integer(3),
        },
        icc_bytes,
    );
    let icc_stream_id = doc.add_object(icc_stream);

    let output_intent = dictionary! {
        "Type" => "OutputIntent",
        "S" => Object::Name(b"GTS_PDFA1".to_vec()),
        "OutputConditionIdentifier" => Object::string_literal("sRGB IEC61966-2.1"),
        "RegistryName" => Object::string_literal("http://www.color.org"),
        "Info" => Object::string_literal("sRGB IEC61966-2.1"),
        "DestOutputProfile" => Object::Reference(icc_stream_id),
    };
    let output_intent_id = doc.add_object(output_intent);

    // 7. Update the Catalog
    let catalog = doc
        .catalog_mut()
        .map_err(|e| RechnungError::Builder(format!("failed to get catalog: {e}")))?;

    catalog.set("AF", Object::Array(vec![Object::Reference(filespec_id)]));
    catalog.set("Names", Object::Reference(names_id));
    catalog.set("Metadata", Object::Reference(metadata_id));
    catalog.set(
        "OutputIntents",
        Object::Array(vec![Object::Reference(output_intent_id)]),
    );
    catalog.set(
        "MarkInfo",
        dictionary! { "Marked" => Object::Boolean(true) },
    );

    Ok(())
}

/// Build a minimal valid ICC v2 sRGB profile for PDF/A-3 OutputIntent.
///
/// This generates a ~290-byte ICC profile with the minimum required tags
/// (desc, wtpt, cprt) that satisfies PDF/A-3 validators.
fn build_srgb_icc_profile() -> Vec<u8> {
    let mut p = vec![0u8; 128];

    // Header — version 2.1.0
    p[8] = 2;
    p[9] = 0x10;
    // Device class: 'mntr'
    p[12..16].copy_from_slice(b"mntr");
    // Color space: 'RGB '
    p[16..20].copy_from_slice(b"RGB ");
    // PCS: 'XYZ '
    p[20..24].copy_from_slice(b"XYZ ");
    // Date: 2024-01-01
    p[24..26].copy_from_slice(&2024u16.to_be_bytes());
    p[26..28].copy_from_slice(&1u16.to_be_bytes());
    p[28..30].copy_from_slice(&1u16.to_be_bytes());
    // File signature: 'acsp'
    p[36..40].copy_from_slice(b"acsp");
    // PCS illuminant D50 (s15Fixed16: 0.9642, 1.0, 0.8249)
    p[68..72].copy_from_slice(&[0x00, 0x00, 0xF6, 0xD6]);
    p[72..76].copy_from_slice(&[0x00, 0x01, 0x00, 0x00]);
    p[76..80].copy_from_slice(&[0x00, 0x00, 0xD3, 0x2D]);

    // Tag count: 3
    p.extend_from_slice(&3u32.to_be_bytes());

    // Tag table starts at 132, data starts at 132 + 3*12 = 168
    let data_start: u32 = 168;

    // desc tag data: textDescriptionType
    // sig(4) + reserved(4) + ascii_count(4) + ascii("sRGB\0"=5) + unicode_langcode(4)
    // + unicode_count(4,=0) + scriptcode(2+1+67=70)
    let desc_size: u32 = 4 + 4 + 4 + 5 + 4 + 4 + 70;
    let wtpt_offset = data_start + desc_size;
    // XYZType: sig(4) + reserved(4) + xyz(12) = 20
    let wtpt_size: u32 = 20;
    let cprt_offset = wtpt_offset + wtpt_size;
    // textType: sig(4) + reserved(4) + text("PD\0"=3) = 11
    let cprt_size: u32 = 11;

    // Tag table entries (sig, offset, size)
    p.extend_from_slice(b"desc");
    p.extend_from_slice(&data_start.to_be_bytes());
    p.extend_from_slice(&desc_size.to_be_bytes());

    p.extend_from_slice(b"wtpt");
    p.extend_from_slice(&wtpt_offset.to_be_bytes());
    p.extend_from_slice(&wtpt_size.to_be_bytes());

    p.extend_from_slice(b"cprt");
    p.extend_from_slice(&cprt_offset.to_be_bytes());
    p.extend_from_slice(&cprt_size.to_be_bytes());

    // desc tag data (textDescriptionType)
    p.extend_from_slice(b"desc");
    p.extend_from_slice(&[0u8; 4]); // reserved
    p.extend_from_slice(&5u32.to_be_bytes()); // ASCII count including null
    p.extend_from_slice(b"sRGB\0"); // ASCII string
    p.extend_from_slice(&[0u8; 4]); // Unicode language code
    p.extend_from_slice(&0u32.to_be_bytes()); // Unicode count (0 = no Unicode)
    p.extend_from_slice(&[0u8; 70]); // ScriptCode (2 code + 1 count + 67 data)

    // wtpt tag data (XYZType) — D65 white point
    p.extend_from_slice(b"XYZ ");
    p.extend_from_slice(&[0u8; 4]); // reserved
    // D65: X=0.9505, Y=1.0, Z=1.0890 as s15Fixed16
    p.extend_from_slice(&[0x00, 0x00, 0xF3, 0x54]); // 0.9505
    p.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]); // 1.0
    p.extend_from_slice(&[0x00, 0x01, 0x16, 0xCF]); // 1.0890

    // cprt tag data (textType)
    p.extend_from_slice(b"text");
    p.extend_from_slice(&[0u8; 4]); // reserved
    p.extend_from_slice(b"PD\0"); // "PD" = public domain

    // Patch profile size in header
    let size = p.len() as u32;
    p[0..4].copy_from_slice(&size.to_be_bytes());

    p
}
