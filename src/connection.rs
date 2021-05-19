use crate::{connection_util::*, models::*, shmem::*};
use futures::*;
use std::{
    collections::HashMap,
    hash::Hash,
    io,
    pin::Pin,
    task::{Context, Poll},
};

pub trait ConnectionKind {
    type OutChannel: Copy + Into<MsgChannel> + Eq + Hash + Unpin;
    type Out: Into<(Self::OutChannel, Vec<u8>)>;
    type In;

    fn pull(app_channel: &SharedAppChannel) -> Option<Self::In>;
}

pub struct Status;
pub struct Control;

impl ConnectionKind for Status {
    type OutChannel = ControlMsgChannel;
    type Out = ControlMessage;
    type In = StatusMessage;

    fn pull(app_channel: &SharedAppChannel) -> Option<Self::In> {
        app_channel.pull_status()
    }
}

impl ConnectionKind for Control {
    type OutChannel = StatusMsgChannel;
    type Out = StatusMessage;
    type In = ControlMessage;

    fn pull(app_channel: &SharedAppChannel) -> Option<Self::In> {
        app_channel.pull_control()
    }
}

pub type ClientHandle = Client<Status>;
pub type AppHandle = Client<Control>;

// Represents a connection with the running application.
pub struct Client<K: ConnectionKind> {
    app_channel: SharedAppChannel,
    send_closed: bool,
    outgoing_slots: HashMap<K::OutChannel, Vec<u8>>,
}

impl<K: ConnectionKind> Stream for Client<K> {
    type Item = K::In;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match K::pull(&self.app_channel) {
            Some(v) => Poll::Ready(Some(v)),
            None => Poll::Pending,
        }
    }
}

impl<K: ConnectionKind> Sink<K::Out> for Client<K> {
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.outgoing_slots.is_empty() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn start_send(self: Pin<&mut Self>, item: K::Out) -> Result<(), Self::Error> {
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

impl<K: ConnectionKind> Client<K> {
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
