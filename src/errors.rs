use treexml;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "missing data {} in message from IPC channel {}", data, channel)]
    MissingDataInIPCMessage { channel: String, data: String },
    #[fail(display = "data parse error: {}", what)]
    DataParseError { what: String },
    #[fail(display = "logic error: {}", what)]
    LogicError { what: String },
    #[fail(display = "invalid variant {} in IPC channel {}", variant, channel)]
    InvalidVariantInIPCChannel { variant: String, channel: String },
}

impl From<treexml::Error> for Error {
    fn from(v: treexml::Error) -> Self {
        Error::DataParseError {
            what: v.to_string(),
        }
    }
}
