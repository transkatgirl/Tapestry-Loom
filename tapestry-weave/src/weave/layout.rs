//! [`TapestryWeave`] rendering helpers.

use std::hash::BuildHasherDefault;

use universal_weave::{
    LayoutItem, Layouter,
    glam::Vec2,
    layout::{IndependentLayouter, Spacing},
    tinyvec::ArrayVec,
};

use crate::{
    content::NodeContent,
    util::RandomIdHasher,
    weave::{
        ShortId, TapestryNode, TapestryWeave, TapestryWeaveInner, wrappers::LoggedTapestryWeave,
    },
};

/// A geometric item within a [`TapestryLayouter`]'s arrangement.
pub type TapestryLayoutItem = LayoutItem<ShortId, Vec2, ArrayVec<[Vec2; 6]>>;

/// An [`IndependentLayouter`] wrapper which arranges a [`TapestryWeave`]'s content for graphical rendering.
#[derive(Default, Debug, Clone)]
#[must_use]
pub struct TapestryLayouter(pub IndependentLayouter<ShortId, BuildHasherDefault<RandomIdHasher>>);

impl From<IndependentLayouter<ShortId, BuildHasherDefault<RandomIdHasher>>> for TapestryLayouter {
    #[inline]
    fn from(value: IndependentLayouter<ShortId, BuildHasherDefault<RandomIdHasher>>) -> Self {
        Self(value)
    }
}

impl From<TapestryLayouter> for IndependentLayouter<ShortId, BuildHasherDefault<RandomIdHasher>> {
    #[inline]
    fn from(value: TapestryLayouter) -> Self {
        value.0
    }
}

impl AsRef<IndependentLayouter<ShortId, BuildHasherDefault<RandomIdHasher>>> for TapestryLayouter {
    #[inline]
    fn as_ref(&self) -> &IndependentLayouter<ShortId, BuildHasherDefault<RandomIdHasher>> {
        &self.0
    }
}

impl TapestryLayouter {
    /// Creates a new [`TapestryLayouter`] with the specified spacing.
    #[inline]
    pub fn new(spacing: Spacing) -> Self {
        Self(IndependentLayouter::new(spacing))
    }
    /// Converts a [`TapestryLayouter`] into the underlying [`IndependentLayouter`].
    #[inline]
    pub fn into_inner(self) -> IndependentLayouter<ShortId, BuildHasherDefault<RandomIdHasher>> {
        self.0
    }
    /// Returns a reference to the underlying [`IndependentLayouter`].
    #[inline]
    pub const fn as_inner(
        &self,
    ) -> &IndependentLayouter<ShortId, BuildHasherDefault<RandomIdHasher>> {
        &self.0
    }
    /// Returns a reference to the [`Spacing`] used to arrange contents.
    #[inline]
    pub const fn spacing(&self) -> &Spacing {
        &self.0.spacing
    }
    /// Returns a mutable reference to the [`Spacing`] used to arrange contents.
    #[inline]
    pub const fn spacing_mut(&mut self) -> &mut Spacing {
        &mut self.0.spacing
    }
    /// Returns the size of the bounding box enclosing the arrangement's content.
    #[inline]
    pub fn size(&self) -> Vec2 {
        pinned(&self.0).size()
    }
    /// Returns the center position of the node corresponding to the specified identifier.
    #[inline]
    pub fn center(&self, id: &ShortId) -> Option<Vec2> {
        pinned(&self.0).center(id)
    }
    /// Returns [`TapestryLayoutItem`]s within the specified bounds in the order that they should be rendered.
    #[inline]
    pub fn view(&mut self, min: Vec2, max: Vec2, callback: impl FnMut(TapestryLayoutItem)) {
        pinned_mut(&mut self.0).view(min, max, callback);
    }
}

impl Layouter<TapestryWeave, ShortId, TapestryNode, NodeContent, Vec2, ArrayVec<[Vec2; 6]>>
    for TapestryLayouter
{
    #[inline]
    fn layout(&mut self, weave: &mut TapestryWeave, sizes: impl FnMut(&TapestryNode) -> Vec2) {
        self.0.layout(&mut weave.0.weave, sizes);
    }
    #[inline]
    fn size(&self) -> Vec2 {
        pinned(&self.0).size()
    }
    #[inline]
    fn center(&self, id: &ShortId) -> Option<Vec2> {
        pinned(&self.0).center(id)
    }
    #[inline]
    fn view(&mut self, min: Vec2, max: Vec2, callback: impl FnMut(TapestryLayoutItem)) {
        pinned_mut(&mut self.0).view(min, max, callback);
    }
}

impl Layouter<LoggedTapestryWeave, ShortId, TapestryNode, NodeContent, Vec2, ArrayVec<[Vec2; 6]>>
    for TapestryLayouter
{
    #[inline]
    fn layout(
        &mut self,
        weave: &mut LoggedTapestryWeave,
        sizes: impl FnMut(&TapestryNode) -> Vec2,
    ) {
        self.0.layout(&mut weave.weave.0.weave, sizes);
    }
    #[inline]
    fn size(&self) -> Vec2 {
        pinned(&self.0).size()
    }
    #[inline]
    fn center(&self, id: &ShortId) -> Option<Vec2> {
        pinned(&self.0).center(id)
    }
    #[inline]
    fn view(&mut self, min: Vec2, max: Vec2, callback: impl FnMut(TapestryLayoutItem)) {
        pinned_mut(&mut self.0).view(min, max, callback);
    }
}

#[inline]
fn pinned(
    layouter: &IndependentLayouter<ShortId, BuildHasherDefault<RandomIdHasher>>,
) -> &impl Layouter<TapestryWeaveInner, ShortId, TapestryNode, NodeContent, Vec2, ArrayVec<[Vec2; 6]>>
{
    layouter
}

#[inline]
fn pinned_mut(
    layouter: &mut IndependentLayouter<ShortId, BuildHasherDefault<RandomIdHasher>>,
) -> &mut impl Layouter<TapestryWeaveInner, ShortId, TapestryNode, NodeContent, Vec2, ArrayVec<[Vec2; 6]>>
{
    layouter
}
