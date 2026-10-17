use synt_proto::{ApprovalId, ApprovalRequest};

use crate::access::AccessRequest;

pub(super) fn approval_request(id: ApprovalId, request: &AccessRequest<'_>) -> ApprovalRequest {
    ApprovalRequest {
        id,
        actor: request.actor.clone(),
        on_behalf_of: request.origin.cloned(),
        action: request.action.clone(),
        target: request.target.cloned(),
        destination: request.destination.map(str::to_owned),
        risk: request.risk,
        params: request.params.clone(),
    }
}
