extern crate failure;
extern crate std;
extern crate treexml;
extern crate treexml_util;

use errors;

use self::std::convert::TryFrom;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIterator)]
pub enum ControlMsgChannel {
    #[serde(rename = "process_control_request")] ProcessControlRequest,
    #[serde(rename = "graphics_request")] GraphicsRequest,
    #[serde(rename = "heartbeat")] Heartbeat,
    #[serde(rename = "trickle_down")] TrickleDown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIterator)]
pub enum StatusMsgChannel {
    #[serde(rename = "process_control_reply")] ProcessControlReply,
    #[serde(rename = "graphics_reply")] GraphicsReply,
    #[serde(rename = "app_status")] AppStatus,
    #[serde(rename = "trickle_up")] TrickleUp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MsgChannel {
    ProcessControlRequest,
    ProcessControlReply,
    GraphicsRequest,
    GraphicsReply,
    Heartbeat,
    AppStatus,
    TrickleUp,
    TrickleDown,
}

impl From<ControlMsgChannel> for MsgChannel {
    fn from(m: ControlMsgChannel) -> MsgChannel {
        match m {
            ControlMsgChannel::ProcessControlRequest => MsgChannel::ProcessControlRequest,
            ControlMsgChannel::GraphicsRequest => MsgChannel::GraphicsRequest,
            ControlMsgChannel::Heartbeat => MsgChannel::Heartbeat,
            ControlMsgChannel::TrickleDown => MsgChannel::TrickleDown,
        }
    }
}

impl From<StatusMsgChannel> for MsgChannel {
    fn from(m: StatusMsgChannel) -> MsgChannel {
        match m {
            StatusMsgChannel::ProcessControlReply => MsgChannel::ProcessControlReply,
            StatusMsgChannel::GraphicsReply => MsgChannel::GraphicsReply,
            StatusMsgChannel::AppStatus => MsgChannel::AppStatus,
            StatusMsgChannel::TrickleUp => MsgChannel::TrickleUp,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "channel", content = "data")]
/// Message from control to app
pub enum ControlMessage {
    #[serde(rename = "process_control_request")] ProcessControlRequest(ProcessControlRequest),
    #[serde(rename = "graphics_request")] GraphicsRequest,
    #[serde(rename = "heartbeat")] Heartbeat(Heartbeat),
    #[serde(rename = "trickle_down")] TrickleDown(TrickleDown),
}

impl TryFrom<(ControlMsgChannel, Vec<u8>)> for ControlMessage {
    type Error = errors::Error;
    fn try_from(m: (ControlMsgChannel, Vec<u8>)) -> Result<Self, Self::Error> {
        let c = m.0;
        let v = m.1;
        let doc = treexml::Document::parse(
            format!("<IPC>{}</IPC>", &String::from_utf8_lossy(&v)).as_bytes(),
        )?;
        let root = doc.root.unwrap();
        match c {
            ControlMsgChannel::ProcessControlRequest => root.children
                .get(0)
                .ok_or_else(|| errors::Error::InvalidVariantInIPCChannel {
                    channel: "process_control_request".into(),
                    variant: "(none)".into(),
                })
                .and_then(|n| match &*n.name {
                    "quit" => Ok(ProcessControlRequest::Quit),
                    "suspend" => Ok(ProcessControlRequest::Suspend),
                    "resume" => Ok(ProcessControlRequest::Resume),
                    "abort" => Ok(ProcessControlRequest::Abort),
                    _ => Err(errors::Error::InvalidVariantInIPCChannel {
                        channel: "process_control_request".into(),
                        variant: n.name.clone(),
                    }),
                })
                .map(ControlMessage::ProcessControlRequest),
            ControlMsgChannel::GraphicsRequest => Ok(ControlMessage::GraphicsRequest),
            ControlMsgChannel::Heartbeat => Ok(ControlMessage::Heartbeat(Heartbeat {
                wss: root.find_value("wss")?,
                max_wss: root.find_value("max_wss")?,
            })),
            ControlMsgChannel::TrickleDown => Ok(ControlMessage::TrickleDown(TrickleDown {
                have_trickle_down: root.find_child(|n| n.name == "have_trickle_down").is_some(),
                upload_file_status: root.find_child(|n| n.name == "upload_file_status")
                    .is_some(),
            })),
        }
    }
}

impl From<ControlMessage> for (ControlMsgChannel, Vec<u8>) {
    fn from(m: ControlMessage) -> Self {
        match m {
            ControlMessage::ProcessControlRequest(v) => (
                ControlMsgChannel::ProcessControlRequest,
                match v {
                    ProcessControlRequest::Quit => "<quit/>",
                    ProcessControlRequest::Suspend => "<suspend/>",
                    ProcessControlRequest::Resume => "<resume/>",
                    ProcessControlRequest::Abort => "<abort/>",
                }.into(),
            ),
            ControlMessage::GraphicsRequest => (ControlMsgChannel::GraphicsRequest, "".into()),
            ControlMessage::Heartbeat(v) => (ControlMsgChannel::Heartbeat, {
                let mut s = String::new();
                if let Some(v) = v.wss {
                    s += &format!("<wss>{}</wss>\n", v);
                }
                if let Some(v) = v.max_wss {
                    s += &format!("<max_wss>{}</max_wss>\n", v);
                }
                s.into()
            }),
            ControlMessage::TrickleDown(v) => (ControlMsgChannel::TrickleDown, {
                let mut s = String::new();
                if v.have_trickle_down {
                    s += "<have_new_trickle_down/>\n";
                }
                if v.upload_file_status {
                    s += "<upload_file_status/>\n";
                }
                s.into()
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "channel", content = "payload")]
/// Message from app to control
pub enum StatusMessage {
    #[serde(rename = "process_control_reply")] ProcessControlReply,
    #[serde(rename = "graphics_reply")] GraphicsReply(GraphicsReply),
    #[serde(rename = "app_status")] AppStatus(AppStatus),
    #[serde(rename = "trickle_up")] TrickleUp(TrickleUp),
}

impl TryFrom<(StatusMsgChannel, Vec<u8>)> for StatusMessage {
    type Error = errors::Error;
    fn try_from(m: (StatusMsgChannel, Vec<u8>)) -> Result<Self, Self::Error> {
        let c = m.0;
        let v = m.1;
        let doc = treexml::Document::parse(
            format!("<IPC>{}</IPC>", &String::from_utf8_lossy(&v)).as_bytes(),
        )?;
        let root = doc.root.unwrap();
        match c {
            StatusMsgChannel::ProcessControlReply => Ok(StatusMessage::ProcessControlReply),
            StatusMsgChannel::GraphicsReply => Ok(StatusMessage::GraphicsReply(GraphicsReply {
                web_graphics_url: root.find_value("web_graphics_url")?,
                remote_desktop_addr: root.find_value("remote_desktop_addr")?,
            })),
            StatusMsgChannel::AppStatus => Ok(StatusMessage::AppStatus(AppStatus {
                current_cpu_time: match root.find_value("current_cpu_time")?.ok_or_else(|| {
                    errors::Error::MissingDataInIPCMessage {
                        channel: "app_status".into(),
                        data: "current_cpu_time".into(),
                    }.into()
                }) {
                    Ok(v) => v,
                    Err(v) => {
                        return Err(v);
                    }
                },
                checkpoint_cpu_time: match root.find_value("checkpoint_cpu_time")?.ok_or_else(
                    || {
                        errors::Error::MissingDataInIPCMessage {
                            channel: "app_status".into(),
                            data: "checkpoint_cpu_time".into(),
                        }.into()
                    },
                ) {
                    Ok(v) => v,
                    Err(v) => {
                        return Err(v);
                    }
                },
                want_network: root.find_child(|n| n.name == "want_network").is_some(),
                fraction_done: match root.find_value("fraction_done")?.ok_or_else(|| {
                    errors::Error::MissingDataInIPCMessage {
                        channel: "app_status".into(),
                        data: "fraction_done".into(),
                    }.into()
                }) {
                    Ok(v) => v,
                    Err(v) => {
                        return Err(v);
                    }
                },
                other_pid: treexml_util::find_value("other_pid", &root)?,
                bytes_sent: treexml_util::find_value("bytes_sent", &root)?,
                bytes_received: treexml_util::find_value("bytes_received", &root)?,
            })),
            StatusMsgChannel::TrickleUp => Ok(StatusMessage::TrickleUp(TrickleUp {
                have_new_upload_file: root.find_child(|n| n.name == "have_new_upload_file")
                    .is_some(),
            })),
        }
    }
}

impl From<StatusMessage> for (StatusMsgChannel, Vec<u8>) {
    fn from(v: StatusMessage) -> (StatusMsgChannel, Vec<u8>) {
        match v {
            StatusMessage::ProcessControlReply => {
                (StatusMsgChannel::ProcessControlReply, "".into())
            }
            StatusMessage::GraphicsReply(ref v) => (StatusMsgChannel::GraphicsReply, {
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
            StatusMessage::AppStatus(ref v) => (StatusMsgChannel::AppStatus, {
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
            StatusMessage::TrickleUp(_) => {
                (StatusMsgChannel::TrickleUp, "<have_new_trickle_up/>".into())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", content = "payload")]
pub enum Message {
    Control(ControlMessage),
    Status(StatusMessage),
}

impl From<Message> for (MsgChannel, Vec<u8>) {
    fn from(v: Message) -> (MsgChannel, Vec<u8>) {
        match v {
            Message::Control(m) => {
                let (id, payload) = m.into();
                (id.into(), payload)
            }
            Message::Status(m) => {
                let (id, payload) = m.into();
                (id.into(), payload)
            }
        }
    }
}
