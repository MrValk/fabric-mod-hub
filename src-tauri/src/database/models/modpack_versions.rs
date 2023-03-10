use derive_new::new;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::{error::Error, marker::PhantomData};

use crate::database::errors;

use super::{ModVersion, NotSaved, Saved};

#[derive(new, Serialize)]
pub struct ModpackVersion<State = NotSaved> {
    #[new(default)]
    pub id: Option<i64>,
    pub modpack_id: i64,
    pub game_version: String,
    #[new(default)]
    pub installed: bool,
    #[new(default)]
    pub loaded: bool,
    state: PhantomData<State>,
}

impl ModpackVersion<NotSaved> {
    pub fn save(self, db: &mut Connection) -> Result<ModpackVersion<Saved>, Box<dyn Error>> {
        let create_modpack_version = include_str!("../../../sql/modpack_versions/create.sql");

        let tx = db.transaction()?;

        let id = match tx.execute(
            create_modpack_version,
            params![self.modpack_id, self.game_version],
        ) {
            Ok(_) => tx.last_insert_rowid(),
            Err(err) => {
                if !errors::is_constraint_err(&err) {
                    return Err(err.into());
                }

                tx.query_row(
                    include_str!("../../../sql/modpack_versions/id_from_unique.sql"),
                    params![self.modpack_id, self.game_version],
                    |row| row.get(0),
                )?
            }
        };

        tx.commit()?;

        Ok(ModpackVersion {
            id: Some(id),
            modpack_id: self.modpack_id,
            game_version: self.game_version,
            installed: self.installed,
            loaded: self.loaded,
            state: PhantomData::<Saved>,
        })
    }
}

impl ModpackVersion {
    pub fn load(modpack_version_id: i64, db: &mut Connection) -> Result<(), Box<dyn Error>> {
        let unload_all_modpack_versions =
            include_str!("../../../sql/modpack_versions/unload_all.sql");
        let load_modpack_version = include_str!("../../../sql/modpack_versions/load.sql");

        let tx = db.transaction()?;

        tx.execute(unload_all_modpack_versions, [])?;
        tx.execute(load_modpack_version, params![modpack_version_id])?;

        tx.commit()?;

        Ok(())
    }

    pub fn unload_all(db: &Connection) -> Result<(), Box<dyn Error>> {
        let unload_all_modpack_versions =
            include_str!("../../../sql/modpack_versions/unload_all.sql");

        db.execute(unload_all_modpack_versions, [])?;

        Ok(())
    }
}

impl ModpackVersion<Saved> {
    pub fn get_mod_versions(
        &self,
        db: &Connection,
    ) -> Result<Vec<ModVersion<Saved>>, Box<dyn Error>> {
        let get_mod_versions =
            include_str!("../../../sql/mod_versions/from_modpack_version_id.sql");

        let mut stmt = db.prepare(get_mod_versions)?;
        let rows = stmt.query_map(params![self.id], |row| {
            Ok(ModVersion {
                id: Some(row.get(0)?),
                mod_id: row.get(1)?,
                version_id: row.get(2)?,
                game_version: row.get(3)?,
                download_url: row.get(4)?,
                dependency_of: row.get(5)?,
                state: PhantomData::<Saved>,
            })
        })?;

        let mut mod_versions = Vec::new();

        for row in rows {
            mod_versions.push(row?);
        }

        Ok(mod_versions)
    }
}
