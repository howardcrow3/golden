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

const COLS: &str = "id, name, kind, year, added, size, fields";

fn row(r: &rusqlite::Row) -> rusqlite::Result<DocRow> {
    Ok(DocRow {
        id: r.get(0)?,
        name: r.get(1)?,
        kind: r.get(2)?,
        year: r.get(3)?,
        added: r.get(4)?,
        size: r.get(5)?,
        fields: r.get(6)?,
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

    /// Whether a document was removed.
    pub fn delete_document(&self, id: i64) -> Result<bool> {
        Ok(self
            .conn
            .execute("DELETE FROM document WHERE id = ?1", [id])?
            > 0)
    }
}
