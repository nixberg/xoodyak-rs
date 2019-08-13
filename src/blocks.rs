use std::cmp;

pub struct Blocks<'a> {
    v: &'a [u8],
    block_size: usize,
    is_first_block: bool,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        if self.v.is_empty() && !self.is_first_block {
            None
        } else {
            self.is_first_block = false;
            let blocksz = cmp::min(self.v.len(), self.block_size);
            let (fst, snd) = self.v.split_at(blocksz);
            self.v = snd;
            Some(fst)
        }
    }
}

pub trait Blockable {
    fn blocks(&self, block_size: usize) -> Blocks<'_>;
}

impl Blockable for [u8] {
    fn blocks(&self, block_size: usize) -> Blocks<'_> {
        Blocks {
            v: &self,
            block_size,
            is_first_block: true,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
