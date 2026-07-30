use std::fs;
use std::path::Path;
use std::time::Duration as StdDuration;

use openbrief_core::{FocusSegment, FocusState, FocusTransition};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use time::{Date, Duration, OffsetDateTime, Time, UtcOffset};

mod attention;

pub use attention::{ConfirmedTriage, IngestOutcome, StoredTriageProposal};

const SCHEMA_VERSION: i64 = 2;
const RETENTION_DAYS: i64 = 7;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid time range: {start} to {end}")]
    InvalidRange {
        start: OffsetDateTime,
        end: OffsetDateTime,
    },
    #[error("insert_segment requires a closed segment")]
    OpenSegment,
    #[error("unsupported database schema version {0}")]
    UnsupportedSchemaVersion(i64),
    #[error("stored focus state is invalid: {0}")]
    InvalidFocusState(String),
    #[error("stored timestamp is invalid: {0}")]
    InvalidTimestamp(String),
    #[error("stored segment violates the core contract: {0}")]
    InvalidSegment(#[from] openbrief_core::CoreError),
    #[error("stored JSON is invalid: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("{entity} {id} was already ingested with different content")]
    IdempotencyConflict { entity: &'static str, id: String },
    #[error("{entity} was not found: {id}")]
    NotFound { entity: &'static str, id: String },
    #[error("triage proposal was already confirmed: {0}")]
    AlreadyConfirmed(String),
    #[error("confirmation selected an unknown {entity}: {id}")]
    UnknownSelection { entity: &'static str, id: String },
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// `SQLite` authority for focus segments.
///
/// A `Store` owns one connection. The collector should keep one writer
/// instance, while readers may open independent instances for the same path.
/// Activity slices are projections and are deliberately not persisted.
pub struct Store {
    connection: Connection,
}

impl Store {
    /// Opens or creates a file-backed store.
    ///
    /// # Errors
    ///
    /// Returns an error if the parent or database cannot be secured, opened,
    /// configured, or migrated.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        prepare_database_path(path)?;

        let connection = Connection::open(path)?;
        set_database_file_permissions(path)?;
        configure(&connection, true)?;

        let mut store = Self { connection };
        store.migrate()?;
        Ok(store)
    }

    /// Opens a transient store, primarily for tests.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` cannot open, configure, or migrate it.
    pub fn open_in_memory() -> Result<Self> {
        let connection = Connection::open_in_memory()?;
        configure(&connection, false)?;

        let mut store = Self { connection };
        store.migrate()?;
        Ok(store)
    }

    /// Reads the migrated schema version.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` cannot read the schema pragma.
    pub fn schema_version(&self) -> Result<i64> {
        Ok(self
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))?)
    }

    /// Inserts an already-closed segment and returns its database identity.
    ///
    /// # Errors
    ///
    /// Returns an error for an open segment, an invalid timestamp, or a
    /// transaction failure.
    pub fn insert_segment(&mut self, segment: &FocusSegment) -> Result<i64> {
        let transaction = self.connection.transaction()?;
        let id = insert_closed_segment(&transaction, segment)?;
        transaction.commit()?;
        Ok(id)
    }

    /// Closes the current segment and opens the transition atomically.
    ///
    /// # Errors
    ///
    /// Returns an error for a non-chronological transition, invalid timestamp,
    /// or transaction failure.
    pub fn append_transition(
        &mut self,
        transition: &FocusTransition,
    ) -> Result<Option<FocusSegment>> {
        let transaction = self.connection.transaction()?;
        let closed = close_current(&transaction, transition.at)?;
        let id = insert_open_segment(&transaction, transition)?;
        transaction.execute(
            "UPDATE collector_state SET current_segment_id = ?1 WHERE singleton = 1",
            params![id],
        )?;
        transaction.commit()?;
        Ok(closed)
    }

    /// Closes the current segment, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if `ended_at` does not follow the current start, or if
    /// the transaction fails.
    pub fn close_current_segment(
        &mut self,
        ended_at: OffsetDateTime,
    ) -> Result<Option<FocusSegment>> {
        let transaction = self.connection.transaction()?;
        let closed = close_current(&transaction, ended_at)?;
        transaction.commit()?;
        Ok(closed)
    }

    /// Returns segments that overlap `[start, end)`, clipped to that range.
    ///
    /// A currently-open segment is projected as ending at `end`, so `recent`
    /// queries include work still in progress without persisting a fake end.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid range, query failure, or invalid stored
    /// segment.
    pub fn segments_between(
        &self,
        start: OffsetDateTime,
        end: OffsetDateTime,
    ) -> Result<Vec<FocusSegment>> {
        validate_range(start, end)?;
        let start_ns = timestamp_nanos(start)?;
        let end_ns = timestamp_nanos(end)?;

        let mut statement = self.connection.prepare(
            "SELECT started_at_ns, started_offset_seconds,
                    ended_at_ns, ended_offset_seconds, state, app_id
             FROM focus_segments
             WHERE started_at_ns < ?2
               AND (ended_at_ns IS NULL OR ended_at_ns > ?1)
             ORDER BY started_at_ns, id",
        )?;

        let stored = statement
            .query_map(params![start_ns, end_ns], read_stored_segment)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        stored
            .into_iter()
            .map(|segment| segment.into_core_clipped(start, end))
            .collect()
    }

    /// Deletes every segment overlapping `[start, end)`.
    ///
    /// Slight over-deletion at the two boundaries is intentional: privacy
    /// deletion must not retain a fragment of a segment that overlapped the
    /// requested interval.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid range or transaction failure.
    pub fn delete_range(&mut self, start: OffsetDateTime, end: OffsetDateTime) -> Result<usize> {
        validate_range(start, end)?;
        let transaction = self.connection.transaction()?;
        let deleted = transaction.execute(
            "DELETE FROM focus_segments
             WHERE started_at_ns < ?2
               AND (ended_at_ns IS NULL OR ended_at_ns > ?1)",
            params![timestamp_nanos(start)?, timestamp_nanos(end)?],
        )?;
        transaction.commit()?;
        Ok(deleted)
    }

    /// Deletes every segment overlapping the requested local date.
    ///
    /// # Errors
    ///
    /// Returns an error if the date has no successor or deletion fails.
    pub fn delete_date(&mut self, date: Date, offset: UtcOffset) -> Result<usize> {
        let start = date.with_time(Time::MIDNIGHT).assume_offset(offset);
        let next = date
            .next_day()
            .ok_or_else(|| StoreError::InvalidTimestamp("date has no next day".to_owned()))?;
        let end = next.with_time(Time::MIDNIGHT).assume_offset(offset);
        self.delete_range(start, end)
    }

    /// Removes segments before `cutoff` and trims a segment crossing the cutoff
    /// so no retained row begins outside the retention window.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid timestamp or transaction failure.
    pub fn purge_before(&mut self, cutoff: OffsetDateTime) -> Result<usize> {
        let transaction = self.connection.transaction()?;
        let cutoff_ns = timestamp_nanos(cutoff)?;
        let deleted = transaction.execute(
            "DELETE FROM focus_segments
             WHERE ended_at_ns <= ?1",
            params![cutoff_ns],
        )?;
        let trimmed = transaction.execute(
            "UPDATE focus_segments
             SET started_at_ns = ?1, started_offset_seconds = ?2
             WHERE started_at_ns < ?1
               AND (ended_at_ns IS NULL OR ended_at_ns > ?1)",
            params![cutoff_ns, cutoff.offset().whole_seconds()],
        )?;
        transaction.commit()?;
        Ok(deleted + trimmed)
    }

    /// Applies the seven-day retention window relative to `now`.
    ///
    /// # Errors
    ///
    /// Returns an error if retention cannot be applied.
    pub fn purge_expired(&mut self, now: OffsetDateTime) -> Result<usize> {
        self.purge_before(now - Duration::days(RETENTION_DAYS))
    }

    #[allow(clippy::too_many_lines)]
    fn migrate(&mut self) -> Result<()> {
        let transaction = self.connection.transaction()?;
        let version: i64 =
            transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if version == 0 {
            transaction.execute_batch(
                "CREATE TABLE focus_segments (
                    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
                    started_at_ns          INTEGER NOT NULL,
                    started_offset_seconds INTEGER NOT NULL,
                    ended_at_ns            INTEGER,
                    ended_offset_seconds   INTEGER,
                    state                  TEXT NOT NULL,
                    app_id                 TEXT,
                    CHECK (state IN (
                        'observed', 'excluded', 'idle', 'locked',
                        'paused', 'disabled', 'source_unavailable'
                    )),
                    CHECK (
                        (state = 'observed' AND app_id IS NOT NULL AND length(app_id) > 0)
                        OR
                        (state <> 'observed' AND app_id IS NULL)
                    ),
                    CHECK (
                        (ended_at_ns IS NULL AND ended_offset_seconds IS NULL)
                        OR
                        (ended_at_ns > started_at_ns AND ended_offset_seconds IS NOT NULL)
                    )
                );

                CREATE INDEX focus_segments_time_range
                    ON focus_segments(started_at_ns, ended_at_ns);

                CREATE TABLE collector_state (
                    singleton     INTEGER PRIMARY KEY CHECK (singleton = 1),
                    current_segment_id INTEGER UNIQUE
                        REFERENCES focus_segments(id) ON DELETE SET NULL,
                    privacy_epoch INTEGER NOT NULL DEFAULT 0 CHECK (privacy_epoch >= 0)
                );

                INSERT INTO collector_state (
                    singleton, current_segment_id, privacy_epoch
                ) VALUES (1, NULL, 0);

                PRAGMA user_version = 1;",
            )?;
        } else if version > SCHEMA_VERSION {
            return Err(StoreError::UnsupportedSchemaVersion(version));
        }

        if version < 2 {
            transaction.execute_batch(
                "CREATE TABLE observation_batches (
                    id              TEXT PRIMARY KEY,
                    generated_at_ns INTEGER NOT NULL,
                    payload_json    TEXT NOT NULL
                );

                CREATE INDEX observation_batches_generated
                    ON observation_batches(generated_at_ns DESC);

                CREATE TABLE observations (
                    id              TEXT PRIMARY KEY,
                    batch_id        TEXT NOT NULL
                        REFERENCES observation_batches(id) ON DELETE CASCADE,
                    source          TEXT NOT NULL,
                    occurred_at_ns  INTEGER NOT NULL,
                    payload_json    TEXT NOT NULL
                );

                CREATE INDEX observations_batch
                    ON observations(batch_id, occurred_at_ns DESC);

                CREATE TABLE source_coverages (
                    batch_id        TEXT NOT NULL
                        REFERENCES observation_batches(id) ON DELETE CASCADE,
                    source          TEXT NOT NULL,
                    payload_json    TEXT NOT NULL,
                    PRIMARY KEY (batch_id, source)
                );

                CREATE TABLE brief_proposals (
                    id              TEXT PRIMARY KEY,
                    batch_id        TEXT NOT NULL
                        REFERENCES observation_batches(id),
                    created_at_ns   INTEGER NOT NULL,
                    payload_json    TEXT NOT NULL
                );

                CREATE INDEX brief_proposals_created
                    ON brief_proposals(created_at_ns DESC);

                CREATE TABLE triage_proposals (
                    id                TEXT PRIMARY KEY,
                    brief_proposal_id TEXT NOT NULL
                        REFERENCES brief_proposals(id),
                    created_at_ns     INTEGER NOT NULL,
                    confirmed_at_ns   INTEGER,
                    payload_json      TEXT NOT NULL
                );

                CREATE INDEX triage_proposals_created
                    ON triage_proposals(created_at_ns DESC);

                CREATE TABLE user_decisions (
                    id              TEXT PRIMARY KEY,
                    proposal_id     TEXT NOT NULL
                        REFERENCES triage_proposals(id),
                    payload_json    TEXT NOT NULL
                );

                CREATE TABLE curiosity_captures (
                    id              TEXT PRIMARY KEY,
                    proposal_id     TEXT NOT NULL
                        REFERENCES triage_proposals(id),
                    payload_json    TEXT NOT NULL
                );

                CREATE TABLE return_anchors (
                    id              TEXT PRIMARY KEY,
                    proposal_id     TEXT NOT NULL
                        REFERENCES triage_proposals(id),
                    payload_json    TEXT NOT NULL
                );

                PRAGMA user_version = 2;",
            )?;
        }

        transaction.commit()?;
        Ok(())
    }
}

