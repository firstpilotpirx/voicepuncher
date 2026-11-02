//! What one option of the configuration file looks like in the settings window.

/// The window's tabs, in the order of its toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    General,
    Dictation,
    Sounds,
    Paste,
    Archive,
    Logging,
    Services,
}

impl Tab {
    pub const ALL: [Self; 7] = [
        Self::General,
        Self::Dictation,
        Self::Sounds,
        Self::Paste,
        Self::Archive,
        Self::Logging,
        Self::Services,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Dictation => "Dictation",
            Self::Sounds => "Sounds",
            Self::Paste => "Paste",
            Self::Archive => "Archive",
            Self::Logging => "Logging",
            Self::Services => "Services",
        }
    }

    /// The SF Symbol of the tab's toolbar item.
    pub fn symbol(self) -> &'static str {
        match self {
            Self::General => "gearshape",
            Self::Dictation => "waveform",
            Self::Sounds => "speaker.wave.2",
            Self::Paste => "doc.on.clipboard",
            Self::Archive => "archivebox",
            Self::Logging => "doc.text",
            Self::Services => "server.rack",
        }
    }
}

/// The lists a pop-up or a pair of radio buttons offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choices {
    Hotkeys,
    RecordingModes,
    Microphones,
    Languages,
    Sounds,
    LogLevels,
}

/// Which service protocol an address field takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Http,
    WebSocket,
}

/// The control of a field and the values it accepts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Control {
    Switch,
    PopUp(Choices),
    Radio(Choices),
    Number { min: i64, max: i64, unit: &'static str },
    Slider { min: f64, max: f64 },
    Text,
    Address(Scheme),
    TextArea,
    LanguageList,
    Folder,
}

/// One option of the configuration file on its tab.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Field {
    pub section: &'static str,
    pub key: &'static str,
    pub tab: Tab,
    pub label: &'static str,
    pub control: Control,
    /// Shown only under the tab's `Show advanced` disclosure.
    pub advanced: bool,
}

impl Field {
    /// `section.key`, as the configuration file and the error messages name the option.
    pub fn path(&self) -> String {
        format!("{}.{}", self.section, self.key)
    }
}

/// A field shown on its tab without the advanced disclosure.
pub const fn field(
    section: &'static str,
    key: &'static str,
    tab: Tab,
    label: &'static str,
    control: Control,
) -> Field {
    Field { section, key, tab, label, control, advanced: false }
}

impl Field {
    /// The same field under the tab's `Show advanced` disclosure.
    pub const fn advanced(self) -> Self {
        Self { advanced: true, ..self }
    }
}
