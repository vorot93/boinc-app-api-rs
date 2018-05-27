use failure;
use treexml;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "data parse error: {}", _0)]
    DataParseError(failure::Error),
    #[fail(display = "logic error: {}", _0)]
    LogicError(failure::Error),
}

impl From<treexml::Error> for Error {
    fn from(v: treexml::Error) -> Self {
        Error::DataParseError(v.into())
    }
}
