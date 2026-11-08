use bones_messages::{DecodeError, DecodeMessage, EncodeMessage, Message, Reader, Writer};

use crate::Lighting;

const TAG_REFRESH: u8 = 0;
const TAG_APPLY: u8 = 1;

/// A request to the native module, published on `m410/command`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Read the lighting from the mouse and publish a [`crate::State`], even if
    /// nothing changed. A newly started extension asks this to learn the state.
    Refresh,
    /// Store these settings on the mouse. Held until the mouse answers, so a
    /// sleeping mouse gets them when it wakes.
    Apply(Lighting),
}

impl Message for Command {
    const TOPIC: &'static str = "m410/command";
}

impl EncodeMessage for Command {
    fn encode(&self) -> Vec<u8> {
        match self {
            Command::Refresh => Writer::new().u8(TAG_REFRESH).finish(),
            Command::Apply(lighting) => lighting.write(Writer::new().u8(TAG_APPLY)).finish(),
        }
    }
}

impl<'a> DecodeMessage<'a> for Command {
    fn decode(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(payload);
        let command = match reader.read_u8()? {
            TAG_REFRESH => Command::Refresh,
            TAG_APPLY => Command::Apply(Lighting::read(&mut reader)?),
            tag => {
                return Err(DecodeError::InvalidTag {
                    message: "m410 command",
                    tag,
                })
            }
        };
        reader.finish()?;
        Ok(command)
    }
}
