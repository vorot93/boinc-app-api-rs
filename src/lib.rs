#![cfg_attr(feature="cargo-clippy", allow(mutex_atomic))]

#[macro_use]
extern crate error_chain;
extern crate libc;
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;
extern crate treexml;
extern crate treexml_util;

use std::io::Write;
use std::os::linux::fs::MetadataExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;

use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::CStr;
use std::sync::{Arc, Mutex, mpsc};
use mpsc::channel;
use libc::c_char;

pub mod errors {
    error_chain!{
        links {
            XMLError(::treexml::Error, ::treexml::ErrorKind);
        }

        errors {
            MissingDataInIPCMessage(channel: String, data: String) {
                description("missing data in IPC message")
                display("missing data {} in IPC channel {}", data, channel)
            }
            InvalidVariantInIPCChannel(channel: String, variant: String) {
                description("invalid variant in IPC channel")
                display("invalid variant {} in IPC channel {}", variant, channel)
            }
        }
    }
}

const MSG_CHANNEL_SIZE: usize = 1024;

#[repr(C)]
pub struct MSG_CHANNEL {
    buf: [c_char; MSG_CHANNEL_SIZE],
}

impl MSG_CHANNEL {
    pub fn is_empty(&self) -> bool {
        self.buf[0] == 0
    }

    pub fn clear(&mut self) {
        self.buf[0] = 0;
    }

    pub fn peek(&self) -> Option<Vec<u8>> {
        if self.is_empty() {
            None
        } else {
            let mut v: Vec<c_char> = (&self.buf[1..MSG_CHANNEL_SIZE - 2]).into();
            v[MSG_CHANNEL_SIZE - 4] = 0;
            Some(unsafe { CStr::from_ptr(v.as_ptr()) }.to_bytes().into())
        }
    }

    pub fn pop(&mut self) -> Option<Vec<u8>> {
        let v = self.peek();
        self.clear();
        v
    }

    pub fn force_push<T>(&mut self, msg: T)
    where
        T: Into<Vec<u8>>,
    {
        let v = msg.into();
        self.buf[0] = 1;
        for (i, e) in v.iter().enumerate().take(std::cmp::min(
            v.len(),
            MSG_CHANNEL_SIZE - 2,
        ))
        {
            let c = *e as c_char;
            self.buf[i + 1] = c;
            if c == 0 {
                break;
            }
        }
        self.buf[MSG_CHANNEL_SIZE - 1] = 0;
    }

    pub fn push<T>(&mut self, msg: T) -> bool
    where
        T: Into<Vec<u8>>,
    {
        if !self.is_empty() {
            false
        } else {
            self.force_push(msg);
            true
        }
    }
}

