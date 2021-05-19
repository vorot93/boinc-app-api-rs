//! BOINC application API. Required for communication between crunching applications and the BOINC client.
//!
//! This crate provides common utilities for building BOINC clients and applications:
//!

#![allow(clippy::mutex_atomic)]

pub mod app_connection;
pub mod client_connection;
pub mod connection_util;
pub mod models;
pub mod shmem;

#[cfg(test)]
mod tests {
    use crate::{app_connection::*, client_connection::*, models::*, shmem::*};
    use futures::prelude::*;
    use std::{sync::Arc, time::Duration};

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

    #[tokio::test]
    /// In this test we create two IPCStreams which communicate via a shared AppChannel.
    async fn test_inmemory_stream() {
        let fixture = AppStatusData {
            current_cpu_time: 4.0,
            checkpoint_cpu_time: 5.0,
            want_network: true,
            fraction_done: 0.15,
            other_pid: None,
            bytes_sent: Some(256.0),
            bytes_received: Some(128.0),
        };
        let expectation = Some(StatusMessage::AppStatus(fixture.clone()));

        let c = MemoryAppChannel::default();
        let app_channel: SharedAppChannel = Arc::new(c);

        let mut client_connection = AppHandle::new(app_channel.clone());
        let mut app_connection = ClientHandle::new(app_channel.clone());

        client_connection
            .send(StatusMessage::AppStatus(fixture.clone()))
            .await
            .unwrap();

        let timeout = Duration::from_millis(1500);
        let result = tokio::time::timeout(timeout, app_connection.next())
            .await
            .unwrap();

        assert_eq!(expectation, result);
    }

    #[tokio::test]
    /// In this test we create two IPCStreams which communicate via a mmapped AppChannel.
    async fn test_mmap_stream() {
        let fixture = AppStatusData {
            current_cpu_time: 4.0,
            checkpoint_cpu_time: 5.0,
            want_network: true,
            fraction_done: 0.15,
            other_pid: None,
            bytes_sent: Some(256.0),
            bytes_received: Some(128.0),
        };
        let expectation = Some(StatusMessage::AppStatus(fixture.clone()));

        let tmp = tempfile::TempDir::new().unwrap();
        let mut mmap_path = tmp.path().to_path_buf();
        mmap_path.push("mmapfile");

        let c = MmapAppChannel::new(mmap_path).unwrap();
        let app_channel: SharedAppChannel = Arc::new(c);

        let mut client_connection = AppHandle::new(app_channel.clone());
        let mut app_connection = ClientHandle::new(app_channel.clone());

        client_connection
            .send(StatusMessage::AppStatus(fixture.clone()))
            .await
            .unwrap();

        let timeout = Duration::from_millis(1500);
        let result = tokio::time::timeout(timeout, app_connection.next())
            .await
            .unwrap();

        assert_eq!(expectation, result);
    }
}
