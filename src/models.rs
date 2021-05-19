use anyhow::format_err;
use serde::{Deserialize, Serialize};
use treexml::Element;
use treexml_util::{parse_node, ElementExt};

fn parse_xml_data(s: &[u8]) -> anyhow::Result<Element> {
    Ok(parse_node(&format!("<root>{}</root>", &String::from_utf8_lossy(s)))?.unwrap())
}

pub(crate) trait MsgChannelXml
where
    Self: Sized,
{
    fn from_xml(s: &[u8]) -> anyhow::Result<Self>;
    fn to_xml(&self) -> Vec<u8>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ControlMsgChannel {
    #[serde(rename = "process_control_request")]
    ProcessControlRequest,
    #[serde(rename = "graphics_request")]
    GraphicsRequest,
    #[serde(rename = "heartbeat")]
    Heartbeat,
    #[serde(rename = "trickle_down")]
    TrickleDown,
}

impl ControlMsgChannel {
    pub fn enum_iter() -> impl Iterator<Item = Self> {
        [
            Self::ProcessControlRequest,
            Self::GraphicsRequest,
            Self::Heartbeat,
            Self::TrickleDown,
        ]
        .into_iter()
        .copied()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatusMsgChannel {
    #[serde(rename = "process_control_reply")]
    ProcessControlReply,
    #[serde(rename = "graphics_reply")]
    GraphicsReply,
    #[serde(rename = "app_status")]
    AppStatus,
    #[serde(rename = "trickle_up")]
    TrickleUp,
}

impl StatusMsgChannel {
    pub fn enum_iter() -> impl Iterator<Item = Self> {
        [
            Self::ProcessControlReply,
            Self::GraphicsReply,
            Self::AppStatus,
            Self::TrickleUp,
        ]
        .into_iter()
        .copied()
    }
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

impl MsgChannelXml for ProcessControlRequest {
    fn from_xml(s: &[u8]) -> anyhow::Result<Self> {
        use self::ProcessControlRequest::*;

        let mut root = parse_xml_data(s)?;

        let variant = root
            .children
            .pop()
            .ok_or_else(|| format_err!("No variant detected"))?
            .name;

        Ok(match variant.as_ref() {
            "quit" => Quit,
            "suspend" => Suspend,
            "resume" => Resume,
            "abort" => Abort,
            _ => {
                return Err(format_err!("Invalid variant detected: {}", &variant));
            }
        })
    }

    fn to_xml(&self) -> Vec<u8> {
        use self::ProcessControlRequest::*;

        match *self {
            Quit => "<quit/>",
            Suspend => "<suspend/>",
            Resume => "<resume/>",
            Abort => "<abort/>",
        }
        .into()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphicsReplyData {
    pub web_graphics_url: Option<String>,
    pub remote_desktop_addr: Option<String>,
}

impl MsgChannelXml for GraphicsReplyData {
    fn from_xml(s: &[u8]) -> anyhow::Result<Self> {
        let root = parse_xml_data(s)?;

        Ok(Self {
            web_graphics_url: root.find_value0("web_graphics_url")?,
            remote_desktop_addr: root.find_value0("remote_desktop_addr")?,
        })
    }

    fn to_xml(&self) -> Vec<u8> {
        let mut s = String::new();
        if let Some(v) = self.web_graphics_url.as_ref() {
            s += &format!("<web_graphics_url>{}</web_graphics_url>\n", v);
        }
        if let Some(v) = self.remote_desktop_addr.as_ref() {
            s += &format!("<remote_desktop_addr>{}</remote_desktop_addr>\n", v);
        }
        s.into()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Heartbeat {
    pub wss: Option<f64>,
    pub max_wss: Option<f64>,
}

impl MsgChannelXml for Heartbeat {
    fn from_xml(s: &[u8]) -> anyhow::Result<Self> {
        let root = parse_xml_data(s)?;

        Ok(Self {
            wss: root.find_value0("wss")?,
            max_wss: root.find_value0("max_wss")?,
        })
    }

    fn to_xml(&self) -> Vec<u8> {
        let mut s = String::new();
        if let Some(v) = self.wss {
            s += &format!("<wss>{}</wss>\n", v);
        }
        if let Some(v) = self.max_wss {
            s += &format!("<max_wss>{}</max_wss>\n", v);
        }
        s.into()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppStatusData {
    pub current_cpu_time: f64,
    pub checkpoint_cpu_time: f64,
    pub want_network: bool,
    pub fraction_done: f64,
    pub other_pid: Option<i64>,
    pub bytes_sent: Option<f64>,
    pub bytes_received: Option<f64>,
}

impl MsgChannelXml for AppStatusData {
    fn from_xml(s: &[u8]) -> anyhow::Result<Self> {
        let root = parse_xml_data(s)?;

        Ok(Self {
            current_cpu_time: root.find_value1("current_cpu_time")?,
            checkpoint_cpu_time: root.find_value1("checkpoint_cpu_time")?,
            want_network: root.find_bool("want_network")?,
            fraction_done: root.find_value1("fraction_done")?,
            other_pid: root.find_value0("other_pid")?,
            bytes_sent: root.find_value0("bytes_sent")?,
            bytes_received: root.find_value0("bytes_received")?,
        })
    }

    fn to_xml(&self) -> Vec<u8> {
        let mut s = String::new();
        s += &format!(
            "<current_cpu_time>{}</current_cpu_time>\n",
            self.current_cpu_time
        );
        s += &format!(
            "<checkpoint_cpu_time>{}</checkpoint_cpu_time>\n",
            self.checkpoint_cpu_time
        );
        if self.want_network {
            s += "<want_network>1</want_network>\n";
        }
        s += &format!("<fraction_done>{}</fraction_done>\n", self.fraction_done);
        if let Some(v) = self.other_pid {
            s += &format!("<other_pid>{}</other_pid>\n", v);
        }
        if let Some(v) = self.bytes_sent {
            s += &format!("<bytes_sent>{}</bytes_sent>\n", v);
        }
        if let Some(v) = self.bytes_received {
            s += &format!("<bytes_received>{}</bytes_received>\n", v);
        }
        s.into()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrickleDownData {
    pub have_new_trickle_down: bool,
    pub upload_file_status: bool,
}

impl MsgChannelXml for TrickleDownData {
    fn from_xml(s: &[u8]) -> anyhow::Result<Self> {
        let root = parse_xml_data(s)?;

        Ok(Self {
            have_new_trickle_down: root.find_bool("have_new_trickle_down")?,
            upload_file_status: root.find_bool("upload_file_status")?,
        })
    }

    fn to_xml(&self) -> Vec<u8> {
        let mut s = String::new();
        if self.have_new_trickle_down {
            s += "<have_new_trickle_down/>\n";
        }
        if self.upload_file_status {
            s += "<upload_file_status/>\n";
        }
        s.into()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrickleUpData {
    pub have_new_trickle_up: bool,
    pub have_new_upload_file: bool,
}

impl MsgChannelXml for TrickleUpData {
    fn from_xml(s: &[u8]) -> anyhow::Result<Self> {
        let root = parse_xml_data(s)?;

        Ok(Self {
            have_new_trickle_up: root.find_bool("have_new_trickle_up")?,
            have_new_upload_file: root.find_bool("have_new_upload_file")?,
        })
    }

    fn to_xml(&self) -> Vec<u8> {
        let mut out = String::new();

        if self.have_new_trickle_up {
            out += "<have_new_trickle_up />\n";
        }
        if self.have_new_upload_file {
            out += "<have_new_upload_file />\n";
        }

        out.into()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "channel", content = "data")]
/// Message from control to app
pub enum ControlMessage {
    #[serde(rename = "process_control_request")]
    ProcessControlRequest(ProcessControlRequest),
    #[serde(rename = "graphics_request")]
    GraphicsRequest,
    #[serde(rename = "heartbeat")]
    Heartbeat(Heartbeat),
    #[serde(rename = "trickle_down")]
    TrickleDown(TrickleDownData),
}

impl ControlMessage {
    pub fn channel(&self) -> ControlMsgChannel {
        match self {
            ControlMessage::ProcessControlRequest(_) => ControlMsgChannel::ProcessControlRequest,
            ControlMessage::GraphicsRequest => ControlMsgChannel::GraphicsRequest,
            ControlMessage::Heartbeat(_) => ControlMsgChannel::Heartbeat,
            ControlMessage::TrickleDown(_) => ControlMsgChannel::TrickleDown,
        }
    }

    pub fn from_raw(c: ControlMsgChannel, b: Vec<u8>) -> anyhow::Result<Self> {
        match c {
            ControlMsgChannel::ProcessControlRequest => {
                MsgChannelXml::from_xml(&b).map(ControlMessage::ProcessControlRequest)
            }
            ControlMsgChannel::GraphicsRequest => Ok(ControlMessage::GraphicsRequest),
            ControlMsgChannel::Heartbeat => {
                MsgChannelXml::from_xml(&b).map(ControlMessage::Heartbeat)
            }
            ControlMsgChannel::TrickleDown => {
                MsgChannelXml::from_xml(&b).map(ControlMessage::TrickleDown)
            }
        }
    }
}

impl From<ControlMessage> for (ControlMsgChannel, Vec<u8>) {
    fn from(m: ControlMessage) -> Self {
        match m {
            ControlMessage::ProcessControlRequest(v) => (
                ControlMsgChannel::ProcessControlRequest,
                MsgChannelXml::to_xml(&v),
            ),
            ControlMessage::GraphicsRequest => (ControlMsgChannel::GraphicsRequest, vec![]),
            ControlMessage::Heartbeat(v) => {
                (ControlMsgChannel::Heartbeat, MsgChannelXml::to_xml(&v))
            }
            ControlMessage::TrickleDown(v) => {
                (ControlMsgChannel::TrickleDown, MsgChannelXml::to_xml(&v))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "channel", content = "payload")]
/// Message from app to control
pub enum StatusMessage {
    #[serde(rename = "process_control_reply")]
    ProcessControlReply,
    #[serde(rename = "graphics_reply")]
    GraphicsReply(GraphicsReplyData),
    #[serde(rename = "app_status")]
    AppStatus(AppStatusData),
    #[serde(rename = "trickle_up")]
    TrickleUp(TrickleUpData),
}

impl StatusMessage {
    pub fn from_raw(c: StatusMsgChannel, b: Vec<u8>) -> anyhow::Result<Self> {
        match c {
            StatusMsgChannel::ProcessControlReply => Ok(StatusMessage::ProcessControlReply),
            StatusMsgChannel::GraphicsReply => {
                MsgChannelXml::from_xml(&b).map(StatusMessage::GraphicsReply)
            }
            StatusMsgChannel::AppStatus => {
                MsgChannelXml::from_xml(&b).map(StatusMessage::AppStatus)
            }
            StatusMsgChannel::TrickleUp => {
                MsgChannelXml::from_xml(&b).map(StatusMessage::TrickleUp)
            }
        }
    }
}

impl From<StatusMessage> for (StatusMsgChannel, Vec<u8>) {
    fn from(v: StatusMessage) -> (StatusMsgChannel, Vec<u8>) {
        match v {
            StatusMessage::ProcessControlReply => {
                (StatusMsgChannel::ProcessControlReply, "".into())
            }
            StatusMessage::GraphicsReply(ref v) => (StatusMsgChannel::GraphicsReply, v.to_xml()),
            StatusMessage::AppStatus(ref v) => (StatusMsgChannel::AppStatus, v.to_xml()),
            StatusMessage::TrickleUp(ref v) => (StatusMsgChannel::TrickleUp, v.to_xml()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graphics_reply_parse() {
        let expectation = GraphicsReplyData {
            web_graphics_url: Some("my_url_a".into()),
            remote_desktop_addr: Some("my_vnc_url".into()),
        };

        let fixture = "<web_graphics_url>my_url_a</web_graphics_url><remote_desktop_addr>my_vnc_url</remote_desktop_addr>".as_bytes();

        assert_eq!(expectation, GraphicsReplyData::from_xml(fixture).unwrap());
    }

    #[test]
    fn test_trickle_up_parse() {
        let expectation = TrickleUpData {
            have_new_trickle_up: false,
            have_new_upload_file: true,
        };

        let fixture = "<have_new_upload_file>1</have_new_upload_file>".as_bytes();

        assert_eq!(expectation, TrickleUpData::from_xml(fixture).unwrap());
    }
}
