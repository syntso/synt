#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    Hello,
    ActionCall,
    ActionsDescribe,
    ApprovalRespond,
}

impl Method {
    pub const ALL: [Self; 4] = [
        Self::Hello,
        Self::ActionCall,
        Self::ActionsDescribe,
        Self::ApprovalRespond,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Hello => "hello",
            Self::ActionCall => "action.call",
            Self::ActionsDescribe => "actions.describe",
            Self::ApprovalRespond => "approval.respond",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|method| method.name() == name)
    }
}

#[cfg(test)]
mod tests {
    use super::Method;

    #[test]
    fn names_round_trip() {
        for method in Method::ALL {
            assert_eq!(Method::parse(method.name()), Some(method));
        }
    }

    #[test]
    fn unknown_names_are_none() {
        assert_eq!(Method::parse("shutdown"), None);
    }
}
