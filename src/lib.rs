//! BOINC application API. Required for communication between crunching applications and the BOINC client.
//!
//! This crate provides common utilities for building BOINC clients and applications:
//!

#[macro_use]
extern crate enum_iter;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate futures;
extern crate futures_timer;
extern crate libc;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate treexml;
extern crate treexml_util;

pub mod app_connection;
pub mod client_connection;
pub mod connection_util;
pub mod errors;
pub mod models;
pub mod shmem;

pub use errors::Error;

#[cfg(test)]
mod tests {
    use app_connection::*;
    use client_connection::*;
    use futures::prelude::*;
    use futures_timer::FutureExt;
    use models::*;
    use shmem::*;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_from_ipc() {
        let expectation = StatusMessage::AppStatus(AppStatusData {
            current_cpu_time: 9999.0,
            checkpoint_cpu_time: 8888.0,
            want_network: true,
            fraction_done: 0.1,
            other_pid: Some(345),

            bytes_received: None,
            bytes_sent: None,
        });

        let result = StatusMessage::from_raw(
            StatusMsgChannel::AppStatus,
            "
            <current_cpu_time>9999.0</current_cpu_time>\n
            <checkpoint_cpu_time>8888.0</checkpoint_cpu_time>\n
            <want_network />
            <fraction_done>0.1</fraction_done>
            <other_pid>345</other_pid>
        "
            .into(),
        )
        .unwrap();

        assert_eq!(expectation, result);
    }

    #[test]
    /// In this test we create two IPCStreams which communicate via a shared AppChannel.
    fn test_inmemory_stream() {
        let fixture = AppStatusData {
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

        let client_connection = AppHandle::new(Arc::clone(&app_channel));
        let app_connection = ClientHandle::new(Arc::clone(&app_channel));

        client_connection
            .send(StatusMessage::AppStatus(fixture.clone()))
            .wait()
            .unwrap();

        let timeout = Duration::from_millis(1500);
        let output = app_connection
            .into_future()
            .map(|(v, _)| v.unwrap())
            .map_err(|(v, _)| v)
            .timeout(timeout)
            .map_err(|_| ())
            .wait();
        match output {
            Ok(result) => assert_eq!(expectation, result),
            Err(_) => panic!("Failed to get result within time limit"),
        };
    }
}
