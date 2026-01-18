//! BDIR kindCode importance semantics per RFC-0001.

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindImportance {
    Core,
    Boilerplate,
    UiChrome,
    Unknown,
}

impl KindImportance {
    pub const fn as_str(self) -> &'static str {
        match self {
            KindImportance::Core => "core",
            KindImportance::Boilerplate => "boilerplate",
            KindImportance::UiChrome => "ui",
            KindImportance::Unknown => "unknown",
        }
    }
}

impl fmt::Display for KindImportance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).as_str())
    }
}

pub mod ranges {
    pub const CORE_START: u16 = 0;
    pub const CORE_END: u16 = 19;
    pub const BOILERPLATE_START: u16 = 20;
    pub const BOILERPLATE_END: u16 = 39;
    pub const UI_CHROME_START: u16 = 40;
    pub const UI_CHROME_END: u16 = 59;
    pub const UNKNOWN: u16 = 99;
}

pub fn importance(kind_code: u16) -> KindImportance {
    use ranges::*;
    match kind_code {
        CORE_START..=CORE_END => KindImportance::Core,
        BOILERPLATE_START..=BOILERPLATE_END => KindImportance::Boilerplate,
        UI_CHROME_START..=UI_CHROME_END => KindImportance::UiChrome,
        _ => KindImportance::Unknown,
    }
}

pub fn description(kind_code: u16) -> &'static str {
    match importance(kind_code) {
        KindImportance::Core => "Primary content relevant for AI and indexing",
        KindImportance::Boilerplate => "Navigation, repeated site boilerplate",
        KindImportance::UiChrome => "Pure UI or decorative chrome",
        KindImportance::Unknown => "Unclassified or out-of-range kindCode",
    }
}

pub fn is_core(kind_code: u16) -> bool {
    matches!(importance(kind_code), KindImportance::Core)
}

pub fn is_boilerplate(kind_code: u16) -> bool {
    matches!(importance(kind_code), KindImportance::Boilerplate)
}

pub fn is_ui_chrome(kind_code: u16) -> bool {
    matches!(importance(kind_code), KindImportance::UiChrome)
}

pub fn is_unknown(kind_code: u16) -> bool {
    matches!(importance(kind_code), KindImportance::Unknown)
}
