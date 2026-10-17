#[derive(Debug, Default)]
pub(super) struct Holders {
    count: usize,
    releases: u64,
}

impl Holders {
    pub(super) fn count(&self) -> usize {
        self.count
    }

    pub(super) fn join(&mut self) {
        self.count += 1;
    }

    pub(super) fn leave(&mut self) -> Option<u64> {
        self.count = self.count.saturating_sub(1);

        if self.count > 0 {
            return None;
        }

        self.releases += 1;
        Some(self.releases)
    }

    pub(super) fn idle_since(&self, release: u64) -> bool {
        self.count == 0 && self.releases == release
    }
}
