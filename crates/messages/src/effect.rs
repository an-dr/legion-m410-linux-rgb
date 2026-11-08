use bones_messages::DecodeError;

/// A lighting effect the mouse can run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Static,
    Breathing,
    ColorCycle,
    RainbowWave,
}

impl Effect {
    pub const ALL: [Effect; 4] = [
        Effect::Static,
        Effect::Breathing,
        Effect::ColorCycle,
        Effect::RainbowWave,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Effect::Static => "Static",
            Effect::Breathing => "Breathing",
            Effect::ColorCycle => "Color cycle",
            Effect::RainbowWave => "Rainbow wave",
        }
    }

    /// Whether the effect shows the chosen color (cycle and wave pick their own).
    pub fn uses_color(self) -> bool {
        matches!(self, Effect::Static | Effect::Breathing)
    }

    /// Whether the effect moves, so that speed means something.
    pub fn animated(self) -> bool {
        self != Effect::Static
    }

    pub(crate) fn tag(self) -> u8 {
        match self {
            Effect::Static => 0,
            Effect::Breathing => 1,
            Effect::ColorCycle => 2,
            Effect::RainbowWave => 3,
        }
    }

    pub(crate) fn from_tag(tag: u8) -> Result<Self, DecodeError> {
        match tag {
            0 => Ok(Effect::Static),
            1 => Ok(Effect::Breathing),
            2 => Ok(Effect::ColorCycle),
            3 => Ok(Effect::RainbowWave),
            tag => Err(DecodeError::InvalidTag {
                message: "m410 effect",
                tag,
            }),
        }
    }
}
