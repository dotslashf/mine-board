use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension};

use crate::errors::{Error, Result};
use crate::models::{
    Category, NewCategory, NewSoundClip, SoundClip, UpdateSoundClip,
};

pub struct Repository {
    conn: Connection,
}

impl Repository {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    // ---------- Categories ----------

    pub fn list_categories(&self) -> Result<Vec<Category>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, sort_order, is_default FROM categories ORDER BY sort_order, name")?;
        let rows = stmt.query_map([], |row| {
            Ok(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                sort_order: row.get(2)?,
                is_default: row.get::<_, i64>(3)? != 0,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn get_category(&self, id: i64) -> Result<Category> {
        self.conn
            .query_row(
                "SELECT id, name, sort_order, is_default FROM categories WHERE id = ?1",
                [id],
                |row| {
                    Ok(Category {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        sort_order: row.get(2)?,
                        is_default: row.get::<_, i64>(3)? != 0,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("category {id}")))
    }

    pub fn create_category(&self, category: NewCategory) -> Result<Category> {
        let sort_order = self.conn.query_row(
            "SELECT COALESCE(MAX(sort_order) + 1, 0) FROM categories",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        self.conn.execute(
            "INSERT INTO categories (name, sort_order, is_default) VALUES (?1, ?2, ?3)",
            params![category.name, sort_order, category.is_default as i64],
        )?;
        self.get_category(self.conn.last_insert_rowid())
    }

    pub fn update_category_name(&self, id: i64, name: &str) -> Result<Category> {
        self.conn.execute(
            "UPDATE categories SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
        self.get_category(id)
    }

    pub fn delete_category(&self, id: i64) -> Result<()> {
        let affected = self
            .conn
            .execute("DELETE FROM categories WHERE id = ?1 AND is_default = 0", [id])?;
        if affected == 0 {
            // Either the row doesn't exist, or it is the protected default category.
            if self.get_category(id).is_ok() {
                return Err(Error::InvalidInput(
                    "the default category cannot be deleted".into(),
                ));
            }
            return Err(Error::NotFound(format!("category {id}")));
        }
        Ok(())
    }

    // ---------- Sound clips ----------

    fn clip_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SoundClip> {
        Ok(SoundClip {
            id: row.get(0)?,
            name: row.get(1)?,
            category_id: row.get(2)?,
            file_path: row.get(3)?,
            duration_ms: row.get(4)?,
            volume: row.get(5)?,
            shortcut: row.get(6)?,
            sort_order: row.get(7)?,
            enabled: row.get(8)?,
        })
    }

    fn select_columns() -> &'static str {
        "id, name, category_id, file_path, duration_ms, volume, shortcut, sort_order, enabled"
    }

    pub fn list_clips(&self, category_id: Option<i64>) -> Result<Vec<SoundClip>> {
        let sql = match category_id {
            Some(_) => format!(
                "SELECT {} FROM sound_clips WHERE category_id = ?1 ORDER BY sort_order, name",
                Self::select_columns()
            ),
            None => format!(
                "SELECT {} FROM sound_clips ORDER BY sort_order, name",
                Self::select_columns()
            ),
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = match category_id {
            Some(id) => stmt.query_map([id], Self::clip_from_row)?,
            None => stmt.query_map([], Self::clip_from_row)?,
        };
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn get_clip(&self, id: i64) -> Result<SoundClip> {
        let sql = format!(
            "SELECT {} FROM sound_clips WHERE id = ?1",
            Self::select_columns()
        );
        self.conn
            .query_row(&sql, [id], Self::clip_from_row)
            .optional()?
            .ok_or_else(|| Error::NotFound(format!("sound clip {id}")))
    }

    pub fn create_clip(&self, clip: NewSoundClip) -> Result<SoundClip> {
        let sort_order = self.conn.query_row(
            "SELECT COALESCE(MAX(sort_order) + 1, 0) FROM sound_clips",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        self.conn.execute(
            "INSERT INTO sound_clips (name, category_id, file_path, duration_ms, volume, shortcut, sort_order, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                clip.name,
                clip.category_id,
                clip.file_path,
                clip.duration_ms,
                clip.volume,
                clip.shortcut,
                sort_order,
                clip.enabled as i64
            ],
        )?;
        self.get_clip(self.conn.last_insert_rowid())
    }

    pub fn update_clip(&self, id: i64, update: UpdateSoundClip) -> Result<SoundClip> {
        let current = self.get_clip(id)?;
        let name = update.name.unwrap_or(current.name);
        let category_id = match update.category_id {
            // field present: Some(id) sets it, None / Some(0) clears it
            Some(value) => value.and_then(|cid| if cid != 0 { Some(cid) } else { None }),
            // field absent: keep current
            None => current.category_id,
        };
        let file_path = update.file_path.unwrap_or(current.file_path);
        let duration_ms = update.duration_ms.unwrap_or(current.duration_ms);
        let volume = update.volume.unwrap_or(current.volume);
        let shortcut = update.shortcut.unwrap_or(current.shortcut);
        let sort_order = update.sort_order.unwrap_or(current.sort_order);
        let enabled = update.enabled.unwrap_or(current.enabled);

        self.conn.execute(
            "UPDATE sound_clips
             SET name = ?1, category_id = ?2, file_path = ?3, duration_ms = ?4,
                 volume = ?5, shortcut = ?6, sort_order = ?7, enabled = ?8
             WHERE id = ?9",
            params![
                name, category_id, file_path, duration_ms, volume, shortcut, sort_order, enabled as i64, id
            ],
        )?;
        self.get_clip(id)
    }

    pub fn delete_clip(&self, id: i64) -> Result<()> {
        let affected = self.conn.execute("DELETE FROM sound_clips WHERE id = ?1", [id])?;
        if affected == 0 {
            return Err(Error::NotFound(format!("sound clip {id}")));
        }
        Ok(())
    }

    // ---------- Settings ----------

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [key],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn all_settings(&self) -> Result<HashMap<String, String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
        rows.collect::<std::result::Result<HashMap<_, _>, _>>()
            .map_err(Into::into)
    }

    // ---------- Tests ----------

    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        crate::db::migrations::run(&conn)?;
        Ok(Self::new(conn))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> Repository {
        Repository::in_memory().unwrap()
    }

    fn new_clip(name: &str) -> NewSoundClip {
        NewSoundClip {
            name: name.into(),
            category_id: None,
            file_path: format!("/tmp/{name}.wav"),
            duration_ms: 1000,
            volume: 1.0,
            shortcut: None,
            enabled: true,
        }
    }

    #[test]
    fn category_crud() {
        let repo = repo();
        let default = repo.list_categories().unwrap().remove(0);
        assert!(default.is_default);

        let cat = repo
            .create_category(NewCategory {
                name: "Effects".into(),
                is_default: false,
            })
            .unwrap();
        assert_eq!(cat.name, "Effects");
        assert!(cat.sort_order > default.sort_order);

        assert!(repo.delete_category(cat.id).is_ok());
        // default cannot be deleted
        assert!(repo.delete_category(default.id).is_err());
    }

    #[test]
    fn clip_crud_with_category() {
        let repo = repo();
        let cat = repo
            .create_category(NewCategory {
                name: "FAA".into(),
                is_default: false,
            })
            .unwrap();

        let mut n = new_clip("cleared");
        n.category_id = Some(cat.id);
        n.shortcut = Some("ctrl+1".into());
        let clip = repo.create_clip(n).unwrap();
        assert_eq!(clip.category_id, Some(cat.id));

        assert_eq!(repo.list_clips(Some(cat.id)).unwrap().len(), 1);
        assert_eq!(repo.list_clips(None).unwrap().len(), 1);

        let updated = repo
            .update_clip(
                clip.id,
                UpdateSoundClip {
                    name: Some("cleared for takeoff".into()),
                    volume: Some(0.5),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(updated.name, "cleared for takeoff");
        assert_eq!(updated.volume, 0.5);

        repo.delete_clip(clip.id).unwrap();
        assert!(repo.list_clips(None).unwrap().is_empty());
    }

    #[test]
    fn deleting_category_nulls_clip_category() {
        let repo = repo();
        let cat = repo
            .create_category(NewCategory {
                name: "Temp".into(),
                is_default: false,
            })
            .unwrap();
        let mut n = new_clip("x");
        n.category_id = Some(cat.id);
        let clip = repo.create_clip(n).unwrap();

        repo.delete_category(cat.id).unwrap();
        let clip = repo.get_clip(clip.id).unwrap();
        assert_eq!(clip.category_id, None);
    }

    #[test]
    fn settings_roundtrip() {
        let repo = repo();
        assert_eq!(repo.get_setting("audio.master_volume").unwrap(), None);
        repo.set_setting("audio.master_volume", "0.9").unwrap();
        assert_eq!(
            repo.get_setting("audio.master_volume").unwrap().as_deref(),
            Some("0.9")
        );
        repo.set_setting("audio.master_volume", "1.0").unwrap();
        let all = repo.all_settings().unwrap();
        assert_eq!(all.get("audio.master_volume").map(String::as_str), Some("1.0"));
    }
}
