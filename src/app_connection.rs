use crate::{connection_util::*, models::*, shmem::*};
use futures::*;
use std::{
    collections::HashMap,
    io,
    pin::Pin,
    task::{Context, Poll},
};

// Represents a connection with the running application.
pub struct ClientHandle {
    app_channel: SharedAppChannel,
    send_closed: bool,
    outgoing_slots: HashMap<ControlMsgChannel, Vec<u8>>,
}

impl Stream for ClientHandle {
    type Item = StatusMessage;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.app_channel.pull_status() {
            Some(v) => Poll::Ready(Some(v)),
            None => Poll::Pending,
        }
    }
}

impl Sink<ControlMessage> for ClientHandle {
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.outgoing_slots.is_empty() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn start_send(self: Pin<&mut Self>, item: ControlMessage) -> Result<(), Self::Error> {
        let (c, data) = item.into();
        assert_eq!(self.get_mut().outgoing_slots.insert(c, data), None);
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.send_closed {
            panic!("Sink has been closed.");
        }

        self.get_mut().flush_data().map(Ok)
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        this.send_closed = true;

        this.flush_data().map(Ok)
    }
}

impl ClientHandle {
    pub fn new(app_channel: SharedAppChannel) -> Self {
        Self {
            send_closed: false,
            app_channel,
            outgoing_slots: Default::default(),
        }
    }

    fn flush_data(&mut self) -> Poll<()> {
        flush_connection(&mut self.outgoing_slots, &self.app_channel)
    }
}
