//! Stable preference IDs; adding a family does not change saved sessions.
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontChoice {
    Hack,
    Consolas,
    CourierNew,
    CascadiaMono,
    #[default]
    #[serde(rename = "jetbrains_mono")]
    #[serde(other)]
    JetBrainsMono,
}

impl FontChoice {
    pub const ALL: [Self; 5] = [
        Self::JetBrainsMono,
        Self::CascadiaMono,
        Self::Hack,
        Self::Consolas,
        Self::CourierNew,
    ];

    pub fn family(self) -> &'static str {
        match self {
            Self::CascadiaMono => "Cascadia Mono",
            Self::Hack => "Hack",
            Self::Consolas => "Consolas",
            Self::CourierNew => "Courier New",
            Self::JetBrainsMono => "JetBrains Mono",
        }
    }

    pub(crate) fn system_faces(self) -> Option<[&'static str; 4]> {
        match self {
            Self::CascadiaMono | Self::Hack | Self::JetBrainsMono => None,
            Self::Consolas => Some([
                "consola.ttf",
                "consolab.ttf",
                "consolai.ttf",
                "consolaz.ttf",
            ]),
            Self::CourierNew => Some(["cour.ttf", "courbd.ttf", "couri.ttf", "courbi.ttf"]),
        }
    }
}
