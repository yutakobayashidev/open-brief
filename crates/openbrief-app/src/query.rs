use std::path::Path;

use openbrief_core::{
    ContextBrief, ContextRange, FocusSegment, around_range, build_slices, recent_range,
};
use openbrief_store::Store;
use serde::Serialize;
use time::{Date, Duration, OffsetDateTime, Time, UtcOffset};

use crate::{
    AppPaths, CollectorStatus, Config, ConfigError, ControlRequest, ControlResponse,
    LocalControlClient, LocalControlError, PathsError, RecordingStatus, ServiceError,
    ServiceManager,
};

#[derive(Debug, Clone)]
pub struct ContextService {
    paths: AppPaths,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContextDetail {
    #[serde(with = "time::serde::rfc3339")]
    pub start: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub end: OffsetDateTime,
    pub segments: Vec<FocusSegment>,
}

impl ContextService {
    pub fn discover() -> Result<Self, QueryError> {
        Ok(Self {
            paths: AppPaths::discover()?,
        })
    }

    pub fn enable(&self, executable: &Path) -> Result<Config, QueryError> {
        let config = Config::load_or_create(&self.paths.config_file)?;
        let _store = Store::open(&self.paths.database_file)?;
        ServiceManager::new(&self.paths.systemd_unit).install_and_start(executable)?;
        Ok(config)
    }

    pub fn disable(&self) -> Result<(), QueryError> {
        ServiceManager::new(&self.paths.systemd_unit).stop_and_disable()?;
        Ok(())
    }

    pub fn status(&self) -> Result<CollectorStatus, QueryError> {
        if !self.paths.control_socket.exists() {
            return Ok(CollectorStatus {
                control_protocol_version: openbrief_protocol::CONTROL_PROTOCOL_VERSION,
                schema_version: 1,
                recording: RecordingStatus::Disabled,
                last_window_event_at: None,
                paused_until: None,
                source_available: false,
            });
        }
        match self.control(&ControlRequest::Status)? {
            ControlResponse::Status(status) => Ok(status),
            response => Err(QueryError::UnexpectedControlResponse(Box::new(response))),
        }
    }

    pub fn pause(&self, until: Option<OffsetDateTime>) -> Result<(), QueryError> {
        expect_ok(self.control(&ControlRequest::Pause { until })?)
    }

    pub fn resume(&self) -> Result<(), QueryError> {
        expect_ok(self.control(&ControlRequest::Resume)?)
    }

    pub fn recent(&self, now: OffsetDateTime, minutes: i64) -> Result<ContextBrief, QueryError> {
        let range = recent_range(now, Duration::minutes(minutes))?;
        self.context(range, now)
    }

    pub fn around(
        &self,
        anchor: OffsetDateTime,
        total_minutes: i64,
    ) -> Result<ContextDetail, QueryError> {
        let radius = Duration::seconds(total_minutes * 30);
        let range = around_range(anchor, radius)?;
        Ok(ContextDetail {
            start: range.start,
            end: range.end,
            segments: self.segments(range, now_local())?,
        })
    }

    pub fn today(&self, date: Date, offset: UtcOffset) -> Result<ContextBrief, QueryError> {
        let start = date.with_time(Time::MIDNIGHT).assume_offset(offset);
        let next = date.next_day().ok_or(QueryError::DateOverflow)?;
        let end = next.with_time(Time::MIDNIGHT).assume_offset(offset);
        self.context(ContextRange::new(start, end)?, now_local())
    }

    pub fn delete_date(&self, date: Date, offset: UtcOffset) -> Result<u64, QueryError> {
        let start = date.with_time(Time::MIDNIGHT).assume_offset(offset);
        let next = date.next_day().ok_or(QueryError::DateOverflow)?;
        let end = next.with_time(Time::MIDNIGHT).assume_offset(offset);
        if self.paths.control_socket.exists() {
            return match self.control(&ControlRequest::Delete { start, end })? {
                ControlResponse::Deleted { segments } => Ok(segments),
                response => Err(QueryError::UnexpectedControlResponse(Box::new(response))),
            };
        }

        let mut store = Store::open(&self.paths.database_file)?;
        Ok(u64::try_from(store.delete_range(start, end)?).unwrap_or(u64::MAX))
    }

    fn context(
        &self,
        range: ContextRange,
        observed_until: OffsetDateTime,
    ) -> Result<ContextBrief, QueryError> {
        let segments = self.segments(range, observed_until)?;
        Ok(ContextBrief {
            start: range.start,
            end: range.end,
            slices: build_slices(&segments, range)?,
        })
    }

    fn segments(
        &self,
        range: ContextRange,
        observed_until: OffsetDateTime,
    ) -> Result<Vec<FocusSegment>, QueryError> {
        let Some(effective_end) = effective_query_end(range, observed_until) else {
            return Ok(Vec::new());
        };
        let store = Store::open(&self.paths.database_file)?;
        Ok(store.segments_between(range.start, effective_end)?)
    }

    fn control(&self, request: &ControlRequest) -> Result<ControlResponse, QueryError> {
        Ok(LocalControlClient::new(&self.paths.control_socket).request(request)?)
    }
}

fn now_local() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

fn effective_query_end(
    range: ContextRange,
    observed_until: OffsetDateTime,
) -> Option<OffsetDateTime> {
    let end = range.end.min(observed_until);
    (end > range.start).then_some(end)
}

fn expect_ok(response: ControlResponse) -> Result<(), QueryError> {
    match response {
        ControlResponse::Ok => Ok(()),
        response => Err(QueryError::UnexpectedControlResponse(Box::new(response))),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error(transparent)]
    Paths(#[from] PathsError),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Service(#[from] ServiceError),
    #[error(transparent)]
    Control(#[from] LocalControlError),
    #[error(transparent)]
    Store(#[from] openbrief_store::StoreError),
    #[error(transparent)]
    Core(#[from] openbrief_core::CoreError),
    #[error("date has no following day")]
    DateOverflow,
    #[error("collector returned an unexpected response: {0:?}")]
    UnexpectedControlResponse(Box<ControlResponse>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn an_open_segment_is_never_projected_beyond_observation_time() {
        let range = ContextRange::new(
            datetime!(2026-07-30 00:00 +09:00),
            datetime!(2026-07-31 00:00 +09:00),
        )
        .unwrap();
        let now = datetime!(2026-07-30 10:46 +09:00);

        assert_eq!(effective_query_end(range, now), Some(now));
    }

    #[test]
    fn a_future_range_has_no_observed_end() {
        let range = ContextRange::new(
            datetime!(2026-07-31 00:00 +09:00),
            datetime!(2026-08-01 00:00 +09:00),
        )
        .unwrap();

        assert_eq!(
            effective_query_end(range, datetime!(2026-07-30 10:46 +09:00)),
            None
        );
    }
}
