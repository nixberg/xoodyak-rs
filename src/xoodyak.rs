use crate::blocks::Blockable;
use crate::xoodoo::Xoodoo;

enum Flag {
    Zero = 0x00,
    AbsorbKey = 0x02,
    Absorb = 0x03,
    Ratchet = 0x10,
    SqueezeKey = 0x20,
    Squeeze = 0x40,
    Crypt = 0x80,
}

#[derive(Clone, PartialEq)]
enum Mode {
    Hash,
    Keyed,
}

#[derive(Clone)]
struct Rates {
    absorb: usize,
    squeeze: usize,
}

impl Rates {
    const HASH: usize = 16;
    const INPUT: usize = 44;
    const OUTPUT: usize = 24;
    const RATCHET: usize = 16;
}

#[derive(Clone, PartialEq)]
enum Phase {
    Up,
    Down,
}

#[derive(Clone)]
pub struct Xoodyak {
    mode: Mode,
    rates: Rates,
    phase: Phase,
    xoodoo: Xoodoo,
}

impl Xoodyak {
    pub fn new() -> Xoodyak {
        Xoodyak {
            mode: Mode::Hash,
            rates: Rates {
                absorb: Rates::HASH,
                squeeze: Rates::HASH,
            },
            phase: Phase::Up,
            xoodoo: Xoodoo::new(),
        }
    }

    pub fn keyed(key: &[u8], id: &[u8], counter: &[u8]) -> Xoodyak {
        assert!(key.len() + id.len() <= Rates::INPUT);

        let mut xoodyak = Xoodyak {
            mode: Mode::Keyed,
            rates: Rates {
                absorb: Rates::INPUT,
                squeeze: Rates::OUTPUT,
            },
            phase: Phase::Up,
            xoodoo: Xoodoo::new(),
        };

        let bytes = [key, id, &[id.len() as u8]].concat();
        xoodyak.absorb_any(&bytes, xoodyak.rates.absorb, Flag::AbsorbKey);

        if !counter.is_empty() {
            xoodyak.absorb_any(counter, 1, Flag::Zero);
        }

        xoodyak
    }

    fn down(&mut self, block: &[u8], flag: Flag) {
        debug_assert!(block.len() <= self.rates.absorb);
        self.phase = Phase::Down;
        for (state_byte, block_byte) in self.xoodoo.state.iter_mut().zip(block.iter()) {
            *state_byte ^= *block_byte;
        }
        self.xoodoo.state[block.len()] ^= 0x01;
        self.xoodoo.state[47] ^= if self.mode == Mode::Hash {
            flag as u8 & 0x01
        } else {
            flag as u8
        };
    }

    fn up(&mut self, flag: Flag) {
        self.phase = Phase::Up;
        if self.mode != Mode::Hash {
            self.xoodoo.state[47] ^= flag as u8;
        }
        self.xoodoo.permute();
    }

    fn up_to(&mut self, block: &mut [u8], flag: Flag) {
        self.up(flag);
        for (block_byte, state_byte) in block.iter_mut().zip(self.xoodoo.state.iter()) {
            *block_byte = *state_byte;
        }
    }

    fn absorb_any(&mut self, data: &[u8], rate: usize, down_flag: Flag) {
        let mut down_flag = down_flag;
        for block in data.blocks(rate) {
            if self.phase != Phase::Up {
                self.up(Flag::Zero);
            }
            self.down(block, down_flag);
            down_flag = Flag::Zero;
        }
    }

    pub fn crypt(&mut self, input: &[u8], output: &mut [u8], decrypt: bool) {
        let mut flag = Flag::Crypt;
        let mut output = output;
        for block in input.blocks(Rates::OUTPUT) {
            self.up(flag);
            flag = Flag::Zero;
            for (output_byte, (block_byte, state_byte)) in output
                .iter_mut()
                .zip(block.iter().zip(self.xoodoo.state.iter()))
            {
                *output_byte = *block_byte ^ *state_byte;
            }
            if decrypt {
                self.down(&output[..block.len()], Flag::Zero);
            } else {
                self.down(block, Flag::Zero);
            }
            output = &mut output[block.len()..];
        }
    }

