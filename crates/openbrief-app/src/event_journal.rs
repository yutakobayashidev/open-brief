use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::{DaemonEvent, SequencedDaemonEvent};

const MAX_EVENTS: usize = 800;

#[derive(Debug, Clone, Default)]
pub(crate) struct EventJournal {
    inner: Arc<Mutex<EventState>>,
}

#[derive(Debug, Default)]
struct EventState {
    next_sequence: u64,
    events: VecDeque<SequencedDaemonEvent>,
}

impl EventJournal {
    pub(crate) fn push(&self, event: DaemonEvent) {
        let mut state = self.inner.lock().expect("daemon event journal poisoned");
        let sequence = state.next_sequence;
        state.next_sequence = state.next_sequence.saturating_add(1);
        state
            .events
            .push_back(SequencedDaemonEvent { sequence, event });
        if state.events.len() > MAX_EVENTS {
            state.events.pop_front();
        }
    }

    pub(crate) fn after(&self, sequence: u64) -> (Vec<SequencedDaemonEvent>, u64) {
        let state = self.inner.lock().expect("daemon event journal poisoned");
        (
            state
                .events
                .iter()
                .filter(|event| event.sequence >= sequence)
                .cloned()
                .collect(),
            state.next_sequence,
        )
    }

    pub(crate) fn cursor(&self) -> u64 {
        self.inner
            .lock()
            .expect("daemon event journal poisoned")
            .next_sequence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursors_are_monotonic_and_reads_are_exclusive_of_prior_cursor() {
        let journal = EventJournal::default();
        let cursor = journal.cursor();
        journal.push(DaemonEvent::TurnFinished);
        let (events, next) = journal.after(cursor);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].sequence, cursor);
        assert_eq!(next, cursor + 1);
        assert!(journal.after(next).0.is_empty());
    }
}
