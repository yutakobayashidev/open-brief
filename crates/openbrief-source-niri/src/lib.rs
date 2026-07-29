//! Blocking foreground-window events from niri.
//!
//! This crate intentionally keeps niri's window title and process ID out of its
//! state and public DTO. A window key is opaque to callers and is valid only for
//! the lifetime of the corresponding niri window.

use std::collections::HashMap;
use std::io;

use niri_ipc::socket::Socket;
use niri_ipc::{Event, Request, Response, Window};
use thiserror::Error;

/// A change to niri's currently focused toplevel window.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NiriFocusChange {
    /// Application ID reported by niri, when one is available.
    pub app_id: Option<String>,
    /// Runtime-only opaque key for the focused window.
    pub window_key: Option<String>,
}

/// Errors from connecting to or reading niri's event stream.
#[derive(Debug, Error)]
pub enum NiriSourceError {
    /// The niri socket could not be accessed or decoded.
    #[error("niri IPC failed: {0}")]
    Io(#[from] io::Error),

    /// Niri rejected the event-stream request.
    #[error("niri rejected the event stream: {0}")]
    Rejected(String),

    /// Niri returned a successful response other than `Handled`.
    #[error("niri returned an unexpected event-stream response: {0}")]
    UnexpectedResponse(String),
}

/// Result type used by the niri source.
pub type Result<T> = std::result::Result<T, NiriSourceError>;

type EventReader = Box<dyn FnMut() -> io::Result<Event>>;

/// A blocking source backed directly by niri's socket event stream.
pub struct NiriEventSource {
    read_event: EventReader,
    reducer: NiriEventReducer,
}

impl NiriEventSource {
    /// Connects to `$NIRI_SOCKET` and requests niri's direct event stream.
    ///
    /// # Errors
    ///
    /// Returns an error when the socket cannot be opened, the request cannot be
    /// exchanged, or niri does not acknowledge the event-stream request.
    pub fn connect() -> Result<Self> {
        let mut socket = Socket::connect()?;
        match socket.send(Request::EventStream)? {
            Ok(Response::Handled) => {}
            Ok(response) => {
                return Err(NiriSourceError::UnexpectedResponse(format!("{response:?}")));
            }
            Err(message) => return Err(NiriSourceError::Rejected(message)),
        }

        Ok(Self {
            read_event: Box::new(socket.read_events()),
            reducer: NiriEventReducer::default(),
        })
    }

    /// Blocks until niri sends one event and reduces it to a focus change.
    ///
    /// `Ok(None)` means that the received event did not change the public
    /// focused-window state. Call again to wait for the next niri event.
    ///
    /// # Errors
    ///
    /// Returns an error when the next event cannot be read or decoded.
    pub fn next_focus_change(&mut self) -> Result<Option<NiriFocusChange>> {
        let event = (self.read_event)()?;
        Ok(self.reducer.apply(event))
    }
}

#[derive(Default)]
struct NiriEventReducer {
    apps: HashMap<u64, Option<String>>,
    focused_id: Option<u64>,
    last_emitted: Option<NiriFocusChange>,
}

impl NiriEventReducer {
    fn apply(&mut self, event: Event) -> Option<NiriFocusChange> {
        match event {
            Event::WindowsChanged { windows } => self.replace_windows(windows),
            Event::WindowOpenedOrChanged { window } => self.upsert_window(window),
            Event::WindowClosed { id } => self.close_window(id),
            Event::WindowFocusChanged { id } => {
                self.focused_id = id;
                Some(self.emit_current())
            }
            _ => None,
        }
    }

    fn replace_windows(&mut self, windows: Vec<Window>) -> Option<NiriFocusChange> {
        self.apps.clear();
        self.focused_id = None;

        for window in windows {
            let id = window.id;
            if window.is_focused {
                self.focused_id = Some(id);
            }
            self.apps.insert(id, window.app_id);
        }

        self.emit_if_changed()
    }

    fn upsert_window(&mut self, window: Window) -> Option<NiriFocusChange> {
        let id = window.id;
        let is_focused = window.is_focused;
        self.apps.insert(id, window.app_id);

        if is_focused {
            self.focused_id = Some(id);
        }

        if self.focused_id == Some(id) {
            self.emit_if_changed()
        } else {
            None
        }
    }

    fn close_window(&mut self, id: u64) -> Option<NiriFocusChange> {
        self.apps.remove(&id);
        if self.focused_id == Some(id) {
            self.focused_id = None;
            Some(self.emit_current())
        } else {
            None
        }
    }

    fn emit_if_changed(&mut self) -> Option<NiriFocusChange> {
        let change = self.current();
        if self.last_emitted.as_ref() == Some(&change) {
            None
        } else {
            self.last_emitted = Some(change.clone());
            Some(change)
        }
    }

    fn emit_current(&mut self) -> NiriFocusChange {
        let change = self.current();
        self.last_emitted = Some(change.clone());
        change
    }

