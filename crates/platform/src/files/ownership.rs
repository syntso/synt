pub(crate) trait Ownership {
    fn owned_by_current_user(&self) -> bool;

    fn writable_by_others(&self) -> bool;
}
