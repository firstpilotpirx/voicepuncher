//! What one option of the configuration file looks like in the settings window.

/// The window's tabs, in the order of its toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    General,
    Dictation,
    Refinement,
    Sounds,
    Paste,
    Archive,
    Logging,
    Services,
}

impl Tab {
    pub const ALL: [Tab; 8] = [
        Tab::General,
        Tab::Dictation,
        Tab::Refinement,
        Tab::Sounds,
        Tab::Paste,
        Tab::Archive,
        Tab::Logging,
        Tab::Services,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Tab::General => "General",
            Tab::Dictation => "Dictation",
            Tab::Refinement => "Refinement",
            Tab::Sounds => "Sounds",
            Tab::Paste => "Paste",
            Tab::Archive => "Archive",
            Tab::Logging => "Logging",
            Tab::Services => "Services",
        }
    }

    /// The SF Symbol of the tab's toolbar item.
    pub fn symbol(self) -> &'static str {
        match self {
            Tab::General => "gearshape",
            Tab::Dictation => "waveform",
            Tab::Refinement => "sparkles",
            Tab::Sounds => "speaker.wave.2",
            Tab::Paste => "doc.on.clipboard",
            Tab::Archive => "archivebox",
            Tab::Logging => "doc.text",
            Tab::Services => "server.rack",
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
    Examples,
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
    pub const fn advanced(self) -> Field {
        Field { advanced: true, ..self }
    }
}
