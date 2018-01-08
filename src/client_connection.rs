extern crate futures;
extern crate std;

use connection_util::*;
use models::*;
use shmem::*;

use self::std::collections::HashMap;
use self::std::collections::hash_map::Entry;
use self::std::convert::TryFrom;
use self::std::io;
use self::std::sync::Arc;
use self::futures::*;

// Represents a connection with the control daemon.
pub struct ControlConnection {
    app_channel: SharedAppChannel,
    send_closed: bool,
    outgoing_slots: HashMap<StatusMsgChannel, Vec<u8>>,
}

impl Stream for ControlConnection {
    type Item = ControlMessage;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.app_channel
            .pull_control()
            .and_then(|m| ControlMessage::try_from(m).ok())
        {
            Some(v) => Ok(Async::Ready(Some(v))),
            None => Ok(Async::NotReady),
        }
    }
}

impl Sink for ControlConnection {
    type SinkItem = StatusMessage;
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

impl ControlConnection {
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
