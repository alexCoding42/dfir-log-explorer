use crate::{Engine, EngineError, Query, PAGE_SIZE};
use duckdb::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedSearch {
    pub id: u64,
    pub name: String,
    pub version: u32,
    pub query: Query,
}

impl Engine {
    fn has_saved_searches(&self) -> Result<bool, EngineError> {
        Ok(self.connection()?.query_row(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'main' AND table_name = 'saved_searches')",
            [], |row| row.get(0),
        )?)
    }

    /// Optional schema-1 extension: old cases are never modified on open/list.
    pub fn saved_searches(&self) -> Result<Vec<SavedSearch>, EngineError> {
        if !self.has_saved_searches()? {
            return Ok(Vec::new());
        }
        let mut statement = self.connection()?.prepare(
            "SELECT id, substring(payload, 1, 131073) FROM saved_searches ORDER BY id LIMIT 201",
        )?;
        let definitions = statement
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        if definitions.len() > 200 {
            return Err(EngineError::UnsupportedSavedSearch);
        }
        let mut searches = Vec::new();
        for (id, payload) in definitions {
            if payload.len() > 131_072 {
                return Err(EngineError::UnsupportedSavedSearch);
            }
            let search: SavedSearch =
                serde_json::from_str(&payload).map_err(|_| EngineError::UnsupportedSavedSearch)?;
            if id <= 0
                || search.id != id as u64
                || search.version != 1
                || search.name.trim().is_empty()
                || search.name.chars().count() > 120
                || search.query.predicate().is_err()
            {
                return Err(EngineError::UnsupportedSavedSearch);
            }
            searches.push(search);
        }
        Ok(searches)
    }

    fn validate_search_source(&self, query: &Query) -> Result<(), EngineError> {
        if let Some(id) = query.source_id {
            let exists: bool = self.connection()?.query_row(
                "SELECT EXISTS (SELECT 1 FROM sources WHERE id = ?)",
                params![id as i64],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(EngineError::SavedSearchSourceMissing);
            }
        }
        Ok(())
    }

    pub fn saved_search_query(&self, id: u64) -> Result<Query, EngineError> {
        let mut query = self
            .saved_searches()?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or(EngineError::UnsupportedSavedSearch)?
            .query;
        self.validate_search_source(&query)?;
        query.offset = 0;
        query.limit = PAGE_SIZE;
        Ok(query)
    }

    pub fn put_saved_search(
        &self,
        id: Option<u64>,
        name: &str,
        mut query: Query,
    ) -> Result<SavedSearch, EngineError> {
        let name = name.trim();
        query.offset = 0;
        query.limit = PAGE_SIZE;
        query.predicate()?;
        self.validate_search_source(&query)?;
        let existing = self.saved_searches()?;
        if name.is_empty()
            || name.chars().count() > 120
            || existing
                .iter()
                .any(|s| Some(s.id) != id && s.name.to_lowercase() == name.to_lowercase())
            || (id.is_none() && existing.len() >= 200)
            || id.is_some_and(|id| !existing.iter().any(|s| s.id == id))
        {
            return Err(EngineError::InvalidSavedSearch);
        }
        let id = match id {
            Some(id) => id,
            None => existing
                .iter()
                .map(|s| s.id)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .filter(|id| *id <= i64::MAX as u64)
                .ok_or(EngineError::InvalidSavedSearch)?,
        };
        let search = SavedSearch {
            id,
            name: name.into(),
            version: 1,
            query,
        };
        let payload =
            serde_json::to_string(&search).map_err(|_| EngineError::InvalidSavedSearch)?;
        if payload.len() > 131_072 {
            return Err(EngineError::SavedSearchTooLarge);
        }
        let connection = self.connection()?;
        connection.execute_batch("BEGIN")?;
        let result = (|| -> Result<(), EngineError> {
            connection.execute_batch("CREATE TABLE IF NOT EXISTS saved_searches (id BIGINT PRIMARY KEY, payload VARCHAR)")?;
            connection.execute("INSERT INTO saved_searches VALUES (?, ?) ON CONFLICT (id) DO UPDATE SET payload = excluded.payload", params![id as i64, payload])?;
            connection.execute_batch("COMMIT")?;
            Ok(())
        })();
        if result.is_err() {
            let _ = connection.execute_batch("ROLLBACK");
        }
        result?;
        Ok(search)
    }

    pub fn delete_saved_search(&self, id: u64) -> Result<(), EngineError> {
        if !self.saved_searches()?.iter().any(|s| s.id == id) {
            return Err(EngineError::UnsupportedSavedSearch);
        }
        self.connection()?.execute(
            "DELETE FROM saved_searches WHERE id = ?",
            params![id as i64],
        )?;
        Ok(())
    }
}
