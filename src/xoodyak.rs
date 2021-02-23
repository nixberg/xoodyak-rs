use crate::xoodoo::Xoodoo;

#[derive(Clone, PartialEq)]
enum Phase {
    Up,
    Down,
}

#[derive(Clone, PartialEq)]
enum Mode {
    Hash,
    Keyed,
}

#[derive(Clone, Copy)]
struct Rate(usize);

impl Rate {
    const HASH: Self = Self(16);
    const KEYED_INPUT: Self = Self(44);
    const KEYED_OUTPUT: Self = Self(24);
    const RATCHET: Self = Self(16);
    const COUNTER: Self = Self(1);
}

#[derive(Clone)]
struct Rates {
    absorb: Rate,
    squeeze: Rate,
}

enum Flag {
    Zero = 0x00,
    AbsorbKey = 0x02,
    Absorb = 0x03,
    Ratchet = 0x10,
    SqueezeKey = 0x20,
    Squeeze = 0x40,
    Crypt = 0x80,
}

#[derive(Clone)]
pub struct Xoodyak {
    phase: Phase,
    state: Xoodoo,
    mode: Mode,
    rates: Rates,
}

impl Xoodyak {
    pub fn new() -> Xoodyak {
        Xoodyak {
            phase: Phase::Up,
            state: Xoodoo::new(),
            mode: Mode::Hash,
            rates: Rates {
                absorb: Rate::HASH,
                squeeze: Rate::HASH,
            },
        }
    }

    pub fn keyed(key: &[u8]) -> Xoodyak {
        Self::keyed_id_counter(key, &[], &[])
    }

    pub fn keyed_id(key: &[u8], id: &[u8]) -> Xoodyak {
        Self::keyed_id_counter(key, id, &[])
    }

    pub fn keyed_counter(key: &[u8], counter: &[u8]) -> Xoodyak {
        Self::keyed_id_counter(key, &[], counter)
    }

    pub fn keyed_id_counter(key: &[u8], id: &[u8], counter: &[u8]) -> Xoodyak {
        assert!(!key.is_empty());
        let mut xoodyak = Xoodyak::new();
        xoodyak.absorb_key(key, id, counter);
        xoodyak
    }

    fn down(&mut self, block: &[u8], flag: Flag) {
        debug_assert!(block.len() <= self.rates.absorb.0);

        self.phase = Phase::Down;

        for (state_byte, block_byte) in self.state.bytes.iter_mut().zip(block.iter()) {
            *state_byte ^= *block_byte;
        }

        self.state.bytes[block.len()] ^= 0x01;
        self.state.bytes[47] ^= if self.mode == Mode::Hash {
            flag as u8 & 0x01
        } else {
            flag as u8
        };
    }

    fn up(&mut self, flag: Flag) {
        self.phase = Phase::Up;
        if self.mode != Mode::Hash {
            self.state.bytes[47] ^= flag as u8;
        }
        self.state.permute();
    }

    fn up_to(&mut self, block: &mut [u8], flag: Flag) {
        self.up(flag);
        for (block_byte, state_byte) in block.iter_mut().zip(self.state.bytes.iter()) {
            *block_byte = *state_byte;
        }
    }

    fn absorb_any(&mut self, data: &[u8], rate: Rate, mut down_flag: Flag) {
        let mut chunks = data.chunks(rate.0);
        let mut block = chunks.next().unwrap_or_default();

        loop {
            if self.phase != Phase::Up {
                self.up(Flag::Zero);
            }
            self.down(block, down_flag);
            down_flag = Flag::Zero;

            if let Some(next_block) = chunks.next() {
                block = next_block;
            } else {
                break;
            }
        }
    }

    fn absorb_key(&mut self, key: &[u8], id: &[u8], counter: &[u8]) {
        self.mode = Mode::Keyed;
        self.rates = Rates {
            absorb: Rate::KEYED_INPUT,
            squeeze: Rate::KEYED_OUTPUT,
        };

        let buffer = [key, id, &[id.len() as u8]].concat();
        assert!(buffer.len() <= Rate::KEYED_INPUT.0);

        self.absorb_any(&buffer, self.rates.absorb, Flag::AbsorbKey);

        if !counter.is_empty() {
            self.absorb_any(counter, Rate::COUNTER, Flag::Zero);
        }
    }

    pub fn crypt(&mut self, input: &[u8], mut output: &mut [u8], decrypt: bool) {
        let mut flag = Flag::Crypt;

        let mut chunks = input.chunks(Rate::KEYED_OUTPUT.0);
        let mut block = chunks.next().unwrap_or_default();

        loop {
            self.up(flag);
            flag = Flag::Zero;

            for (output_byte, (block_byte, state_byte)) in output
                .iter_mut()
                .zip(block.iter().zip(self.state.bytes.iter()))
            {
                *output_byte = *block_byte ^ *state_byte;
            }

            if decrypt {
                self.down(&output[..block.len()], Flag::Zero);
            } else {
                self.down(block, Flag::Zero);
            }

            output = &mut output[block.len()..];

            if let Some(next_block) = chunks.next() {
                block = next_block;
            } else {
                break;
            }
        }
    }

    fn squeeze_any_to(&mut self, buffer: &mut [u8], up_flag: Flag) {
        assert!(!buffer.is_empty());

        let mut chunks = buffer.chunks_mut(self.rates.squeeze.0);

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
        let mut buffer = [0u8; Rate::RATCHET.0];
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

        for kat in kats {
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

        for kat in kats {
            let key = hex::decode(&kat.key).unwrap();
            let nonce = hex::decode(&kat.nonce).unwrap();
            let pt = hex::decode(&kat.pt).unwrap();
            let ad = hex::decode(&kat.ad).unwrap();
            let ct = hex::decode(&kat.ct).unwrap();
            let (ct_only, tag) = ct.split_at(pt.len());

            let mut encryptor = Xoodyak::keyed(&key);
            encryptor.absorb(&nonce);
            encryptor.absorb(&ad);
            let mut decryptor = encryptor.clone();

            let mut new_ct = vec![0; ct.len()];
            let (new_ct_only, new_tag) = new_ct.split_at_mut(pt.len());
            encryptor.encrypt(&pt, new_ct_only);
            encryptor.squeeze_to(new_tag);

            assert_eq!(ct, new_ct);

            let mut new_pt = vec![0; pt.len()];
            decryptor.decrypt(ct_only, &mut new_pt);
            let mut new_tag = vec![0; tag.len()];
            decryptor.squeeze_to(&mut new_tag);

            assert_eq!(pt, new_pt);
            assert_eq!(tag, &new_tag);
        }
    }
}
