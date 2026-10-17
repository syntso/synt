use russh::Sig;

pub(crate) trait SignalName {
    fn to_name(&self) -> String;
}

impl SignalName for Sig {
    fn to_name(&self) -> String {
        let name = match self {
            Self::ABRT => "ABRT",
            Self::ALRM => "ALRM",
            Self::FPE => "FPE",
            Self::HUP => "HUP",
            Self::ILL => "ILL",
            Self::INT => "INT",
            Self::KILL => "KILL",
            Self::PIPE => "PIPE",
            Self::QUIT => "QUIT",
            Self::SEGV => "SEGV",
            Self::TERM => "TERM",
            Self::USR1 => "USR1",
            Self::Custom(name) => name,
        };

        name.to_owned()
    }
}
