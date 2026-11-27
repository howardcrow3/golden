//! Documents kept inside the (encrypted) database.
use crate::{Db, Result};
use rusqlite::{params, OptionalExtension};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocRow {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub year: Option<i32>,
    pub added: String,
    pub size: i64,
    /// JSON text of the figures read from the document.
    pub fields: String,
    /// False when no text could be read (for example a scan no engine could read).
    pub has_text: bool,
}

pub struct NewDoc<'a> {
    pub name: &'a str,
    pub kind: &'a str,
    pub year: Option<i32>,
    pub added: &'a str,
    pub fields: &'a str,
    pub body: &'a str,
    pub content: &'a [u8],
}

const COLS: &str = "id, name, kind, year, added, size, fields, length(trim(body)) > 0";

fn row(r: &rusqlite::Row) -> rusqlite::Result<DocRow> {
    Ok(DocRow {
        id: r.get(0)?,
        name: r.get(1)?,
        kind: r.get(2)?,
        year: r.get(3)?,
        added: r.get(4)?,
        size: r.get(5)?,
        fields: r.get(6)?,
        has_text: r.get(7)?,
    })
}

impl Db {
    pub fn add_document(&self, d: &NewDoc) -> Result<DocRow> {
        self.conn.execute(
            "INSERT INTO document (name, kind, year, added, size, fields, body, content)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                d.name,
                d.kind,
                d.year,
                d.added,
                d.content.len() as i64,
                d.fields,
                d.body,
                d.content
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(self.conn.query_row(
            &format!("SELECT {COLS} FROM document WHERE id = ?1"),
            [id],
            row,
        )?)
    }

    /// Newest first; `query` matches the name, kind or text inside.
    pub fn list_documents(&self, query: &str) -> Result<Vec<DocRow>> {
        let like = format!("%{}%", query.trim().replace(['%', '_'], ""));
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLS} FROM document
             WHERE name LIKE ?1 OR kind LIKE ?1 OR body LIKE ?1
             ORDER BY added DESC, id DESC"
        ))?;
        let rows = stmt
            .query_map([like], row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_document(&self, id: i64) -> Result<Option<(String, Vec<u8>)>> {
        Ok(self
            .conn
            .query_row(
                "SELECT name, content FROM document WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?)
    }

    /// The text read from a document (empty when none could be read).
    pub fn document_text(&self, id: i64) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT body FROM document WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .optional()?)
    }

    /// Match a receipt to the transaction it paid for, replacing any earlier match.
    pub fn link_receipt(&self, document: i64, txn: i64) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO receipt_link(document_id, txn_id) VALUES (?1, ?2)",
            [document, txn],
        )?;
        Ok(())
    }

    pub fn unlink_receipt(&self, document: i64) -> Result<bool> {
        Ok(self.conn.execute(
            "DELETE FROM receipt_link WHERE document_id = ?1",
            [document],
        )? > 0)
    }

    /// Every (document id, transaction id) match.
    pub fn receipt_links(&self) -> Result<Vec<(i64, i64)>> {
        let mut st = self
            .conn
            .prepare("SELECT document_id, txn_id FROM receipt_link ORDER BY document_id")?;
        let rows = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Whether a document was removed.
    pub fn delete_document(&self, id: i64) -> Result<bool> {
        Ok(self
            .conn
            .execute("DELETE FROM document WHERE id = ?1", [id])?
            > 0)
    }
}
