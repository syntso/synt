#[derive(Debug, Default)]
pub(crate) struct Holders {
    count: usize,
    releases: u64,
}

impl Holders {
    pub(crate) fn count(&self) -> usize {
        self.count
    }

    pub(crate) fn join(&mut self) {
        self.count += 1;
    }

    pub(crate) fn leave(&mut self) -> Option<u64> {
        self.count = self.count.saturating_sub(1);

        if self.count > 0 {
            return None;
        }

        self.releases += 1;
        Some(self.releases)
    }

    pub(crate) fn idle_since(&self, release: u64) -> bool {
        self.count == 0 && self.releases == release
    }
}
