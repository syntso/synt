#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum RiskLevel {
    Read,
    Write,
    Privileged,
}

impl From<synt_proto::RiskLevel> for RiskLevel {
    fn from(risk: synt_proto::RiskLevel) -> Self {
        match risk {
            synt_proto::RiskLevel::Read => Self::Read,
            synt_proto::RiskLevel::Write => Self::Write,
            synt_proto::RiskLevel::Privileged => Self::Privileged,
        }
    }
}
