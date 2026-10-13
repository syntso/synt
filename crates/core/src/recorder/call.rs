use synt_journal::{ActionId, Actor, ResourceRef};

pub(crate) struct Call<'a> {
    pub(crate) actor: &'a Actor,
    pub(crate) action: &'a ActionId,
    pub(crate) target: Option<&'a ResourceRef>,
}