impl Default for MSG_CHANNEL {
    fn default() -> Self {
        Self { buf: [0; MSG_CHANNEL_SIZE] }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct SHARED_MEM {
    process_control_request: MSG_CHANNEL,
    process_control_reply: MSG_CHANNEL,
    graphics_request: MSG_CHANNEL,
    graphics_reply: MSG_CHANNEL,
    heartbeat: MSG_CHANNEL,
    app_status: MSG_CHANNEL,
    trickle_up: MSG_CHANNEL,
    trickle_down: MSG_CHANNEL,
}

impl SHARED_MEM {
    pub fn get_channel(&self, m: MsgChannel) -> &MSG_CHANNEL {
        match m {
            MsgChannel::ProcessControlRequest => &self.process_control_request,
            MsgChannel::ProcessControlReply => &self.process_control_reply,
            MsgChannel::GraphicsRequest => &self.graphics_request,
            MsgChannel::GraphicsReply => &self.graphics_reply,
            MsgChannel::Heartbeat => &self.heartbeat,
            MsgChannel::AppStatus => &self.app_status,
            MsgChannel::TrickleUp => &self.trickle_up,
            MsgChannel::TrickleDown => &self.trickle_down,
        }
    }
    pub fn get_channel_mut(&mut self, m: MsgChannel) -> &mut MSG_CHANNEL {
        match m {
            MsgChannel::ProcessControlRequest => &mut self.process_control_request,
            MsgChannel::ProcessControlReply => &mut self.process_control_reply,
            MsgChannel::GraphicsRequest => &mut self.graphics_request,
            MsgChannel::GraphicsReply => &mut self.graphics_reply,
            MsgChannel::Heartbeat => &mut self.heartbeat,
            MsgChannel::AppStatus => &mut self.app_status,
            MsgChannel::TrickleUp => &mut self.trickle_up,
            MsgChannel::TrickleDown => &mut self.trickle_down,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProcessControlRequest {
    Quit,
    Suspend,
    Resume,
    Abort,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphicsReply {
    pub web_graphics_url: Option<String>,
    pub remote_desktop_addr: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Heartbeat {
    pub wss: Option<f64>,
    pub max_wss: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppStatus {
    pub current_cpu_time: f64,
    pub checkpoint_cpu_time: f64,
    pub want_network: bool,
    pub fraction_done: f64,
    pub other_pid: Option<i64>,
    pub bytes_sent: Option<f64>,
    pub bytes_received: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrickleDown {
    pub have_trickle_down: bool,
    pub upload_file_status: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrickleUp {
    pub have_new_upload_file: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MsgChannel {
    #[serde(rename = "process_control_request")]
    ProcessControlRequest,
    #[serde(rename = "process_control_reply")]
    ProcessControlReply,
    #[serde(rename = "graphics_request")]
    GraphicsRequest,
    #[serde(rename = "graphics_reply")]
    GraphicsReply,
    #[serde(rename = "heartbeat")]
    Heartbeat,
    #[serde(rename = "app_status")]
    AppStatus,
    #[serde(rename = "trickle_up")]
    TrickleUp,
    #[serde(rename = "trickle_down")]
    TrickleDown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "channel", content = "payload")]
pub enum Message {
    #[serde(rename = "process_control_request")]
    ProcessControlRequest(ProcessControlRequest),
    #[serde(rename = "process_control_reply")]
    ProcessControlReply,
    #[serde(rename = "graphics_request")]
    GraphicsRequest,
    #[serde(rename = "graphics_reply")]
    GraphicsReply(GraphicsReply),
    #[serde(rename = "heartbeat")]
    Heartbeat(Heartbeat),
    #[serde(rename = "app_status")]
    AppStatus(AppStatus),
    #[serde(rename = "trickle_up")]
    TrickleUp(TrickleUp),
    #[serde(rename = "trickle_down")]
    TrickleDown(TrickleDown),
}

impl Message {
    pub fn from_ipc(c: MsgChannel, v: &str) -> errors::Result<Self> {
        let doc = treexml::Document::parse(format!("<IPC>{}</IPC>", v).as_bytes())?;
        let root = doc.root.unwrap();
        match c {
            MsgChannel::ProcessControlRequest => {
                root.children
                    .get(0)
                    .ok_or_else(|| {
                        errors::Error::from(errors::ErrorKind::InvalidVariantInIPCChannel(
                            "process_control_request".into(),
                            "(none)".into(),
                        ))
                    })
                    .and_then(|n| match &*n.name {
                        "quit" => Ok(ProcessControlRequest::Quit),
                        "suspend" => Ok(ProcessControlRequest::Suspend),
                        "resume" => Ok(ProcessControlRequest::Resume),
                        "abort" => Ok(ProcessControlRequest::Abort),
                        _ => {
                            Err(errors::Error::from(
                                errors::ErrorKind::InvalidVariantInIPCChannel(
                                    "process_control_request".into(),
                                    n.name.clone(),
                                ),
                            ))
                        }
                    })
                    .map(Message::ProcessControlRequest)
            }
            MsgChannel::ProcessControlReply => Ok(Message::ProcessControlReply),
            MsgChannel::GraphicsRequest => Ok(Message::GraphicsRequest),
            MsgChannel::GraphicsReply => {
                Ok(Message::GraphicsReply(GraphicsReply {
                    web_graphics_url: root.find_value("web_graphics_url")?,
                    remote_desktop_addr: root.find_value("remote_desktop_addr")?,
                }))
            }
            MsgChannel::Heartbeat => {
                Ok(Message::Heartbeat(Heartbeat {
                    wss: root.find_value("wss")?,
                    max_wss: root.find_value("max_wss")?,
                }))
            }
            MsgChannel::AppStatus => {
                Ok(Message::AppStatus(AppStatus {
                    current_cpu_time: match root.find_value("current_cpu_time")?.ok_or_else(|| {
                        errors::ErrorKind::MissingDataInIPCMessage(
                            "app_status".into(),
                            "current_cpu_time".into(),
                        ).into()
                    }) {
                        Ok(v) => v,
                        Err(v) => {
                            return Err(v);
                        }
                    },
                    checkpoint_cpu_time: match root.find_value("checkpoint_cpu_time")?
                        .ok_or_else(|| {
                            errors::ErrorKind::MissingDataInIPCMessage(
                                "app_status".into(),
                                "checkpoint_cpu_time".into(),
                            ).into()
                        }) {
                        Ok(v) => v,
                        Err(v) => {
                            return Err(v);
                        }
                    },
                    want_network: root.find_child(|n| n.name == "want_network").is_some(),
                    fraction_done: match root.find_value("fraction_done")?.ok_or_else(|| {
                        errors::ErrorKind::MissingDataInIPCMessage(
                            "app_status".into(),
                            "fraction_done".into(),
                        ).into()
                    }) {
                        Ok(v) => v,
                        Err(v) => {
                            return Err(v);
                        }
                    },
                    other_pid: treexml_util::find_value("other_pid", &root)?,
                    bytes_sent: match root.find_value("bytes_sent") {
                        Ok(v) => v,
                        Err(e) => {
                            match *e.kind() {
                                treexml::ErrorKind::ElementNotFound(_) => None,
                                _ => {
                                    return Err(e.into());
                                }
                            }
                        }
                    },
                    bytes_received: match root.find_value("bytes_received") {
                        Ok(v) => v,
                        Err(e) => {
                            match *e.kind() {
                                treexml::ErrorKind::ElementNotFound(_) => None,
                                _ => {
                                    return Err(e.into());
                                }
                            }
                        }
                    },
                }))
            }
            MsgChannel::TrickleDown => {
                Ok(Message::TrickleDown(TrickleDown {
                    have_trickle_down: root.find_child(|n| n.name == "have_trickle_down").is_some(),
                    upload_file_status: root.find_child(|n| n.name == "upload_file_status")
                        .is_some(),
                }))
            }
            MsgChannel::TrickleUp => {
                Ok(Message::TrickleUp(TrickleUp {
                    have_new_upload_file: root.find_child(|n| n.name == "have_new_upload_file")
                        .is_some(),
                }))
            }
        }
    }

    pub fn to_ipc(&self) -> (MsgChannel, Vec<u8>) {
        match *self {
            Message::ProcessControlRequest(ref v) => (
                MsgChannel::ProcessControlRequest,
                match *v {
                    ProcessControlRequest::Quit => "<quit/>",
                    ProcessControlRequest::Suspend => "<suspend/>",
                    ProcessControlRequest::Resume => "<resume/>",
                    ProcessControlRequest::Abort => "<abort/>",
                }.into(),
            ),
            Message::ProcessControlReply => (MsgChannel::ProcessControlReply, "".into()),
            Message::GraphicsRequest => (MsgChannel::GraphicsRequest, "".into()),
            Message::GraphicsReply(ref v) => (MsgChannel::GraphicsReply, {
                let mut s = String::new();
                if v.web_graphics_url.is_some() {
                    s += &format!(
                        "<web_graphics_url>{}</web_graphics_url>\n",
                        v.web_graphics_url.as_ref().unwrap()
                    );
                }
                if v.remote_desktop_addr.is_some() {
                    s += &format!(
                        "<remote_desktop_addr>{}</remote_desktop_addr>\n",
                        v.remote_desktop_addr.as_ref().unwrap()
                    );
                }
                s.into()
            }),
            Message::Heartbeat(ref v) => (MsgChannel::Heartbeat, {
                let mut s = String::new();
                if v.wss.is_some() {
                    s += &format!("<wss>{}</wss>\n", v.wss.as_ref().unwrap());
                }
                if v.max_wss.is_some() {
                    s += &format!("<max_wss>{}</max_wss>\n", v.max_wss.as_ref().unwrap());
                }
                s.into()
            }),
            Message::AppStatus(ref v) => (MsgChannel::AppStatus, {
                let mut s = String::new();
                s += &format!(
                    "<current_cpu_time>{}</current_cpu_time>\n",
                    v.current_cpu_time
                );
                s += &format!(
                    "<checkpoint_cpu_time>{}</checkpoint_cpu_time>\n",
                    v.checkpoint_cpu_time
                );
                if v.want_network {
                    s += "<want_network>1</want_network>\n";
                }
                s += &format!("<fraction_done>{}</fraction_done>\n", v.fraction_done);
                if v.other_pid.is_some() {
                    s += &format!("<other_pid>{}</other_pid>\n", v.other_pid.as_ref().unwrap());
                }
                if v.bytes_sent.is_some() {
                    s += &format!(
                        "<bytes_sent>{}</bytes_sent>\n",
                        v.bytes_sent.as_ref().unwrap()
                    );
                }
                if v.bytes_received.is_some() {
                    s += &format!(
                        "<bytes_received>{}</bytes_received>\n",
                        v.bytes_received.as_ref().unwrap()
                    );
                }
                s.into()
            }),
            Message::TrickleDown(ref v) => (MsgChannel::TrickleDown, {
                let mut s = String::new();
                if v.have_trickle_down {
                    s += "<have_new_trickle_down/>\n";
                }
                if v.upload_file_status {
                    s += "<upload_file_status/>\n";
                }
                s.into()
            }),
            Message::TrickleUp(_) => (MsgChannel::TrickleUp, "<have_new_trickle_up/>".into()),
        }
    }
}

pub trait AppChannel {
    /// Internal accessor for shared memory.
    fn transaction(&self, f: Box<FnMut(&mut SHARED_MEM)>);

    /// Check if `MsgChannel` contains a message.
    fn is_empty(&self, c: MsgChannel) -> bool {
        let (tx, rx) = channel();
        self.transaction(Box::new(move |data| {
            tx.send(data.get_channel(c).is_empty()).unwrap();
        }));
        rx.recv().unwrap()
    }

    /// Check `MsgChannel` contents without extracting.
    fn peek(&self, c: MsgChannel) -> Option<String> {
        let (tx, rx) = channel();
        self.transaction(Box::new(move |data| {
            tx.send(data.get_channel(c).peek().map(|v| {
                String::from_utf8_lossy(&v).into_owned()
            })).unwrap();
        }));
        rx.recv().unwrap()
    }

    /// Extract data from the specified `MsgChannel`.
    fn receive(&self, c: MsgChannel) -> Option<String> {
        let (tx, rx) = channel();
        self.transaction(Box::new(move |data| {
            tx.send(data.get_channel_mut(c).pop().map(|v| {
                String::from_utf8_lossy(&v).into_owned()
            })).unwrap();
        }));
        rx.recv().unwrap()
    }

    /// Clear channel contents.
    fn clear(&self, c: MsgChannel) {
        let (tx, rx) = channel();
        self.transaction(Box::new(move |data| {
            tx.send(data.get_channel_mut(c).clear()).unwrap();
        }));
        rx.recv().unwrap()
    }

    /// Send the data to the channel.
    fn push(&self, m: &Message) -> bool {
        let (c, v) = m.to_ipc();
        let (tx, rx) = channel();
        self.transaction(Box::new(move |data| {
            tx.send(data.get_channel_mut(c).push(v.as_slice())).unwrap();
        }));
        rx.recv().unwrap()
    }

    /// Send the data to the channel. This version does not check message validity and is thus marked unsafe.
    unsafe fn push_unchecked(&self, c: MsgChannel, v: Vec<u8>) -> bool {
        let (tx, rx) = channel();
        self.transaction(Box::new(move |data| {
            tx.send(data.get_channel_mut(c).push(v.clone())).unwrap();
        }));
        rx.recv().unwrap()
    }

    /// Overwrite channel contents.
    fn force(&self, m: &Message) {
        let (c, v) = m.to_ipc();
        let (tx, rx) = channel();
        self.transaction(Box::new(move |data| {
            tx.send(data.get_channel_mut(c).force_push(v.as_slice()))
                .unwrap();
        }));
        rx.recv().unwrap()
    }

    /// Overwrite channel contents. This version does not check message validity and is thus marked unsafe.
    unsafe fn force_unchecked(&self, c: MsgChannel, v: Vec<u8>) {
        let (tx, rx) = channel();
        self.transaction(Box::new(move |data| {
            tx.send(data.get_channel_mut(c).force_push(v.clone()))
                .unwrap();
        }));
        rx.recv().unwrap()
    }
}

#[derive(Default)]
pub struct MemoryAppChannel {
    data: Mutex<SHARED_MEM>,
}

impl AppChannel for MemoryAppChannel {
    fn transaction(&self, mut f: Box<FnMut(&mut SHARED_MEM)>) {
        f(&mut *self.data.lock().unwrap());
    }
}

/// Wrapper to operate on shared mapped memory.
pub struct MmapAppChannel {
    data: Mutex<*mut SHARED_MEM>,
}

impl Drop for MmapAppChannel {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(
                *self.data.lock().unwrap() as *mut libc::c_void,
                std::mem::size_of::<SHARED_MEM>(),
            );
        }
    }
}

impl AppChannel for MmapAppChannel {
    fn transaction(&self, mut f: Box<FnMut(&mut SHARED_MEM)>) {
        let mut p = self.data.lock().unwrap();
        f(unsafe { &mut **p })
    }
}

impl MmapAppChannel {
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<Self> {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .mode(0o666)
            .open(path)?;

        let sz = std::mem::size_of::<SHARED_MEM>();
        let md = f.metadata()?;

        if md.st_size() < sz as u64 {
            f.write_all(&vec![0; sz])?;
        }

        let shmem = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                sz,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_FILE | libc::MAP_SHARED,
                f.as_raw_fd(),
                0,
            )
        };

        if shmem == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error());
        }

        Ok(Self { data: Mutex::new(shmem as *mut SHARED_MEM) })
    }
}


// true == Send, false == Receive
pub type ChannelDirMap = HashMap<MsgChannel, bool>;

#[derive(Copy, Clone, Debug)]
pub enum QueueMode {
    Client,
    App,
}

impl From<QueueMode> for ChannelDirMap {
    fn from(v: QueueMode) -> ChannelDirMap {
        match v {
            QueueMode::Client => {
                hashmap!{
                    MsgChannel::ProcessControlRequest => true,
                    MsgChannel::ProcessControlReply => false,
                    MsgChannel::GraphicsRequest => true,
                    MsgChannel::GraphicsReply => false,
                    MsgChannel::Heartbeat => true,
                    MsgChannel::AppStatus => false,
                    MsgChannel::TrickleUp => false,
                    MsgChannel::TrickleDown => true,
                }
            }
            QueueMode::App => {
                hashmap!{
                    MsgChannel::ProcessControlRequest => false,
                    MsgChannel::ProcessControlReply => true,
                    MsgChannel::GraphicsRequest => false,
                    MsgChannel::GraphicsReply => true,
                    MsgChannel::Heartbeat => false,
                    MsgChannel::AppStatus => true,
                    MsgChannel::TrickleUp => true,
                    MsgChannel::TrickleDown => false,
                }
            }
        }
    }
}

fn get_channel_groups(g: HashMap<MsgChannel, bool>) -> (HashSet<MsgChannel>, HashSet<MsgChannel>) {
    let mut snd = HashSet::<MsgChannel>::new();
    let mut rcv = HashSet::<MsgChannel>::new();
    for (c, is_snd) in g {
        (if is_snd { &mut snd } else { &mut rcv }).insert(c);
    }
    (snd, rcv)
}

pub type SharedAppChannel = Arc<AppChannel + Send + Sync + 'static>;

pub struct IPCStream {
    kill_switch: Arc<std::sync::atomic::AtomicBool>,
    app_channel: Option<SharedAppChannel>,
    sender: Option<std::thread::JoinHandle<()>>,
    receiver: Option<std::thread::JoinHandle<()>>,
    send_queues: Arc<Mutex<HashMap<MsgChannel, VecDeque<Vec<u8>>>>>,
}

impl Drop for IPCStream {
    fn drop(&mut self) {
        self.stop()
    }
}

impl IPCStream {
    fn stop(&mut self) {
        self.kill_switch.store(
            true,
            std::sync::atomic::Ordering::Relaxed,
        );
        self.sender.take().map(|t| t.join().unwrap());
        self.receiver.take().map(|t| t.join().unwrap());
    }

    pub fn new<F, E>(app_channel: SharedAppChannel, dir_map: ChannelDirMap, rcv_cb: F, err_cb: E) -> Self
    where
        F: Fn(Message) + Send + 'static,
        E: Fn(errors::Error) + Send + 'static,
    {
        let (snd_c, rcv_c) = get_channel_groups(dir_map.into());
        let send_queues = {
            let mut v = HashMap::<MsgChannel, VecDeque<Vec<u8>>>::new();
            for chan in snd_c {
                v.insert(chan, VecDeque::<Vec<u8>>::new());
            }
            Arc::new(Mutex::new(v))
        };
        let kill_switch = Arc::new(std::sync::atomic::AtomicBool::new(false));
        Self {
            sender: Some(std::thread::spawn({
                let app_channel = Arc::clone(&app_channel);
                let send_queues = Arc::clone(&send_queues);
                let kill_switch = Arc::clone(&kill_switch);
                move || loop {
                    if kill_switch.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }
                    for (c, q) in send_queues.lock().unwrap().iter_mut() {
                        if match q.front_mut() {
                            None => false,
                            Some(v) => unsafe { app_channel.push_unchecked(*c, v.clone()) },
                        }
                        {
                            q.pop_front();
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(300));
                }
            })),
            receiver: Some(std::thread::spawn({
                let app_channel = Arc::clone(&app_channel);
                let kill_switch = Arc::clone(&kill_switch);
                move || loop {
                    if kill_switch.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }
                    for chan in &rcv_c {
                        match app_channel.receive(*chan) {
                            None => {}
                            Some(s) => {
                                match Message::from_ipc(*chan, &s) {
                                    Ok(msg) => rcv_cb(msg),
                                    Err(e) => err_cb(e),
                                }
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(300));
                }
            })),
            kill_switch: kill_switch,
            send_queues: send_queues,
            app_channel: Some(Arc::clone(&app_channel)),
        }
    }

    pub fn send(&self, msg: Message) -> bool {
        let (c, v) = msg.to_ipc();
        match self.send_queues.lock().unwrap().get_mut(&c) {
            Some(chan) => {
                chan.push_back(v);
                true
            }
            None => false,
        }
    }

    pub fn into_inner(mut self) -> SharedAppChannel {
        self.stop();
        self.app_channel.take().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_ipc() {
        let expectation = Message::AppStatus(AppStatus {
            current_cpu_time: 9999.0,
            checkpoint_cpu_time: 8888.0,
            want_network: true,
            fraction_done: 0.1,
            other_pid: Some(345),

            bytes_received: None,
            bytes_sent: None,
        });

        let result = Message::from_ipc(
            MsgChannel::AppStatus,
            "
            <current_cpu_time>9999.0</current_cpu_time>\n
            <checkpoint_cpu_time>8888.0</checkpoint_cpu_time>\n
            <want_network />
            <fraction_done>0.1</fraction_done>
            <other_pid>345</other_pid>
        ",
        ).unwrap();

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
        let app_channel: SharedAppChannel = Arc::new(MemoryAppChannel::default());
        let input_stream = IPCStream::new(
            Arc::clone(&app_channel),
            ChannelDirMap::from(QueueMode::App),
            |_| {},
            |e| { println!("{}", e); },
        );
        let (tx, rx) = channel();
        let output_stream = IPCStream::new(
            Arc::clone(&app_channel),
            QueueMode::Client.into(),
            move |msg| { tx.send(msg).unwrap(); },
            |e| { println!("{}", e); },
        );
        let _ = output_stream;

        let expectation = Some(Message::AppStatus(fixture.clone()));
        let mut result = None;

        input_stream.send(Message::AppStatus(fixture.clone()));

        for _ in 0..5 {
            if let Ok(v) = rx.try_recv() {
                result = Some(v);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }

        assert_eq!(expectation, result);
    }
}
