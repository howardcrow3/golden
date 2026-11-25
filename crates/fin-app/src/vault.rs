//! A vault for tax forms, statements and other papers. Files live inside the encrypted
//! database (never in a plain folder), Golden reads what it can from each one on this computer,
//! and nothing leaves it.
use crate::App;
use fin_db::documents::{DocRow, NewDoc};
use fin_ingest::docclass::recognise;
use serde::Deserialize;
use serde::Serialize;

/// Largest file the vault accepts.
pub const MAX_BYTES: usize = 25 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocField {
    pub label: String,
    pub cents: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentRow {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub year: Option<i32>,
    pub added: String,
    pub size: i64,
    pub fields: Vec<DocField>,
}

pub use fin_ingest::ocr::Ocr;

fn is_picture(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8, 0xFF]) || bytes.starts_with(&[0x89, b'P', b'N', b'G'])
}

fn read_pictures(ocr: &dyn Ocr, pictures: &[&[u8]]) -> String {
    pictures
        .iter()
        .filter_map(|p| ocr.read_image(p).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

fn text_of(bytes: &[u8], ocr: Option<&dyn Ocr>) -> String {
    if bytes.starts_with(b"%PDF") {
        if let Ok(text) = fin_ingest::pdf::extract_text(bytes) {
            return text;
        }
        // No text layer: a scan. Read the page pictures when an OCR engine is available.
        let Some(ocr) = ocr else {
            return String::new();
        };
        let jpegs = fin_ingest::pdf::embedded_jpegs(bytes);
        let refs: Vec<&[u8]> = jpegs.iter().map(|j| j.as_slice()).collect();
        return read_pictures(ocr, &refs);
    }
    if is_picture(bytes) {
        return ocr.map(|o| read_pictures(o, &[bytes])).unwrap_or_default();
    }
    // Plain text, CSV and the like; other binary files have no readable text.
    match std::str::from_utf8(bytes) {
        Ok(s) if !s.contains('\0') => s.to_string(),
        _ => String::new(),
    }
}

fn view(r: DocRow) -> DocumentRow {
    DocumentRow {
        id: r.id,
        name: r.name,
        kind: r.kind,
        year: r.year,
        added: r.added,
        size: r.size,
        fields: serde_json::from_str(&r.fields).unwrap_or_default(),
    }
}

impl App {
    /// Use the operating system's text recognition for scans and photos (or stop with `None`).
    pub fn set_ocr(&mut self, ocr: Option<std::sync::Arc<dyn Ocr>>) {
        self.ocr = ocr;
    }

    /// Put a file in the vault and read what it can. Returns the stored row.
    pub fn vault_add(&self, name: &str, bytes: &[u8]) -> Result<DocumentRow, String> {
        self.require_edit()?;
        let name = name.trim();
        if name.is_empty() {
            return Err("Give the document a name.".into());
        }
        if bytes.is_empty() {
            return Err("That file is empty.".into());
        }
        if bytes.len() > MAX_BYTES {
            return Err("That file is over 25 MB.".into());
        }
        let body = text_of(bytes, self.ocr.as_deref());
        let found = recognise(&body);
        let fields: Vec<DocField> = found
            .fields
            .iter()
            .map(|f| DocField {
                label: f.label.to_string(),
                cents: f.amount.cents(),
            })
            .collect();
        let added = (self.clock)().format("%Y-%m-%d").to_string();
        let fields = serde_json::to_string(&fields).map_err(|e| e.to_string())?;
        self.db
            .add_document(&NewDoc {
                name,
                kind: found.kind.label(),
                year: found.year,
                added: &added,
                fields: &fields,
                body: &body,
                content: bytes,
            })
            .map(view)
            .map_err(|e| e.to_string())
    }

    /// Documents, newest first. `query` matches names, kinds and the text inside.
    pub fn vault_list(&self, query: &str) -> Result<Vec<DocumentRow>, String> {
        Ok(self
            .db
            .list_documents(query)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(view)
            .collect())
    }

    /// The stored file's name and bytes.
    pub fn vault_get(&self, id: i64) -> Result<(String, Vec<u8>), String> {
        self.db
            .get_document(id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "That document isn't in the vault.".to_string())
    }

    pub fn vault_delete(&self, id: i64) -> Result<(), String> {
        self.require_edit()?;
        if self.db.delete_document(id).map_err(|e| e.to_string())? {
            Ok(())
        } else {
            Err("That document isn't in the vault.".into())
        }
    }
}
