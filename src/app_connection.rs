use connection_util::*;
use models::*;
use shmem::*;

use futures::*;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io;
use std::sync::Arc;

// Represents a connection with the running application.
pub struct ClientHandle {
    app_channel: SharedAppChannel,
    send_closed: bool,
    outgoing_slots: HashMap<ControlMsgChannel, Vec<u8>>,
}

impl Stream for ClientHandle {
    type Item = StatusMessage;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.app_channel.pull_status() {
            Some(v) => Ok(Async::Ready(Some(v))),
            None => Ok(Async::NotReady),
        }
    }
}

impl Sink for ClientHandle {
    type SinkItem = ControlMessage;
    type SinkError = io::Error;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        let (c, data) = item.clone().into();
        match self.outgoing_slots.entry(c) {
            Entry::Occupied(_) => Ok(AsyncSink::NotReady(item)),
            Entry::Vacant(e) => {
                e.insert(data);
                Ok(AsyncSink::Ready)
            }
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        if self.send_closed {
            panic!("Sink has been closed.");
        }

        self.flush()
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.send_closed = true;
        try_ready!(self.flush());
        Ok(Async::Ready(()))
    }
}

impl ClientHandle {
    pub fn new(app_channel: SharedAppChannel) -> Self {
        Self {
            send_closed: false,
            app_channel: Arc::clone(&app_channel),
            outgoing_slots: Default::default(),
        }
    }

    fn flush(&mut self) -> Poll<(), io::Error> {
        flush_connection(&mut self.outgoing_slots, &self.app_channel)
    }
}
