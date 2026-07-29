use openbrief_core::{ActivitySlice, ContextRange, FocusState, FocusTransition, build_slices};
use openbrief_store::Store;
use serde::Deserialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const FIXTURE: &str =
    include_str!("../../../fixtures/golden-cases/gc-02-activity-recall-timeline.json");

#[derive(Deserialize)]
struct Fixture {
    given: Given,
    steps: Vec<Step>,
}

#[derive(Deserialize)]
struct Given {
    #[serde(rename = "focusEvents")]
    focus_events: Vec<FocusEvent>,
}

#[derive(Deserialize)]
struct FocusEvent {
    at: String,
    state: FocusState,
    #[serde(rename = "appId")]
    app_id: Option<String>,
}

#[derive(Deserialize)]
struct Step {
    command: String,
    expect: StepExpectation,
}

#[derive(Deserialize)]
struct StepExpectation {
    #[serde(rename = "activitySlices", default)]
    activity_slices: Vec<ExpectedSlice>,
}

#[derive(Deserialize)]
struct ExpectedSlice {
    start: String,
    end: String,
    durations: Vec<ExpectedDuration>,
}

#[derive(Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
struct ExpectedDuration {
    #[serde(rename = "appId")]
    app_id: Option<String>,
    state: Option<FocusState>,
    seconds: u64,
}

#[test]
fn gc_02_focus_metadata_round_trips_through_sqlite_into_activity_slices() {
    let fixture: Fixture = serde_json::from_str(FIXTURE).expect("GC-02 must remain valid JSON");
    let expected = &fixture
        .steps
        .iter()
        .find(|step| step.command == "timeline.today")
        .expect("GC-02 must define timeline.today")
        .expect
        .activity_slices;
    let range = ContextRange::new(
        parse_time(&expected.first().expect("expected slices").start),
        parse_time(&expected.last().expect("expected slices").end),
    )
    .unwrap();

    let mut store = Store::open_in_memory().unwrap();
    for event in fixture.given.focus_events {
        let at = parse_time(&event.at);
        let transition = if event.state == FocusState::Observed {
            FocusTransition::observed(at, event.app_id.expect("observed event needs appId"))
                .unwrap()
        } else {
            FocusTransition::gap(at, event.state).unwrap()
        };
        store.append_transition(&transition).unwrap();
    }

    let segments = store.segments_between(range.start, range.end).unwrap();
    let actual = build_slices(&segments, range).unwrap();

    assert_slices_match(&actual, expected);
    assert!(
        segments
            .iter()
            .filter(|segment| segment.state != FocusState::Observed)
            .all(|segment| segment.app_id().is_none()),
        "excluded and idle segments must not retain source app IDs"
    );
}

fn assert_slices_match(actual: &[ActivitySlice], expected: &[ExpectedSlice]) {
    assert_eq!(actual.len(), expected.len());

    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual.start, parse_time(&expected.start));
        assert_eq!(actual.end, parse_time(&expected.end));

        let mut actual_durations = actual
            .durations
            .iter()
            .map(|duration| ExpectedDuration {
                app_id: duration.app_id().map(ToOwned::to_owned),
                state: (duration.state != FocusState::Observed).then(|| duration.state.clone()),
                seconds: duration.seconds,
            })
            .collect::<Vec<_>>();
        actual_durations.sort();

        let mut expected_durations = expected
            .durations
            .iter()
            .map(|duration| ExpectedDuration {
                app_id: duration.app_id.clone(),
                state: duration.state.clone(),
                seconds: duration.seconds,
            })
            .collect::<Vec<_>>();
        expected_durations.sort();

        assert_eq!(actual_durations, expected_durations);
    }
}

fn parse_time(value: &str) -> OffsetDateTime {
    OffsetDateTime::parse(value, &Rfc3339).expect("fixture timestamp must be RFC 3339")
}
