use std::hash::Hasher;

#[derive(Default)]
pub struct UlidHasher(u64);

impl Hasher for UlidHasher {
    fn write(&mut self, _: &[u8]) {
        unimplemented!()
    }

    fn write_u128(&mut self, i: u128) {
        self.0 = unsafe { std::mem::transmute::<u128, [u64; 2]>(i)[1] };
    }

    fn finish(&self) -> u64 {
        self.0
    }
}
