use tapestry_weave::v1::dependent::TapestryWeave;

use crate::AppShared;

pub struct InferenceEngine {}

impl InferenceEngine {
    pub fn new(shared: &mut AppShared) -> Self {
        Self {}
    }
    pub fn requests(&self) -> usize {
        0 // TODO
    }
    pub fn update(&mut self, shared: &mut AppShared, weave: &mut Option<TapestryWeave>) {}
    pub fn cancel(&mut self, shared: &mut AppShared) {}
}
