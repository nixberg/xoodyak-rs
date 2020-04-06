use std::cmp::min;

pub trait Blockable {
    fn blocks(&self, block_size: usize) -> Blocks<'_>;
}

impl Blockable for [u8] {
    fn blocks(&self, rate: usize) -> Blocks<'_> {
        Blocks {
            tail: &self,
            rate,
            is_first_block: true,
        }
    }
}

pub struct Blocks<'a> {
    tail: &'a [u8],
    rate: usize,
    is_first_block: bool,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.tail.is_empty() && !self.is_first_block {
            return None;
        }
        self.is_first_block = false;
        let (block, tail) = self.tail.split_at(min(self.tail.len(), self.rate));
        self.tail = tail;
        Some(block)
    }
}
