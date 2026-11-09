//! Interactive event system and cross-thread event streaming for Nereus TUI.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use crate::cockpit::FocusedPane;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Domain events consumed by the Cockpit event loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    /// Terminal keyboard event
    Key(KeyEvent),
    /// Periodic animation and refresh tick
    Tick,
    /// Terminal window resize event with new columns and rows
    Resize(u16, u16),
}

/// Actions emitted by the event handler in response to keyboard inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    /// No state modification required
    None,
    /// Signal the application event loop to terminate gracefully
    Quit,
    /// Submit student code for verification and sandbox execution
    Submit(String),
    /// Focus transferred to another pane
    FocusChanged(FocusedPane),
    /// Scroll offset adjusted within the specified pane
    Scrolled { pane: FocusedPane, delta: isize },
    /// Editor text buffer was modified
    BufferModified,
}

/// Asynchronous event handler dispatching crossterm events and periodic ticks.
pub struct EventHandler {
    receiver: mpsc::Receiver<AppEvent>,
    sender: mpsc::Sender<AppEvent>,
    worker: Option<JoinHandle<()>>,
}

impl EventHandler {
    /// Spawns an event listener polling crossterm events with the specified tick rate.
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::channel(256);
        let event_sender = sender.clone();

        let worker = tokio::spawn(async move {
            loop {
                // Poll for terminal events using synchronous crossterm poll within non-blocking window
                let has_event = event::poll(tick_rate).unwrap_or_default();

                if has_event {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key))
                            if key.kind == KeyEventKind::Press
                                && event_sender.send(AppEvent::Key(key)).await.is_err() =>
                        {
                            break;
                        }
                        Ok(CrosstermEvent::Resize(cols, rows))
                            if event_sender
                                .send(AppEvent::Resize(cols, rows))
                                .await
                                .is_err() =>
                        {
                            break;
                        }
                        _ => {}
                    }
                } else if event_sender.send(AppEvent::Tick).await.is_err() {
                    break;
                }
            }
        });

        Self {
            receiver,
            sender,
            worker: Some(worker),
        }
    }

    /// Creates an offline, manual event handler without background OS threads, ideal for unit testing.
    pub fn new_manual() -> (Self, mpsc::Sender<AppEvent>) {
        let (sender, receiver) = mpsc::channel(256);
        let handler = Self {
            receiver,
            sender: sender.clone(),
            worker: None,
        };
        (handler, sender)
    }

    /// Access the internal event channel sender.
    pub fn sender(&self) -> mpsc::Sender<AppEvent> {
        self.sender.clone()
    }

    /// Asynchronously receive the next event in the queue.
    pub async fn next(&mut self) -> Option<AppEvent> {
        self.receiver.recv().await
    }

    /// Attempt to retrieve the next event synchronously without blocking.
    pub fn try_next(&mut self) -> Option<AppEvent> {
        self.receiver.try_recv().ok()
    }
}

impl Drop for EventHandler {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            worker.abort();
        }
    }
}
