#![cfg_attr(feature = "cargo-clippy", allow(mutex_atomic))]
#![feature(try_from)]

#[macro_use]
extern crate enum_iter;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate futures;
#[macro_use]
extern crate serde_derive;

pub mod errors;
pub mod models;
pub mod shmem;
pub mod app_connection;
pub mod client_connection;
pub mod connection_util;

#[cfg(test)]
mod tests {
    extern crate tokio_core;
    extern crate tokio_timer;
    use client_connection::*;
    use app_connection::*;
    use models::*;
    use shmem::*;
    use self::tokio_timer::*;
    use futures::{Future, Sink, Stream};
    use std::convert::TryFrom;
    use std::time::Duration;
    use std::sync::Arc;

    #[test]
    fn test_from_ipc() {
        let expectation = StatusMessage::AppStatus(AppStatus {
            current_cpu_time: 9999.0,
            checkpoint_cpu_time: 8888.0,
            want_network: true,
            fraction_done: 0.1,
            other_pid: Some(345),

            bytes_received: None,
            bytes_sent: None,
        });

        let result = StatusMessage::try_from((
            StatusMsgChannel::AppStatus,
            "
            <current_cpu_time>9999.0</current_cpu_time>\n
            <checkpoint_cpu_time>8888.0</checkpoint_cpu_time>\n
            <want_network />
            <fraction_done>0.1</fraction_done>
            <other_pid>345</other_pid>
        "
                .into(),
        )).unwrap();

        assert_eq!(expectation, result);
    }

    #[test]
    /// In this test we create two IPCStreams which communicate via a shared AppChannel.
    fn test_inmemory_stream() {
        let fixture = AppStatus {
            current_cpu_time: 4.0,
            checkpoint_cpu_time: 5.0,
            want_network: true,
            fraction_done: 0.15,
            other_pid: None,
            bytes_sent: Some(256.0),
            bytes_received: Some(128.0),
        };
        let expectation = StatusMessage::AppStatus(fixture.clone());

        let c = MemoryAppChannel::default();
        let app_channel: SharedAppChannel = Arc::new(c);

        let client_connection = ControlConnection::new(Arc::clone(&app_channel));
        let app_connection = AppConnection::new(Arc::clone(&app_channel));

        client_connection
            .send(StatusMessage::AppStatus(fixture.clone()))
            .wait()
            .unwrap();

        let timer = Timer::default();
        let timeout = timer.sleep(Duration::from_millis(1500)).then(|_| Err(()));
        let output = timeout
            .select(
                app_connection
                    .into_future()
                    .map(|(v, _)| v.unwrap())
                    .map_err(|(_, _)| ()),
            )
            .map(|(v, _)| v)
            .wait();
        match output {
            Ok(result) => assert_eq!(expectation, result),
            Err(_) => panic!("Failed to get result within time limit"),
        };
    }
}
