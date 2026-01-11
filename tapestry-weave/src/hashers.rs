use std::hash::Hasher;

pub use rustc_hash::FxBuildHasher;

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

#[derive(Default)]
pub struct RandomIdHasher(u64);

impl Hasher for RandomIdHasher {
    fn write(&mut self, _: &[u8]) {
        unimplemented!()
    }

    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}
