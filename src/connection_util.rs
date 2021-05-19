use crate::{models::*, shmem::*};
use std::{self, collections::HashMap, hash::Hash, task::Poll};

pub fn flush_connection<T>(
    outgoing_slots: &mut HashMap<T, Vec<u8>>,
    app_channel: &SharedAppChannel,
) -> Poll<()>
where
    T: Copy + Into<MsgChannel> + Eq + Hash,
{
    let mut tmp = HashMap::new();
    std::mem::swap(&mut tmp, outgoing_slots);

    for (c, m) in tmp {
        if let Some((_, m)) = unsafe { app_channel.push_unchecked((c.into(), m)) } {
            outgoing_slots.insert(c, m);
        }
    }

    if outgoing_slots.is_empty() {
        Poll::Ready(())
    } else {
        Poll::Pending
    }
}