fn configure(connection: &Connection, file_backed: bool) -> Result<()> {
    connection.pragma_update(None, "foreign_keys", true)?;
    connection.busy_timeout(StdDuration::from_secs(2))?;
    if file_backed {
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
    }
    Ok(())
}

fn prepare_database_path(path: &Path) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "database path has no parent",
        )
    })?;
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    fs::create_dir_all(parent)?;
    set_mode(parent, 0o700)?;
    Ok(())
}

fn set_database_file_permissions(path: &Path) -> Result<()> {
    set_mode(path, 0o600)
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

fn insert_closed_segment(transaction: &Transaction<'_>, segment: &FocusSegment) -> Result<i64> {
    let (started_at_ns, started_offset) = encode_timestamp(segment.started_at)?;
    let ended_at = segment.ended_at.ok_or(StoreError::OpenSegment)?;
    let (ended_at_ns, ended_offset) = encode_timestamp(ended_at)?;
    transaction.execute(
        "INSERT INTO focus_segments (
            started_at_ns, started_offset_seconds,
            ended_at_ns, ended_offset_seconds, state, app_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            started_at_ns,
            started_offset,
            ended_at_ns,
            ended_offset,
            focus_state_name(&segment.state),
            segment.app_id()
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

fn insert_open_segment(transaction: &Transaction<'_>, transition: &FocusTransition) -> Result<i64> {
    let (started_at_ns, started_offset) = encode_timestamp(transition.at)?;
    transaction.execute(
        "INSERT INTO focus_segments (
            started_at_ns, started_offset_seconds,
            ended_at_ns, ended_offset_seconds, state, app_id
         ) VALUES (?1, ?2, NULL, NULL, ?3, ?4)",
        params![
            started_at_ns,
            started_offset,
            focus_state_name(&transition.state),
            transition.app_id()
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

fn close_current(
    transaction: &Transaction<'_>,
    ended_at: OffsetDateTime,
) -> Result<Option<FocusSegment>> {
    let current = transaction
        .query_row(
            "SELECT segment.started_at_ns, segment.started_offset_seconds,
                    segment.ended_at_ns, segment.ended_offset_seconds,
                    segment.state, segment.app_id
             FROM collector_state AS state
             JOIN focus_segments AS segment
               ON segment.id = state.current_segment_id
             WHERE state.singleton = 1",
            [],
            read_stored_segment,
        )
        .optional()?;

    let Some(current) = current else {
        return Ok(None);
    };
    validate_range(current.started_at, ended_at)?;
    let (ended_at_ns, ended_offset) = encode_timestamp(ended_at)?;
    transaction.execute(
        "UPDATE focus_segments
         SET ended_at_ns = ?1, ended_offset_seconds = ?2
         WHERE id = (
             SELECT current_segment_id FROM collector_state WHERE singleton = 1
         )",
        params![ended_at_ns, ended_offset],
    )?;
    transaction.execute(
        "UPDATE collector_state SET current_segment_id = NULL WHERE singleton = 1",
        [],
    )?;
    current.into_core(Some(ended_at))
}

#[derive(Debug)]
struct StoredSegment {
    started_at: OffsetDateTime,
    ended_at: Option<OffsetDateTime>,
    state: FocusState,
    app_id: Option<String>,
}

impl StoredSegment {
    fn into_core(self, ended_at: Option<OffsetDateTime>) -> Result<Option<FocusSegment>> {
        FocusSegment::new(self.started_at, ended_at, self.state, self.app_id)
            .map(Some)
            .map_err(StoreError::from)
    }

    fn into_core_clipped(
        self,
        range_start: OffsetDateTime,
        range_end: OffsetDateTime,
    ) -> Result<FocusSegment> {
        let started_at = self.started_at.max(range_start);
        let ended_at = self.ended_at.unwrap_or(range_end).min(range_end);
        FocusSegment::new(started_at, Some(ended_at), self.state, self.app_id)
            .map_err(StoreError::from)
    }
}

fn read_stored_segment(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredSegment> {
    let started_ns: i64 = row.get(0)?;
    let started_offset: i32 = row.get(1)?;
    let ended_ns: Option<i64> = row.get(2)?;
    let ended_offset: Option<i32> = row.get(3)?;
    let state_name: String = row.get(4)?;
    let app_id: Option<String> = row.get(5)?;

    let started_at =
        decode_timestamp(started_ns, started_offset).map_err(|error| conversion_error(0, error))?;
    let ended_at = match (ended_ns, ended_offset) {
        (Some(nanos), Some(offset)) => {
            Some(decode_timestamp(nanos, offset).map_err(|error| conversion_error(2, error))?)
        }
        (None, None) => None,
        _ => {
            return Err(conversion_error(
                2,
                StoreError::InvalidTimestamp(
                    "end timestamp and offset must both be null or non-null".to_owned(),
                ),
            ));
        }
    };
    let state = parse_focus_state(&state_name).map_err(|error| conversion_error(4, error))?;

    Ok(StoredSegment {
        started_at,
        ended_at,
        state,
        app_id,
    })
}

fn conversion_error(index: usize, error: StoreError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, Box::new(error))
}

fn focus_state_name(state: &FocusState) -> &'static str {
    match state {
        FocusState::Observed => "observed",
        FocusState::Excluded => "excluded",
        FocusState::Idle => "idle",
        FocusState::Locked => "locked",
        FocusState::Paused => "paused",
        FocusState::Disabled => "disabled",
        FocusState::SourceUnavailable => "source_unavailable",
    }
}

fn parse_focus_state(value: &str) -> Result<FocusState> {
    match value {
        "observed" => Ok(FocusState::Observed),
        "excluded" => Ok(FocusState::Excluded),
        "idle" => Ok(FocusState::Idle),
        "locked" => Ok(FocusState::Locked),
        "paused" => Ok(FocusState::Paused),
        "disabled" => Ok(FocusState::Disabled),
        "source_unavailable" => Ok(FocusState::SourceUnavailable),
        other => Err(StoreError::InvalidFocusState(other.to_owned())),
    }
}

fn encode_timestamp(value: OffsetDateTime) -> Result<(i64, i32)> {
    Ok((timestamp_nanos(value)?, value.offset().whole_seconds()))
}

fn timestamp_nanos(value: OffsetDateTime) -> Result<i64> {
    i64::try_from(value.unix_timestamp_nanos()).map_err(|_| {
        StoreError::InvalidTimestamp(format!("{value} is outside SQLite nanosecond range"))
    })
}

fn decode_timestamp(nanos: i64, offset_seconds: i32) -> Result<OffsetDateTime> {
    let offset = UtcOffset::from_whole_seconds(offset_seconds)
        .map_err(|error| StoreError::InvalidTimestamp(error.to_string()))?;
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(nanos))
        .map(|value| value.to_offset(offset))
        .map_err(|error| StoreError::InvalidTimestamp(error.to_string()))
}

fn validate_range(start: OffsetDateTime, end: OffsetDateTime) -> Result<()> {
    if end <= start {
        return Err(StoreError::InvalidRange { start, end });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::{date, datetime};

    fn segment(
        start: OffsetDateTime,
        end: OffsetDateTime,
        state: FocusState,
        app_id: Option<&str>,
    ) -> FocusSegment {
        FocusSegment::new(start, Some(end), state, app_id.map(ToOwned::to_owned)).unwrap()
    }

    #[test]
    fn migration_creates_only_authority_tables() {
        let store = Store::open_in_memory().unwrap();

        assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
        let tables: Vec<String> = {
            let mut statement = store
                .connection
                .prepare(
                    "SELECT name FROM sqlite_schema
                     WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
                     ORDER BY name",
                )
                .unwrap();
            statement
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<std::result::Result<Vec<_>, _>>()
                .unwrap()
        };
        assert_eq!(
            tables,
            vec![
                "brief_proposals",
                "collector_state",
                "curiosity_captures",
                "focus_segments",
                "observation_batches",
                "observations",
                "return_anchors",
                "source_coverages",
                "triage_proposals",
                "user_decisions",
            ]
        );
    }

    #[test]
    fn append_transition_closes_current_segment_transactionally() {
        let mut store = Store::open_in_memory().unwrap();
        let first =
            FocusTransition::observed(datetime!(2026-07-29 10:00:00 +09:00), "ghostty").unwrap();
        let second =
            FocusTransition::observed(datetime!(2026-07-29 10:06:00 +09:00), "firefox").unwrap();

        assert_eq!(store.append_transition(&first).unwrap(), None);
        assert_eq!(
            store.append_transition(&second).unwrap(),
            Some(segment(
                datetime!(2026-07-29 10:00:00 +09:00),
                datetime!(2026-07-29 10:06:00 +09:00),
                FocusState::Observed,
                Some("ghostty"),
            ))
        );
    }

    #[test]
    fn range_query_clips_closed_and_open_segments() {
        let mut store = Store::open_in_memory().unwrap();
        store
            .insert_segment(&segment(
                datetime!(2026-07-29 09:55:00 +09:00),
                datetime!(2026-07-29 10:05:00 +09:00),
                FocusState::Observed,
                Some("ghostty"),
            ))
            .unwrap();
        store
            .append_transition(
                &FocusTransition::gap(datetime!(2026-07-29 10:05:00 +09:00), FocusState::Idle)
                    .unwrap(),
            )
            .unwrap();

        let segments = store
            .segments_between(
                datetime!(2026-07-29 10:00:00 +09:00),
                datetime!(2026-07-29 10:10:00 +09:00),
            )
            .unwrap();

        assert_eq!(
            segments,
            vec![
                segment(
                    datetime!(2026-07-29 10:00:00 +09:00),
                    datetime!(2026-07-29 10:05:00 +09:00),
                    FocusState::Observed,
                    Some("ghostty"),
                ),
                segment(
                    datetime!(2026-07-29 10:05:00 +09:00),
                    datetime!(2026-07-29 10:10:00 +09:00),
                    FocusState::Idle,
                    None,
                ),
            ]
        );
    }

    #[test]
    fn range_delete_removes_only_overlapping_segments() {
        let mut store = Store::open_in_memory().unwrap();
        for (start, end, app_id) in [
            (
                datetime!(2026-07-29 08:00:00 +09:00),
                datetime!(2026-07-29 09:00:00 +09:00),
                "before",
            ),
            (
                datetime!(2026-07-29 10:00:00 +09:00),
                datetime!(2026-07-29 11:00:00 +09:00),
                "overlap",
            ),
            (
                datetime!(2026-07-29 12:00:00 +09:00),
                datetime!(2026-07-29 13:00:00 +09:00),
                "after",
            ),
        ] {
            store
                .insert_segment(&segment(start, end, FocusState::Observed, Some(app_id)))
                .unwrap();
        }

        let deleted = store
            .delete_range(
                datetime!(2026-07-29 09:30:00 +09:00),
                datetime!(2026-07-29 11:30:00 +09:00),
            )
            .unwrap();

        assert_eq!(deleted, 1);
        let retained = store
            .segments_between(
                datetime!(2026-07-29 07:00:00 +09:00),
                datetime!(2026-07-29 14:00:00 +09:00),
            )
            .unwrap();
        assert_eq!(retained.len(), 2);
        assert_eq!(retained[0].app_id(), Some("before"));
        assert_eq!(retained[1].app_id(), Some("after"));
    }

    #[test]
    fn range_delete_cascades_to_current_segment_reference() {
        let mut store = Store::open_in_memory().unwrap();
        store
            .append_transition(
                &FocusTransition::observed(datetime!(2026-07-29 10:00:00 +09:00), "ghostty")
                    .unwrap(),
            )
            .unwrap();

        assert_eq!(
            store
                .delete_range(
                    datetime!(2026-07-29 09:00:00 +09:00),
                    datetime!(2026-07-29 11:00:00 +09:00),
                )
                .unwrap(),
            1
        );
        let current_segment_id: Option<i64> = store
            .connection
            .query_row(
                "SELECT current_segment_id FROM collector_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(current_segment_id, None);
    }

    #[test]
    fn delete_date_uses_the_requested_local_day() {
        let mut store = Store::open_in_memory().unwrap();
        store
            .insert_segment(&segment(
                datetime!(2026-07-28 23:00:00 +09:00),
                datetime!(2026-07-28 23:30:00 +09:00),
                FocusState::Observed,
                Some("ghostty"),
            ))
            .unwrap();
        store
            .insert_segment(&segment(
                datetime!(2026-07-29 09:00:00 +09:00),
                datetime!(2026-07-29 09:30:00 +09:00),
                FocusState::Observed,
                Some("firefox"),
            ))
            .unwrap();

        assert_eq!(
            store
                .delete_date(date!(2026 - 07 - 29), UtcOffset::from_hms(9, 0, 0).unwrap(),)
                .unwrap(),
            1
        );
        assert_eq!(
            store
                .segments_between(
                    datetime!(2026-07-28 00:00:00 +09:00),
                    datetime!(2026-07-30 00:00:00 +09:00),
                )
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn retention_removes_old_rows_and_trims_crossing_rows() {
        let mut store = Store::open_in_memory().unwrap();
        store
            .insert_segment(&segment(
                datetime!(2026-07-20 10:00:00 +09:00),
                datetime!(2026-07-20 11:00:00 +09:00),
                FocusState::Observed,
                Some("old"),
            ))
            .unwrap();
        store
            .insert_segment(&segment(
                datetime!(2026-07-22 09:55:00 +09:00),
                datetime!(2026-07-22 10:05:00 +09:00),
                FocusState::Observed,
                Some("crossing"),
            ))
            .unwrap();

        let affected = store
            .purge_expired(datetime!(2026-07-29 10:00:00 +09:00))
            .unwrap();

        assert_eq!(affected, 2);
        let retained = store
            .segments_between(
                datetime!(2026-07-22 10:00:00 +09:00),
                datetime!(2026-07-22 11:00:00 +09:00),
            )
            .unwrap();
        assert_eq!(retained.len(), 1);
        assert_eq!(
            retained[0].started_at,
            datetime!(2026-07-22 10:00:00 +09:00)
        );
    }

    #[test]
    fn non_observed_identity_is_null_in_storage() {
        let mut store = Store::open_in_memory().unwrap();
        let excluded = segment(
            datetime!(2026-07-29 10:00:00 +09:00),
            datetime!(2026-07-29 10:05:00 +09:00),
            FocusState::Excluded,
            Some("signal"),
        );
        store.insert_segment(&excluded).unwrap();

        let app_id: Option<String> = store
            .connection
            .query_row("SELECT app_id FROM focus_segments", [], |row| row.get(0))
            .unwrap();
        assert_eq!(app_id, None);
    }
}
