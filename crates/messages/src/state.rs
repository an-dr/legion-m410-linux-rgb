use bones_messages::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

use crate::Lighting;

/// How the mouse is connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connection {
    /// Neither the cable nor the receiver is plugged in.
    None,
    Wired,
    Wireless,
}

/// How the last attempt to talk to the mouse went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// The mouse answered; `lighting` is what it holds.
    Ready,
    /// The mouse did not answer, usually because it is asleep. A pending change
    /// is applied as soon as it wakes.
    Waiting,
    /// No M410 is connected.
    NotFound,
    /// The mouse is there but this user may not open it (the udev rule is missing).
    NoAccess,
    Failed,
}

/// What the native module knows, published on `m410/state` after every change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State<'a> {
    pub connection: Connection,
    pub status: Status,
    /// The lighting last read from or written to the mouse, if any.
    pub lighting: Option<Lighting>,
    /// A sentence for the user about what went wrong or what to do; empty when fine.
    pub detail: &'a str,
}

impl Message for State<'_> {
    const TOPIC: &'static str = "m410/state";
}

impl Connection {
    fn tag(self) -> u8 {
        match self {
            Connection::None => 0,
            Connection::Wired => 1,
            Connection::Wireless => 2,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, DecodeError> {
        match tag {
            0 => Ok(Connection::None),
            1 => Ok(Connection::Wired),
            2 => Ok(Connection::Wireless),
            tag => Err(DecodeError::InvalidTag {
                message: "m410 connection",
                tag,
            }),
        }
    }
}

impl Status {
    fn tag(self) -> u8 {
        match self {
            Status::Ready => 0,
            Status::Waiting => 1,
            Status::NotFound => 2,
            Status::NoAccess => 3,
            Status::Failed => 4,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, DecodeError> {
        match tag {
            0 => Ok(Status::Ready),
            1 => Ok(Status::Waiting),
            2 => Ok(Status::NotFound),
            3 => Ok(Status::NoAccess),
            4 => Ok(Status::Failed),
            tag => Err(DecodeError::InvalidTag {
                message: "m410 status",
                tag,
            }),
        }
    }
}

impl EncodeMessage for State<'_> {
    fn encode(&self) -> Vec<u8> {
        let writer = Writer::new().u8(self.connection.tag()).u8(self.status.tag());
        let writer = match &self.lighting {
            Some(lighting) => lighting.write(writer.u8(1)),
            None => writer.u8(0),
        };
        // The detail runs to the end of the payload, so it needs no length prefix.
        writer.bytes(self.detail.as_bytes()).finish()
    }
}

impl<'a> DecodeMessage<'a> for State<'a> {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let connection = Connection::from_tag(reader.read_u8()?)?;
        let status = Status::from_tag(reader.read_u8()?)?;
        let lighting = match reader.read_u8()? {
            0 => None,
            _ => Some(Lighting::read(&mut reader)?),
        };
        Ok(Self {
            connection,
            status,
            lighting,
            detail: reader.read_str_rest()?,
        })
    }
}
