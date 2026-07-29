use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

const ACTIVITY_BUCKET_SECONDS: i64 = 15 * 60;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FocusState {
    Observed,
    Excluded,
    Idle,
    Locked,
    Paused,
    Disabled,
    SourceUnavailable,
}

impl FocusState {
    pub fn gap_reason(&self) -> Option<GapReason> {
        match self {
            Self::Observed => None,
            Self::Excluded => Some(GapReason::Excluded),
            Self::Idle => Some(GapReason::Idle),
            Self::Locked => Some(GapReason::Locked),
            Self::Paused => Some(GapReason::Paused),
            Self::Disabled => Some(GapReason::Disabled),
            Self::SourceUnavailable => Some(GapReason::SourceUnavailable),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapReason {
    Excluded,
    Idle,
    Locked,
    Paused,
    Disabled,
    SourceUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusTransition {
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    pub state: FocusState,
    #[serde(skip_serializing_if = "Option::is_none")]
    app_id: Option<String>,
}

impl FocusTransition {
    pub fn new(
        at: OffsetDateTime,
        state: FocusState,
        app_id: Option<String>,
    ) -> Result<Self, CoreError> {
        Ok(Self {
            at,
            app_id: sanitize_app_id(&state, app_id)?,
            state,
        })
    }

    pub fn observed(at: OffsetDateTime, app_id: impl Into<String>) -> Result<Self, CoreError> {
        Self::new(at, FocusState::Observed, Some(app_id.into()))
    }

    pub fn gap(at: OffsetDateTime, state: FocusState) -> Result<Self, CoreError> {
        Self::new(at, state, None)
    }

    pub fn app_id(&self) -> Option<&str> {
        self.app_id.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusSegment {
    pub id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(
        with = "time::serde::rfc3339::option",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ended_at: Option<OffsetDateTime>,
    pub state: FocusState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
}

impl FocusSegment {
    pub fn new(
        started_at: OffsetDateTime,
        ended_at: Option<OffsetDateTime>,
        state: FocusState,
        app_id: Option<String>,
    ) -> Result<Self, CoreError> {
        if let Some(ended_at) = ended_at
            && ended_at <= started_at
        {
            return Err(CoreError::InvalidRange {
                start: started_at,
                end: ended_at,
            });
        }

        Ok(Self {
            id: segment_id(started_at),
            started_at,
            ended_at,
            app_id: sanitize_app_id(&state, app_id)?,
            state,
        })
    }

    pub fn app_id(&self) -> Option<&str> {
        self.app_id.as_deref()
    }

    pub fn duration(&self) -> Option<Duration> {
        self.ended_at.map(|ended_at| ended_at - self.started_at)
    }
}

#[derive(Debug, Clone)]
pub struct FocusReducer {
    initial_at: OffsetDateTime,
    current: Option<FocusTransition>,
}

impl FocusReducer {
    pub fn new(initial_at: OffsetDateTime) -> Self {
        Self {
            initial_at,
            current: None,
        }
    }

    pub fn transition(
        &mut self,
        at: OffsetDateTime,
        state: FocusState,
        app_id: Option<String>,
    ) -> Result<Option<FocusSegment>, CoreError> {
        if at < self.initial_at {
            return Err(CoreError::NonChronologicalTransition {
                previous: self.initial_at,
                current: at,
            });
        }

        if let Some(current) = &self.current
            && at <= current.at
        {
            return Err(CoreError::NonChronologicalTransition {
                previous: current.at,
                current: at,
            });
        }

        let next = FocusTransition::new(at, state, app_id)?;
        let closed = self
            .current
            .take()
            .map(|current| FocusSegment::new(current.at, Some(at), current.state, current.app_id));
        self.current = Some(next);
        closed.transpose()
    }

    pub fn finish(&mut self, at: OffsetDateTime) -> Result<Option<FocusSegment>, CoreError> {
        let Some(current) = self.current.take() else {
            return Ok(None);
        };

        if at <= current.at {
            self.current = Some(current);
            return Err(CoreError::InvalidRange {
                start: self.current.as_ref().expect("restored current").at,
                end: at,
            });
        }

        FocusSegment::new(current.at, Some(at), current.state, current.app_id).map(Some)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurationEntry {
    pub state: FocusState,
    #[serde(skip_serializing_if = "Option::is_none")]
    app_id: Option<String>,
    pub seconds: u64,
}

impl DurationEntry {
    pub fn new(state: FocusState, app_id: Option<String>, seconds: u64) -> Result<Self, CoreError> {
        Ok(Self {
            app_id: sanitize_app_id(&state, app_id)?,
            state,
            seconds,
        })
    }

    pub fn app_id(&self) -> Option<&str> {
        self.app_id.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivitySlice {
    #[serde(with = "time::serde::rfc3339")]
    pub start: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub end: OffsetDateTime,
    pub durations: Vec<DurationEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextBrief {
    #[serde(with = "time::serde::rfc3339")]
    pub start: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub end: OffsetDateTime,
    pub slices: Vec<ActivitySlice>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextRange {
    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
}

impl ContextRange {
    pub fn new(start: OffsetDateTime, end: OffsetDateTime) -> Result<Self, CoreError> {
        if end <= start {
            return Err(CoreError::InvalidRange { start, end });
        }
        Ok(Self { start, end })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoreError {
    #[error("observed focus state requires a non-empty app id")]
    MissingObservedAppId,
    #[error("focus transitions must be strictly chronological: {previous} then {current}")]
    NonChronologicalTransition {
        previous: OffsetDateTime,
        current: OffsetDateTime,
    },
    #[error("invalid time range: {start} to {end}")]
    InvalidRange {
        start: OffsetDateTime,
        end: OffsetDateTime,
    },
    #[error("duration cannot be negative or exceed u64 seconds")]
    InvalidDuration,
}

pub fn reduce_focus_transitions(
    transitions: &[FocusTransition],
    end_at: OffsetDateTime,
) -> Result<Vec<FocusSegment>, CoreError> {
    let mut segments = Vec::with_capacity(transitions.len());

    for pair in transitions.windows(2) {
        let current = &pair[0];
        let next = &pair[1];
        if next.at <= current.at {
            return Err(CoreError::NonChronologicalTransition {
                previous: current.at,
                current: next.at,
            });
        }
        segments.push(FocusSegment::new(
            current.at,
            Some(next.at),
            current.state.clone(),
            current.app_id.clone(),
        )?);
    }

    if let Some(last) = transitions.last() {
        if end_at <= last.at {
            return Err(CoreError::InvalidRange {
                start: last.at,
                end: end_at,
            });
        }
        segments.push(FocusSegment::new(
            last.at,
            Some(end_at),
            last.state.clone(),
            last.app_id.clone(),
        )?);
    }

    Ok(segments)
}

pub fn project_activity_slices(segments: &[FocusSegment]) -> Result<Vec<ActivitySlice>, CoreError> {
    let Some(start) = segments.first().map(|segment| segment.started_at) else {
        return Ok(Vec::new());
    };
    let Some(end) = segments.iter().rev().find_map(|segment| segment.ended_at) else {
        return Ok(Vec::new());
    };
    build_slices(segments, ContextRange::new(start, end)?)
}

pub fn build_slices(
    segments: &[FocusSegment],
    range: ContextRange,
) -> Result<Vec<ActivitySlice>, CoreError> {
    validate_segments(segments)?;

    let mut buckets: BTreeMap<OffsetDateTime, BTreeMap<(FocusState, Option<String>), u64>> =
        BTreeMap::new();

    for segment in segments {
        let Some(segment_end) = segment.ended_at else {
            continue;
        };
        let mut cursor = segment.started_at.max(range.start);
        let effective_end = segment_end.min(range.end);
        while cursor < effective_end {
            let bucket_start = floor_to_activity_bucket(cursor);
            let bucket_end = bucket_start + Duration::seconds(ACTIVITY_BUCKET_SECONDS);
            let piece_end = effective_end.min(bucket_end);
            let seconds: u64 = (piece_end - cursor)
                .whole_seconds()
                .try_into()
                .map_err(|_| CoreError::InvalidDuration)?;

            if seconds > 0 {
                *buckets
                    .entry(bucket_start)
                    .or_default()
                    .entry((segment.state.clone(), segment.app_id.clone()))
                    .or_default() += seconds;
            }
            cursor = piece_end;
        }
    }

    buckets
        .into_iter()
        .map(|(start, entries)| {
            let durations = entries
                .into_iter()
                .map(|((state, app_id), seconds)| DurationEntry::new(state, app_id, seconds))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ActivitySlice {
                start,
                end: start + Duration::seconds(ACTIVITY_BUCKET_SECONDS),
                durations,
            })
        })
        .collect()
}

pub fn recent_context(
    slices: &[ActivitySlice],
    anchor: OffsetDateTime,
    lookback: Duration,
) -> Result<ContextBrief, CoreError> {
    if lookback.is_negative() {
        return Err(CoreError::InvalidRange {
            start: anchor,
            end: anchor - lookback,
        });
    }
    context_in_range(slices, anchor - lookback, anchor)
}

pub fn around_context(
    slices: &[ActivitySlice],
    anchor: OffsetDateTime,
    radius: Duration,
) -> Result<ContextBrief, CoreError> {
    if radius.is_negative() {
        return Err(CoreError::InvalidRange {
            start: anchor + radius,
            end: anchor - radius,
        });
    }
    context_in_range(slices, anchor - radius, anchor + radius)
}

pub fn context_in_range(
    slices: &[ActivitySlice],
    start: OffsetDateTime,
    end: OffsetDateTime,
) -> Result<ContextBrief, CoreError> {
    if end <= start {
        return Err(CoreError::InvalidRange { start, end });
    }

    Ok(ContextBrief {
        start,
        end,
        slices: slices
            .iter()
            .filter(|slice| slice.start < end && slice.end > start)
            .cloned()
            .collect(),
    })
}

pub fn recent_range(anchor: OffsetDateTime, lookback: Duration) -> Result<ContextRange, CoreError> {
    if lookback.is_negative() || lookback.is_zero() {
        return Err(CoreError::InvalidRange {
            start: anchor,
            end: anchor - lookback,
        });
    }
    ContextRange::new(anchor - lookback, anchor)
}

pub fn around_range(anchor: OffsetDateTime, radius: Duration) -> Result<ContextRange, CoreError> {
    if radius.is_negative() || radius.is_zero() {
        return Err(CoreError::InvalidRange {
            start: anchor + radius,
            end: anchor - radius,
        });
    }
    ContextRange::new(anchor - radius, anchor + radius)
}

fn sanitize_app_id(
    state: &FocusState,
    app_id: Option<String>,
) -> Result<Option<String>, CoreError> {
    if *state != FocusState::Observed {
        return Ok(None);
    }

    let app_id = app_id
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or(CoreError::MissingObservedAppId)?;
    Ok(Some(app_id))
}

fn validate_segments(segments: &[FocusSegment]) -> Result<(), CoreError> {
    for pair in segments.windows(2) {
        if pair[0]
            .ended_at
            .is_some_and(|ended_at| pair[1].started_at < ended_at)
        {
            return Err(CoreError::NonChronologicalTransition {
                previous: pair[0].ended_at.expect("checked some"),
                current: pair[1].started_at,
            });
        }
    }
    Ok(())
}

fn segment_id(started_at: OffsetDateTime) -> String {
    format!("focus-{}", started_at.unix_timestamp_nanos())
}

fn floor_to_activity_bucket(value: OffsetDateTime) -> OffsetDateTime {
    let offset = value.offset();
    let local_seconds = value.unix_timestamp() + i64::from(offset.whole_seconds());
    let bucket_local_seconds =
        local_seconds.div_euclid(ACTIVITY_BUCKET_SECONDS) * ACTIVITY_BUCKET_SECONDS;
    let bucket_unix_seconds = bucket_local_seconds - i64::from(offset.whole_seconds());

    OffsetDateTime::from_unix_timestamp(bucket_unix_seconds)
        .expect("bucket timestamp remains in OffsetDateTime range")
        .to_offset(offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::{UtcOffset, macros::datetime};

    #[test]
    fn reducer_uses_transition_boundaries_without_overlap() {
        let transitions = vec![
            FocusTransition::observed(datetime!(2026-07-29 10:00:00 +09:00), "ghostty").unwrap(),
            FocusTransition::observed(datetime!(2026-07-29 10:06:00 +09:00), "firefox").unwrap(),
            FocusTransition::gap(datetime!(2026-07-29 10:14:00 +09:00), FocusState::Idle).unwrap(),
        ];

        let segments =
            reduce_focus_transitions(&transitions, datetime!(2026-07-29 10:20:00 +09:00)).unwrap();

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].duration(), Some(Duration::minutes(6)));
        assert_eq!(segments[1].duration(), Some(Duration::minutes(8)));
        assert_eq!(segments[2].duration(), Some(Duration::minutes(6)));
        assert_eq!(segments[1].app_id(), Some("firefox"));
    }

    #[test]
    fn incremental_reducer_closes_the_previous_segment() {
        let mut reducer = FocusReducer::new(datetime!(2026-07-29 10:00:00 +09:00));

        assert!(
            reducer
                .transition(
                    datetime!(2026-07-29 10:00:00 +09:00),
                    FocusState::Observed,
                    Some("ghostty".to_owned()),
                )
                .unwrap()
                .is_none()
        );
        let observed = reducer
            .transition(
                datetime!(2026-07-29 10:06:00 +09:00),
                FocusState::Excluded,
                Some("signal".to_owned()),
            )
            .unwrap()
            .unwrap();
        let excluded = reducer
            .finish(datetime!(2026-07-29 10:10:00 +09:00))
            .unwrap()
            .unwrap();

        assert_eq!(
            observed.ended_at,
            Some(datetime!(2026-07-29 10:06:00 +09:00))
        );
        assert_eq!(observed.app_id(), Some("ghostty"));
        assert_eq!(excluded.state, FocusState::Excluded);
        assert_eq!(excluded.app_id(), None);
    }

    #[test]
    fn non_observed_constructor_discards_content_identity() {
        let excluded = FocusTransition::new(
            datetime!(2026-07-29 10:00:00 +09:00),
            FocusState::Excluded,
            Some("signal".to_owned()),
        )
        .unwrap();

        assert_eq!(excluded.app_id(), None);
        assert_eq!(
            FocusTransition::new(
                datetime!(2026-07-29 10:00:00 +09:00),
                FocusState::Observed,
                Some("   ".to_owned()),
            ),
            Err(CoreError::MissingObservedAppId)
        );
    }

    #[test]
    fn projection_splits_segments_at_local_quarter_hour() {
        let segment = FocusSegment::new(
            datetime!(2026-07-29 10:14:00 +09:00),
            Some(datetime!(2026-07-29 10:16:00 +09:00)),
            FocusState::Observed,
            Some("ghostty".to_owned()),
        )
        .unwrap();

        let slices = project_activity_slices(&[segment]).unwrap();

        assert_eq!(slices.len(), 2);
        assert_eq!(slices[0].start, datetime!(2026-07-29 10:00:00 +09:00));
        assert_eq!(slices[0].durations[0].seconds, 60);
        assert_eq!(slices[1].start, datetime!(2026-07-29 10:15:00 +09:00));
        assert_eq!(slices[1].durations[0].seconds, 60);
    }

    #[test]
    fn projection_preserves_excluded_privacy() {
        let segment = FocusSegment::new(
            datetime!(2026-07-29 10:00:00 +09:00),
            Some(datetime!(2026-07-29 10:05:00 +09:00)),
            FocusState::Excluded,
            Some("signal".to_owned()),
        )
        .unwrap();

        let slices = project_activity_slices(&[segment]).unwrap();

        assert_eq!(slices[0].durations[0].state, FocusState::Excluded);
        assert_eq!(slices[0].durations[0].app_id(), None);
    }

    #[test]
    fn recent_context_is_anchored_instead_of_using_latest_slice() {
        let segments = vec![
            FocusSegment::new(
                datetime!(2026-07-29 09:55:00 +09:00),
                Some(datetime!(2026-07-29 10:35:00 +09:00)),
                FocusState::Observed,
                Some("ghostty".to_owned()),
            )
            .unwrap(),
        ];
        let slices = project_activity_slices(&segments).unwrap();

        let brief = recent_context(
            &slices,
            datetime!(2026-07-29 10:20:00 +09:00),
            Duration::minutes(10),
        )
        .unwrap();

        assert_eq!(brief.start, datetime!(2026-07-29 10:10:00 +09:00));
        assert_eq!(brief.end, datetime!(2026-07-29 10:20:00 +09:00));
        assert_eq!(brief.slices.len(), 2);
        assert_eq!(brief.slices[0].start, datetime!(2026-07-29 10:00:00 +09:00));
        assert_eq!(brief.slices[1].start, datetime!(2026-07-29 10:15:00 +09:00));

        let range =
            recent_range(datetime!(2026-07-29 10:20:00 +09:00), Duration::minutes(10)).unwrap();
        assert_eq!(range.start, datetime!(2026-07-29 10:10:00 +09:00));
        assert_eq!(range.end, datetime!(2026-07-29 10:20:00 +09:00));
    }

    #[test]
    fn bucketing_keeps_non_utc_offset() {
        let segment = FocusSegment::new(
            datetime!(2026-07-29 10:14:30 +09:00),
            Some(datetime!(2026-07-29 10:15:30 +09:00)),
            FocusState::Observed,
            Some("ghostty".to_owned()),
        )
        .unwrap();

        let slices = project_activity_slices(&[segment]).unwrap();

        assert_eq!(
            slices[0].start.offset(),
            UtcOffset::from_hms(9, 0, 0).unwrap()
        );
        assert_eq!(slices[0].durations[0].seconds, 30);
        assert_eq!(slices[1].durations[0].seconds, 30);
    }
}
