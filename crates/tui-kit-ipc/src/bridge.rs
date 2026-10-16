//! The host's end of one renderer connection, with no I/O: it decides what to
//! send and turns what arrives into what the host must handle. The supervisor
//! moves the lines.

use crate::{PROTOCOL, Size, ToHost, ToRenderer, encode};
use serde::Serialize;
use std::collections::VecDeque;

/// What a message from the renderer means for the host.
#[derive(Debug, PartialEq)]
pub enum Received<I> {
    /// Handshake done: the terminal's size.
    Ready(Size),
    /// The versions differ; the renderer has been told and is not restarted.
    Refused(String),
    Intent(I),
    Resize(Size),
    Log {
        level: String,
        text: String,
    },
    Bye,
    Nothing,
}

pub struct Bridge<V> {
    view: u32,
    cursor: u64,
    ready: bool,
    in_flight: Option<u64>,
    last_sent: Option<V>,
    outbox: VecDeque<String>,
}

impl<V: Serialize + PartialEq + Clone> Bridge<V> {
    /// `view` is the app's view version.
    pub fn new(view: u32) -> Self {
        Bridge {
            view,
            cursor: 0,
            ready: false,
            in_flight: None,
            last_sent: None,
            outbox: VecDeque::new(),
        }
    }

    pub fn receive<I>(&mut self, message: ToHost<I>) -> Received<I> {
        match message {
            ToHost::Hello {
                protocol,
                view,
                size,
            } => {
                if protocol != PROTOCOL || view != self.view {
                    let (wanted, got) = if protocol != PROTOCOL {
                        (PROTOCOL, protocol)
                    } else {
                        (self.view, view)
                    };
                    let message = format!(
                        "the renderer speaks protocol {protocol}, view {view}; this host speaks protocol {PROTOCOL}, view {}",
                        self.view
                    );
                    self.push(&ToRenderer::<V>::Refuse {
                        wanted,
                        got,
                        message: message.clone(),
                    });
                    return Received::Refused(message);
                }
                self.ready = true;
                self.push(&ToRenderer::<V>::Welcome {
                    protocol: PROTOCOL,
                    view: self.view,
                    cursor: self.cursor,
                });
                Received::Ready(size)
            }
            ToHost::Ack { cursor } => {
                if self.in_flight.is_some_and(|c| cursor >= c) {
                    self.in_flight = None;
                }
                Received::Nothing
            }
            // A stale cursor is handled against the current state: targets are stable.
            ToHost::Intent { intent, .. } => Received::Intent(intent),
            ToHost::Resize { size } => Received::Resize(size),
            ToHost::Log { level, text } => Received::Log { level, text },
            ToHost::Bye => Received::Bye,
        }
    }

    /// The host's state now. Sent when the renderer is ready, nothing is in
    /// flight and it differs from the last one sent; otherwise the next offer
    /// after the ack sends whatever is newest then.
    pub fn offer(&mut self, view: &V) {
        if !self.ready || self.in_flight.is_some() || self.last_sent.as_ref() == Some(view) {
            return;
        }
        self.cursor += 1;
        self.in_flight = Some(self.cursor);
        self.last_sent = Some(view.clone());
        self.push(&ToRenderer::Snapshot {
            cursor: self.cursor,
            view: view.clone(),
        });
    }

    pub fn notice(&mut self, text: impl Into<String>) {
        self.push(&ToRenderer::<V>::Notice { text: text.into() });
    }

    pub fn shutdown(&mut self) {
        self.push(&ToRenderer::<V>::Shutdown);
    }

    /// The lines to write, in order.
    pub fn take(&mut self) -> Vec<String> {
        self.outbox.drain(..).collect()
    }

    pub fn in_flight(&self) -> Option<u64> {
        self.in_flight
    }

    /// A new renderer starts: it is told everything again. The cursor keeps rising.
    pub fn reconnect(&mut self) {
        self.ready = false;
        self.in_flight = None;
        self.last_sent = None;
        self.outbox.clear();
    }

    fn push(&mut self, message: &ToRenderer<V>) {
        self.outbox.push_back(encode(message));
    }
}
