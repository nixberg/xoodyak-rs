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

    pub fn squeeze_to(&mut self, buffer: &mut [u8]) {
        self.squeeze_any_to(buffer, Flag::Squeeze);
    }
}

#[derive(Clone)]
pub struct KeyedXoodyak(Xoodyak);

impl KeyedXoodyak {
    pub fn new(key: &[u8]) -> Self {
        Self::new_id_counter(key, &[], &[])
    }

    pub fn new_id(key: &[u8], id: &[u8]) -> Self {
        Self::new_id_counter(key, id, &[])
    }

    pub fn new_counter(key: &[u8], counter: &[u8]) -> Self {
        Self::new_id_counter(key, &[], counter)
    }

    pub fn new_id_counter(key: &[u8], id: &[u8], counter: &[u8]) -> Self {
        assert!(!key.is_empty());
        let mut keyed_xoodyak = KeyedXoodyak(Xoodyak::new());
        keyed_xoodyak.absorb_key(key, id, counter);
        keyed_xoodyak
    }

    fn absorb_key(&mut self, key: &[u8], id: &[u8], counter: &[u8]) {
        self.0.mode = Mode::Keyed;
        self.0.rates = Rates {
            absorb: Rate::KEYED_INPUT,
            squeeze: Rate::KEYED_OUTPUT,
        };

        let buffer = [key, id, &[id.len() as u8]].concat();
        assert!(buffer.len() <= Rate::KEYED_INPUT.0);

        self.0
            .absorb_any(&buffer, self.0.rates.absorb, Flag::AbsorbKey);

        if !counter.is_empty() {
            self.0.absorb_any(counter, Rate::COUNTER, Flag::Zero);
        }
    }

    pub fn crypt(&mut self, input: &[u8], mut output: &mut [u8], decrypt: bool) {
        let mut flag = Flag::Crypt;

        let mut chunks = input.chunks(Rate::KEYED_OUTPUT.0);
        let mut block = chunks.next().unwrap_or_default();

        loop {
            self.0.up(flag);
            flag = Flag::Zero;

            for (output_byte, (block_byte, state_byte)) in output
                .iter_mut()
                .zip(block.iter().zip(self.0.state.bytes.iter()))
            {
                *output_byte = *block_byte ^ *state_byte;
            }

            if decrypt {
                self.0.down(&output[..block.len()], Flag::Zero);
            } else {
                self.0.down(block, Flag::Zero);
            }

            output = &mut output[block.len()..];

            if let Some(next_block) = chunks.next() {
                block = next_block;
            } else {
                break;
            }
        }
    }

    #[inline]
    pub fn absorb(&mut self, input: &[u8]) {
        self.0.absorb(input);
    }

    pub fn encrypt(&mut self, plaintext: &[u8], ciphertext: &mut [u8]) {
        self.crypt(plaintext, ciphertext, false);
    }

    pub fn decrypt(&mut self, ciphertext: &[u8], plaintext: &mut [u8]) {
        self.crypt(ciphertext, plaintext, true);
    }

    #[inline]
    pub fn squeeze_to(&mut self, buffer: &mut [u8]) {
        self.0.squeeze_to(buffer);
    }

    pub fn squeeze_key_to(&mut self, buffer: &mut [u8]) {
        self.0.squeeze_any_to(buffer, Flag::SqueezeKey);
    }

    pub fn ratchet(&mut self) {
        let mut buffer = [0u8; Rate::RATCHET.0];
        self.0.squeeze_any_to(&mut buffer, Flag::Ratchet);
        self.0.absorb_any(&buffer, self.0.rates.absorb, Flag::Zero);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hash_mode() {
        use super::Xoodyak;

        #[derive(serde::Deserialize)]
        struct KAT {
            msg: String,
            md: String,
        }

        let kat_bytes = include_bytes!("../test/hash.json");
        let kats: Vec<KAT> = serde_json::from_slice(kat_bytes).unwrap();

        for kat in kats {
            let msg = hex::decode(&kat.msg).unwrap();
            let md = hex::decode(&kat.md).unwrap();

            let mut xoodyak = Xoodyak::new();
            xoodyak.absorb(&msg);
            let mut new_md = vec![0; md.len()];
            xoodyak.squeeze_to(&mut new_md);

            assert_eq!(new_md, md);
        }
    }

    #[test]
    fn aead_mode() {
        use super::KeyedXoodyak;

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

            let mut encryptor = KeyedXoodyak::new(&key);
            encryptor.absorb(&nonce);
            encryptor.absorb(&ad);
            let mut decryptor = encryptor.clone();

            let mut new_ct = vec![0; ct.len()];
            let (new_ct_only, new_tag) = new_ct.split_at_mut(pt.len());
            encryptor.encrypt(&pt, new_ct_only);
            encryptor.squeeze_to(new_tag);

            assert_eq!(new_ct, ct);

            let mut new_pt = vec![0; pt.len()];
            decryptor.decrypt(ct_only, &mut new_pt);
            let mut new_tag = vec![0; tag.len()];
            decryptor.squeeze_to(&mut new_tag);

            assert_eq!(new_pt, pt);
            assert_eq!(&new_tag, tag);
        }
    }
}
