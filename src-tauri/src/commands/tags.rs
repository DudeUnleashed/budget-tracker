use crate::db::Db;
use crate::models::Tag;
use rusqlite::{params, Row};
use tauri::State;

#[tauri::command]
pub fn create_tag(db: State<Db>, name: String, color: Option<String>) -> Result<Tag, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO tags (name, color) VALUES (?1, ?2)",
        params![name, color],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, name, color FROM tags WHERE id = ?1",
        params![id],
        row_to_tag,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_tag(db: State<Db>, id: i64, name: String, color: Option<String>) -> Result<Tag, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE tags SET name = ?1, color = ?2 WHERE id = ?3",
        params![name, color, id],
    )
    .map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, name, color FROM tags WHERE id = ?1",
        params![id],
        row_to_tag,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_tags(db: State<Db>) -> Result<Vec<Tag>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, color FROM tags ORDER BY name")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_tag).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn row_to_tag(row: &Row) -> rusqlite::Result<Tag> {
    Ok(Tag {
        id: row.get(0)?,
        name: row.get(1)?,
        color: row.get(2)?,
    })
}
