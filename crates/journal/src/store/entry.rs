use crate::chain::{Fields, Link};

pub(crate) struct Entry<'a> {
    pub(crate) fields: Fields<'a>,
    pub(crate) link: Link,
}
