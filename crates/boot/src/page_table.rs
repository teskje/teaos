pub struct PageTable {}

impl PageTable {
    pub fn new() -> Self {
        Self {}
    }

    pub fn map(&self, _va: usize, _pa: usize, _size: usize) {
        // TODO
    }
}
