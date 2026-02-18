use lopdf::{Document, Object};

use crate::core::RechnungError;

/// Extract the Factur-X/ZUGFeRD XML from a PDF.
///
/// Searches for `factur-x.xml` (or `zugferd-invoice.xml` for older versions)
/// in the PDF's embedded files. Returns the XML as a string.
pub fn extract_from_pdf(pdf_bytes: &[u8]) -> Result<String, RechnungError> {
    let doc = Document::load_mem(pdf_bytes)
        .map_err(|e| RechnungError::Builder(format!("failed to load PDF: {e}")))?;

    // Try extraction via Names > EmbeddedFiles first, then via AF array
    extract_via_names(&doc)
        .or_else(|_| extract_via_af(&doc))
        .map_err(|e| RechnungError::Builder(format!("no ZUGFeRD/Factur-X XML found in PDF: {e}")))
}

fn extract_via_names(doc: &Document) -> Result<String, String> {
    let catalog = doc.catalog().map_err(|e| e.to_string())?;

    let names_obj = catalog.get(b"Names").map_err(|e| e.to_string())?;
    let names_dict = resolve_dict(doc, names_obj)?;

    let ef_obj = names_dict
        .get(b"EmbeddedFiles")
        .map_err(|e| e.to_string())?;
    let ef_dict = resolve_dict(doc, ef_obj)?;

    let names_array = ef_dict
        .get(b"Names")
        .map_err(|e| e.to_string())?
        .as_array()
        .map_err(|e| e.to_string())?;

    // Names array: [name1, ref1, name2, ref2, ...]
    for chunk in names_array.chunks(2) {
        if chunk.len() < 2 {
            continue;
        }

        let name = obj_to_string(&chunk[0]).unwrap_or_default();
        if is_facturx_filename(&name) {
            return extract_xml_from_filespec(doc, &chunk[1]);
        }
    }

    Err("factur-x.xml not found in EmbeddedFiles name tree".to_string())
}

fn extract_via_af(doc: &Document) -> Result<String, String> {
    let catalog = doc.catalog().map_err(|e| e.to_string())?;

    let af_obj = catalog.get(b"AF").map_err(|e| e.to_string())?;
    let af_array = af_obj.as_array().map_err(|e| e.to_string())?;

    for obj in af_array {
        let fs_id = obj.as_reference().map_err(|e| e.to_string())?;
        let fs_dict = doc.get_dictionary(fs_id).map_err(|e| e.to_string())?;

        // Check filename from UF or F
        let fname = fs_dict
            .get(b"UF")
            .or_else(|_| fs_dict.get(b"F"))
            .ok()
            .and_then(obj_to_string)
            .unwrap_or_default();

        if is_facturx_filename(&fname) {
            return extract_xml_from_filespec_dict(doc, fs_dict);
        }
    }

    Err("factur-x.xml not found in AF array".to_string())
}

fn extract_xml_from_filespec(doc: &Document, obj: &Object) -> Result<String, String> {
    let fs_id = obj.as_reference().map_err(|e| e.to_string())?;
    let fs_dict = doc.get_dictionary(fs_id).map_err(|e| e.to_string())?;
    extract_xml_from_filespec_dict(doc, fs_dict)
}

fn extract_xml_from_filespec_dict(
    doc: &Document,
    fs_dict: &lopdf::Dictionary,
) -> Result<String, String> {
    let ef_obj = fs_dict.get(b"EF").map_err(|e| e.to_string())?;
    let ef_dict = resolve_dict(doc, ef_obj)?;

    let f_obj = ef_dict.get(b"F").map_err(|e| e.to_string())?;
    let stream_obj = resolve_obj(doc, f_obj)?;
    let stream = stream_obj.as_stream().map_err(|e| e.to_string())?;

    // decompressed_content() fails if no Filter key exists (uncompressed stream),
    // so fall back to raw content in that case.
    let content = stream
        .decompressed_content()
        .unwrap_or_else(|_| stream.content.clone());
    String::from_utf8(content).map_err(|e| e.to_string())
}

fn resolve_dict<'a>(doc: &'a Document, obj: &'a Object) -> Result<&'a lopdf::Dictionary, String> {
    match obj {
        Object::Reference(id) => doc.get_dictionary(*id).map_err(|e| e.to_string()),
        Object::Dictionary(d) => Ok(d),
        _ => Err("expected dictionary or reference".to_string()),
    }
}

fn resolve_obj<'a>(doc: &'a Document, obj: &'a Object) -> Result<&'a Object, String> {
    match obj {
        Object::Reference(id) => doc.get_object(*id).map_err(|e| e.to_string()),
        other => Ok(other),
    }
}

fn obj_to_string(obj: &Object) -> Option<String> {
    match obj {
        Object::String(bytes, _) => String::from_utf8(bytes.clone()).ok(),
        _ => None,
    }
}

fn is_facturx_filename(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("factur-x") || lower.contains("zugferd")
}
