use synt_journal::Name;
use synt_proto::ActionCall;

#[derive(Debug, Clone, PartialEq)]
pub struct ExtensionCall {
    extension: Name,
    request: ActionCall,
}

impl ExtensionCall {
    pub fn new(extension: Name, request: ActionCall) -> Self {
        Self { extension, request }
    }

    pub fn extension(&self) -> &Name {
        &self.extension
    }

    pub fn request(&self) -> &ActionCall {
        &self.request
    }

    pub fn into_request(self) -> ActionCall {
        self.request
    }
}