    fn current(&self) -> NiriFocusChange {
        let app_id = self
            .focused_id
            .and_then(|id| self.apps.get(&id).cloned().flatten());
        let window_key = self.focused_id.map(opaque_window_key);

        NiriFocusChange { app_id, window_key }
    }
}

fn opaque_window_key(id: u64) -> String {
    format!("niri-window:{id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOCUSED_WINDOW: &str = r#"{
        "WindowOpenedOrChanged": {
            "window": {
                "id": 41,
                "title": "secret title",
                "app_id": "com.example.Editor",
                "pid": 9876,
                "workspace_id": 2,
                "is_focused": true,
                "is_floating": false,
                "is_urgent": false,
                "layout": {
                    "pos_in_scrolling_layout": [1, 1],
                    "tile_size": [1200.0, 800.0],
                    "window_size": [1198, 798],
                    "tile_pos_in_workspace_view": [0.0, 0.0],
                    "window_offset_in_tile": [1.0, 1.0]
                },
                "focus_timestamp": null
            }
        }
    }"#;

    const UNFOCUSED_WINDOW: &str = r#"{
        "WindowOpenedOrChanged": {
            "window": {
                "id": 42,
                "title": "another private title",
                "app_id": "org.example.Browser",
                "pid": 1234,
                "workspace_id": 2,
                "is_focused": false,
                "is_floating": false,
                "is_urgent": false,
                "layout": {
                    "pos_in_scrolling_layout": [2, 1],
                    "tile_size": [1200.0, 800.0],
                    "window_size": [1198, 798],
                    "tile_pos_in_workspace_view": [1200.0, 0.0],
                    "window_offset_in_tile": [1.0, 1.0]
                },
                "focus_timestamp": null
            }
        }
    }"#;

    fn parse(json: &str) -> Event {
        serde_json::from_str(json).expect("valid native niri event JSON")
    }

    #[test]
    fn native_window_json_keeps_only_app_and_opaque_key() {
        let mut reducer = NiriEventReducer::default();

        let change = reducer
            .apply(parse(FOCUSED_WINDOW))
            .expect("focused window produces a change");

        assert_eq!(
            change,
            NiriFocusChange {
                app_id: Some("com.example.Editor".to_owned()),
                window_key: Some("niri-window:41".to_owned()),
            }
        );
        assert!(!format!("{change:?}").contains("secret title"));
        assert!(!format!("{change:?}").contains("9876"));
    }

    #[test]
    fn focus_change_uses_cached_app_and_focus_none_is_explicit() {
        let mut reducer = NiriEventReducer::default();
        assert!(reducer.apply(parse(UNFOCUSED_WINDOW)).is_none());

        let focused = reducer
            .apply(parse(r#"{"WindowFocusChanged":{"id":42}}"#))
            .expect("focus event is public");
        assert_eq!(
            focused,
            NiriFocusChange {
                app_id: Some("org.example.Browser".to_owned()),
                window_key: Some("niri-window:42".to_owned()),
            }
        );

        let unfocused = reducer
            .apply(parse(r#"{"WindowFocusChanged":{"id":null}}"#))
            .expect("focus-none event is public");
        assert_eq!(
            unfocused,
            NiriFocusChange {
                app_id: None,
                window_key: None,
            }
        );
    }

    #[test]
    fn windows_changed_replaces_state_and_closed_focus_clears_it() {
        let mut reducer = NiriEventReducer::default();
        let snapshot = format!(
            r#"{{"WindowsChanged":{{"windows":[{},{}]}}}}"#,
            window_value(41, "com.example.Editor", false),
            window_value(42, "org.example.Browser", true)
        );

        assert_eq!(
            reducer.apply(parse(&snapshot)),
            Some(NiriFocusChange {
                app_id: Some("org.example.Browser".to_owned()),
                window_key: Some("niri-window:42".to_owned()),
            })
        );
        assert_eq!(
            reducer.apply(parse(r#"{"WindowClosed":{"id":42}}"#)),
            Some(NiriFocusChange {
                app_id: None,
                window_key: None,
            })
        );
    }

    #[test]
    fn focused_window_app_change_is_emitted_and_unknown_event_is_ignored() {
        let mut reducer = NiriEventReducer::default();
        reducer.apply(parse(FOCUSED_WINDOW));

        let changed = window_event(41, "com.example.Editor.Renamed", true);
        assert_eq!(
            reducer.apply(parse(&changed)),
            Some(NiriFocusChange {
                app_id: Some("com.example.Editor.Renamed".to_owned()),
                window_key: Some("niri-window:41".to_owned()),
            })
        );

        assert!(
            reducer
                .apply(parse(r#"{"OverviewOpenedOrClosed":{"is_open":true}}"#))
                .is_none()
        );
    }

    fn window_event(id: u64, app_id: &str, focused: bool) -> String {
        format!(
            r#"{{"WindowOpenedOrChanged":{{"window":{}}}}}"#,
            window_value(id, app_id, focused)
        )
    }

    fn window_value(id: u64, app_id: &str, focused: bool) -> String {
        format!(
            r#"{{
                "id":{id},
                "title":"not retained",
                "app_id":"{app_id}",
                "pid":5678,
                "workspace_id":1,
                "is_focused":{focused},
                "is_floating":false,
                "is_urgent":false,
                "layout":{{
                    "pos_in_scrolling_layout":[1,1],
                    "tile_size":[1000.0,700.0],
                    "window_size":[998,698],
                    "tile_pos_in_workspace_view":[0.0,0.0],
                    "window_offset_in_tile":[1.0,1.0]
                }},
                "focus_timestamp":null
            }}"#
        )
    }
}