    fn squeeze_any_to(&mut self, buffer: &mut [u8], up_flag: Flag) {
        assert!(!buffer.is_empty());
        let mut chunks = buffer.chunks_mut(self.rates.squeeze);
        self.up_to(chunks.next().unwrap(), up_flag);
        for chunk in chunks {
            self.down(&[], Flag::Zero);
            self.up_to(chunk, Flag::Zero);
        }
    }

    pub fn absorb(&mut self, data: &[u8]) {
        self.absorb_any(data, self.rates.absorb, Flag::Absorb);
    }

    pub fn encrypt(&mut self, plaintext: &[u8], ciphertext: &mut [u8]) {
        assert!(self.mode == Mode::Keyed);
        self.crypt(plaintext, ciphertext, false);
    }

    pub fn decrypt(&mut self, ciphertext: &[u8], plaintext: &mut [u8]) {
        assert!(self.mode == Mode::Keyed);
        self.crypt(ciphertext, plaintext, true);
    }

    pub fn squeeze_to(&mut self, buffer: &mut [u8]) {
        self.squeeze_any_to(buffer, Flag::Squeeze);
    }

    pub fn squeeze_key_to(&mut self, buffer: &mut [u8]) {
        assert!(self.mode == Mode::Keyed);
        self.squeeze_any_to(buffer, Flag::SqueezeKey);
    }

    pub fn ratchet(&mut self) {
        assert!(self.mode == Mode::Keyed);
        let mut buffer = [0u8; Rates::RATCHET];
        self.squeeze_any_to(&mut buffer, Flag::Ratchet);
        self.absorb_any(&buffer, self.rates.absorb, Flag::Zero);
    }
}

impl Default for Xoodyak {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Xoodyak;

    #[test]
    fn hash_mode() {
        #[derive(serde::Deserialize)]
        struct KAT {
            msg: String,
            md: String,
        }

        let kat_bytes = include_bytes!("../test/hash.json");
        let kats: Vec<KAT> = serde_json::from_slice(kat_bytes).unwrap();

        for kat in kats.iter() {
            let msg_bytes = hex::decode(&kat.msg).unwrap();
            let md_bytes = hex::decode(&kat.md).unwrap();

            let mut xoodyak = Xoodyak::new();
            xoodyak.absorb(&msg_bytes);
            let mut new_md_bytes = vec![0; md_bytes.len()];
            xoodyak.squeeze_to(&mut new_md_bytes);

            assert_eq!(md_bytes, new_md_bytes);
        }
    }

    #[test]
    fn aead_mode() {
        #[derive(serde::Deserialize)]
        struct KAT {
            key: String,
            nonce: String,
            pt: String,
            ad: String,
            ct: String,
        }

        let kat_bytes = include_bytes!("../test/aead.json");
        let kats: Vec<KAT> = serde_json::from_slice(kat_bytes).unwrap();

        for kat in kats.iter() {
            let key = hex::decode(&kat.key).unwrap();
            let nonce = hex::decode(&kat.nonce).unwrap();
            let pt = hex::decode(&kat.pt).unwrap();
            let ad = hex::decode(&kat.ad).unwrap();
            let ct = hex::decode(&kat.ct).unwrap();
            let (ct_only, tag) = ct.split_at(pt.len());

            let mut xoodyak = Xoodyak::keyed(&key, &[], &[]);
            xoodyak.absorb(&nonce);
            xoodyak.absorb(&ad);
            let mut new_ct = vec![0; ct.len()];
            let (new_ct_only, new_tag) = new_ct.split_at_mut(pt.len());
            xoodyak.encrypt(&pt, new_ct_only);
            xoodyak.squeeze_to(new_tag);

            assert_eq!(ct, new_ct);

            xoodyak = Xoodyak::keyed(&key, &[], &[]);
            xoodyak.absorb(&nonce);
            xoodyak.absorb(&ad);
            let mut new_pt = vec![0; pt.len()];
            xoodyak.decrypt(ct_only, &mut new_pt);
            let mut new_tag = vec![0; tag.len()];
            xoodyak.squeeze_to(&mut new_tag);

            assert_eq!(pt, new_pt);
            assert_eq!(tag, new_tag.as_slice());
        }
    }
}
