use anyhow::Result;
use chrono::Utc;
use fp_domain::{ConnectionState, DeviceSummary, StepEvent};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct StorageManager {
    conn: Arc<Mutex<Connection>>,
}

impl StorageManager {
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let sm = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        sm.run_migrations()?;
        Ok(sm)
    }

    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let sm = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        sm.run_migrations()?;
        Ok(sm)
    }

    pub fn run_migrations(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;

            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY,
                adb_serial TEXT UNIQUE,
                alias TEXT NOT NULL,
                android_api INTEGER,
                model TEXT,
                width_px INTEGER NOT NULL,
                height_px INTEGER NOT NULL,
                rotation_deg INTEGER NOT NULL,
                connection_state TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                draft_json TEXT NOT NULL,
                draft_updated_at TEXT NOT NULL,
                archived_at TEXT
            );

            CREATE TABLE IF NOT EXISTS workflow_versions (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL REFERENCES workflows(id),
                version_no INTEGER NOT NULL,
                graph_json TEXT NOT NULL,
                sha256 TEXT NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(workflow_id, version_no)
            );

            CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                alias TEXT NOT NULL,
                game_profile_id TEXT,
                secret_ref TEXT,
                state TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                device_id TEXT NOT NULL REFERENCES devices(id),
                account_id TEXT REFERENCES accounts(id),
                workflow_version_id TEXT NOT NULL REFERENCES workflow_versions(id),
                priority INTEGER NOT NULL DEFAULT 0,
                state TEXT NOT NULL,
                fencing_generation INTEGER NOT NULL DEFAULT 0,
                current_node_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS step_events (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL REFERENCES jobs(id),
                node_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                attempt_no INTEGER NOT NULL,
                at TEXT NOT NULL,
                payload_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS checkpoints (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL REFERENCES jobs(id),
                node_id TEXT NOT NULL,
                state_json TEXT NOT NULL,
                evidence_ref TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS device_leases (
                device_id TEXT PRIMARY KEY REFERENCES devices(id),
                job_id TEXT NOT NULL REFERENCES jobs(id),
                generation INTEGER NOT NULL,
                expires_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_jobs_state_priority ON jobs(state, priority DESC, created_at);
            CREATE INDEX IF NOT EXISTS idx_events_job_time ON step_events(job_id, at);
            ",
        )?;

        Ok(())
    }

    pub fn upsert_device(&self, dev: &DeviceSummary) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let state_str = serde_json::to_string(&dev.connection_state)?.replace('"', "");
        conn.execute(
            "INSERT INTO devices (id, adb_serial, alias, android_api, model, width_px, height_px, rotation_deg, connection_state, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                alias = excluded.alias,
                android_api = excluded.android_api,
                model = excluded.model,
                width_px = excluded.width_px,
                height_px = excluded.height_px,
                rotation_deg = excluded.rotation_deg,
                connection_state = excluded.connection_state,
                updated_at = excluded.updated_at;",
            params![
                dev.id,
                dev.adb_serial,
                dev.alias,
                dev.android_api,
                dev.model,
                dev.width_px,
                dev.height_px,
                dev.rotation_deg,
                state_str,
                dev.updated_at.to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn list_devices(&self) -> Result<Vec<DeviceSummary>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, adb_serial, alias, android_api, model, width_px, height_px, rotation_deg, connection_state, updated_at FROM devices")?;
        let rows = stmt.query_map([], |row| {
            let state_str: String = row.get(8)?;
            let conn_state = match state_str.as_str() {
                "ready" => ConnectionState::Ready,
                "unauthorized" => ConnectionState::Unauthorized,
                "authorizing" => ConnectionState::Authorizing,
                "busy" => ConnectionState::Busy,
                "recovering" => ConnectionState::Recovering,
                _ => ConnectionState::Disconnected,
            };
            let updated_at_str: String = row.get(9)?;
            let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(DeviceSummary {
                id: row.get(0)?,
                adb_serial: row.get(1)?,
                alias: row.get(2)?,
                android_api: row.get(3)?,
                model: row.get(4)?,
                width_px: row.get(5)?,
                height_px: row.get(6)?,
                rotation_deg: row.get(7)?,
                connection_state: conn_state,
                updated_at,
            })
        })?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
    }

    pub fn save_workflow_draft(&self, id: &str, name: &str, draft_json: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO workflows (id, name, draft_json, draft_updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                draft_json = excluded.draft_json,
                draft_updated_at = excluded.draft_updated_at;",
            params![id, name, draft_json, now],
        )?;
        Ok(())
    }

    pub fn get_workflow_draft(&self, id: &str) -> Result<Option<(String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT name, draft_json FROM workflows WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some((row.get(0)?, row.get(1)?)))
        } else {
            Ok(None)
        }
    }

    pub fn insert_step_event(&self, event: &StepEvent) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let type_str = serde_json::to_string(&event.event_type)?.replace('"', "");
        conn.execute(
            "INSERT INTO step_events (id, job_id, node_id, event_type, attempt_no, at, payload_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                event.id,
                event.job_id,
                event.node_id,
                type_str,
                event.attempt_no,
                event.at.to_rfc3339(),
                event.payload_json
            ],
        )?;
        Ok(())
    }

    pub fn list_step_events(&self, job_id: Option<&str>) -> Result<Vec<StepEvent>> {
        let conn = self.conn.lock().unwrap();
        let query = if job_id.is_some() {
            "SELECT id, job_id, node_id, event_type, attempt_no, at, payload_json FROM step_events WHERE job_id = ?1 ORDER BY at ASC"
        } else {
            "SELECT id, job_id, node_id, event_type, attempt_no, at, payload_json FROM step_events ORDER BY at DESC LIMIT 100"
        };

        let mut stmt = conn.prepare(query)?;
        let map_row = |row: &rusqlite::Row| {
            let type_str: String = row.get(3)?;
            let event_type = match type_str.as_str() {
                "command_sent" => fp_domain::StepEventType::CommandSent,
                "device_ack" => fp_domain::StepEventType::DeviceAck,
                "verification_passed" => fp_domain::StepEventType::VerificationPassed,
                "verification_failed" => fp_domain::StepEventType::VerificationFailed,
                "checkpoint_saved" => fp_domain::StepEventType::CheckpointSaved,
                _ => fp_domain::StepEventType::Unknown,
            };

            let at_str: String = row.get(5)?;
            let at = chrono::DateTime::parse_from_rfc3339(&at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(StepEvent {
                id: row.get(0)?,
                job_id: row.get(1)?,
                node_id: row.get(2)?,
                event_type,
                attempt_no: row.get(4)?,
                at,
                payload_json: row.get(6)?,
            })
        };

        let events: Result<Vec<StepEvent>, rusqlite::Error> = if let Some(jid) = job_id {
            stmt.query_map(params![jid], map_row)?.collect()
        } else {
            stmt.query_map([], map_row)?.collect()
        };

        Ok(events?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_in_memory_migrations_and_crud() {
        let storage = StorageManager::new_in_memory().unwrap();
        let dev = DeviceSummary {
            id: "dev-1".to_string(),
            adb_serial: "serial-123".to_string(),
            alias: "Pixel 6".to_string(),
            android_api: Some(33),
            model: Some("Pixel 6".to_string()),
            width_px: 1080,
            height_px: 2400,
            rotation_deg: 0,
            connection_state: ConnectionState::Ready,
            updated_at: Utc::now(),
        };

        storage.upsert_device(&dev).unwrap();
        let devices = storage.list_devices().unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].alias, "Pixel 6");

        storage
            .save_workflow_draft("wf-1", "Quest Workflow", "{\"nodes\":[]}")
            .unwrap();
        let (name, draft) = storage.get_workflow_draft("wf-1").unwrap().unwrap();
        assert_eq!(name, "Quest Workflow");
        assert_eq!(draft, "{\"nodes\":[]}");
    }
}
