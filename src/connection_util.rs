extern crate futures;
extern crate std;

use self::std::collections::HashMap;
use std::hash::Hash;
use self::std::io;
use self::futures::{Async, AsyncSink, Poll};

use shmem::*;
use models::*;

pub fn flush_connection<T>(
    outgoing_slots: &mut HashMap<T, Vec<u8>>,
    app_channel: &SharedAppChannel,
) -> Poll<(), io::Error>
where
    T: Copy + Into<MsgChannel> + Eq + Hash,
{
    let mut tmp = HashMap::new();
    std::mem::swap(&mut tmp, outgoing_slots);

    for (c, m) in tmp {
        if let AsyncSink::NotReady((_, m)) = unsafe { app_channel.push_unchecked((c.into(), m)) } {
            outgoing_slots.insert(c, m);
        }
    }

    if outgoing_slots.is_empty() {
        Ok(Async::Ready(()))
    } else {
        Ok(Async::NotReady)
    }
}
