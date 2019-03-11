use std::str::FromStr;
use std::{fmt, str, string};

use crate::resp::{RespValue, FromResp};
use crate::stream::{Stream, StreamName, StreamNameError, ParseStreamError};

pub enum Command {
    Publish { stream: StreamName, event: Vec<u8> },
    Subscribe { streams: Vec<Stream> },
}

impl fmt::Debug for Command {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Command::Publish { stream, event } => {
                let mut dbg = fmt.debug_struct("Publish");
                dbg.field("stream", &stream);
                match str::from_utf8(&event) {
                    Ok(event) => dbg.field("event", &event),
                    Err(_) => dbg.field("event", &event),
                };
                dbg.finish()
            },
            Command::Subscribe { streams } => {
                fmt.debug_struct("Subscribe")
                    .field("streams", &streams)
                    .finish()
            }
        }
    }
}

impl Into<RespValue> for Command {
    fn into(self) -> RespValue {
        match self {
            Command::Publish { stream, event } => {
                RespValue::Array(vec![
                    RespValue::bulk_string(&"publish"[..]),
                    RespValue::bulk_string(stream.into_bytes()),
                    RespValue::bulk_string(event),
                ])
            },
            Command::Subscribe { streams } => {
                let streams = streams.into_iter().map(|s| RespValue::bulk_string(s.to_string()));
                let command = RespValue::bulk_string(&"subscribe"[..]);
                let args = Some(command).into_iter().chain(streams).collect();

                RespValue::Array(args)
            }
        }
    }
}

#[derive(Debug)]
pub enum RespCommandConvertError {
    InvalidRespType,
    MissingCommandName,
    UnknownCommand(String),
    InvalidStream(ParseStreamError),
    InvalidNumberOfArguments { expected: usize },
    InvalidUtf8String(str::Utf8Error),
}

impl From<str::Utf8Error> for RespCommandConvertError {
    fn from(error: str::Utf8Error) -> RespCommandConvertError {
        RespCommandConvertError::InvalidUtf8String(error)
    }
}

impl From<string::FromUtf8Error> for RespCommandConvertError {
    fn from(error: string::FromUtf8Error) -> RespCommandConvertError {
        RespCommandConvertError::InvalidUtf8String(error.utf8_error())
    }
}

impl From<ParseStreamError> for RespCommandConvertError {
    fn from(error: ParseStreamError) -> RespCommandConvertError {
        RespCommandConvertError::InvalidStream(error)
    }
}

impl From<StreamNameError> for RespCommandConvertError {
    fn from(error: StreamNameError) -> RespCommandConvertError {
        RespCommandConvertError::InvalidStream(ParseStreamError::StreamNameError(error))
    }
}

impl fmt::Display for RespCommandConvertError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use RespCommandConvertError::*;
        match self {
            InvalidRespType => write!(f, "invalid RESP type, expected array of bulk string"),
            InvalidStream(e) => write!(f, "invalid stream; {}", e),
            UnknownCommand(command) => write!(f, "command {:?} not found", command),
            MissingCommandName => write!(f, "missing command name"),
            InvalidNumberOfArguments { expected } => {
                write!(f, "invalid number of arguments (expected {})", expected)
            },
            InvalidUtf8String(error) => write!(f, "invalid utf8 string: {}", error),
        }
    }
}

impl FromResp for Command {
    type Error = RespCommandConvertError;

    fn from_resp(value: RespValue) -> Result<Self, Self::Error> {
        use RespCommandConvertError::*;

        let mut args = match Vec::<Vec<u8>>::from_resp(value) {
            Ok(args) => args,
            Err(e) => return Err(InvalidRespType),
        };

        let mut args = args.drain(..);

        let command = match args.next() {
            Some(command) => str::from_utf8(&command)?.to_lowercase(),
            None => return Err(MissingCommandName),
        };

        match command.as_str() {
            "publish" => {
                match (args.next(), args.next(), args.next()) {
                    (Some(stream), Some(event), None) => {
                        let text = str::from_utf8(&stream)?;
                        let stream = StreamName::from_str(text)?;
                        Ok(Command::Publish { stream, event })
                    },
                    _ => Err(InvalidNumberOfArguments { expected: 2 })
                }
            },
            "subscribe" => {
                let mut streams = Vec::new();
                for bytes in args {
                    let text = str::from_utf8(&bytes)?;
                    let stream = Stream::from_str(&text)?;
                    streams.push(stream);
                }
                Ok(Command::Subscribe { streams })
            },
            _unknown => Err(UnknownCommand(command)),
        }
    }
}
