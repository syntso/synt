use russh::keys::Algorithm;

pub(crate) trait Family {
    fn same_family(&self, other: &Algorithm) -> bool;
}

impl Family for Algorithm {
    fn same_family(&self, other: &Algorithm) -> bool {
        let both_rsa = matches!((self, other), (Self::Rsa { .. }, Self::Rsa { .. }));

        self == other || both_rsa
    }
}
