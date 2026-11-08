use bones_messages::{DecodeError, Reader, Writer};

use crate::Effect;

pub const BRIGHTNESS_MAX: u8 = 4;
pub const SPEED_MIN: u8 = 1;
pub const SPEED_MAX: u8 = 5;

/// Everything the mouse remembers about its lighting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lighting {
    pub effect: Effect,
    /// 0 (off) to [`BRIGHTNESS_MAX`].
    pub brightness: u8,
    /// [`SPEED_MIN`] (slowest) to [`SPEED_MAX`] (fastest).
    pub speed: u8,
    /// A new random color each cycle instead of `color`.
    pub random: bool,
    pub color: (u8, u8, u8),
}

impl Lighting {
    pub(crate) fn write(&self, writer: Writer) -> Writer {
        writer
            .u8(self.effect.tag())
            .u8(self.brightness)
            .u8(self.speed)
            .u8(u8::from(self.random))
            .u8(self.color.0)
            .u8(self.color.1)
            .u8(self.color.2)
    }

    pub(crate) fn read(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            effect: Effect::from_tag(reader.read_u8()?)?,
            brightness: reader.read_u8()?.min(BRIGHTNESS_MAX),
            speed: reader.read_u8()?.clamp(SPEED_MIN, SPEED_MAX),
            random: reader.read_u8()? != 0,
            color: (reader.read_u8()?, reader.read_u8()?, reader.read_u8()?),
        })
    }
}
