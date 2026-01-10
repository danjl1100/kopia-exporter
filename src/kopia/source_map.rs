use crate::SourceStr;
use std::collections::BTreeMap;

/// Map from [`SourceStr`] to the desired data elements
#[derive(Clone, Debug, Default)]
pub struct SourceMap<T>(BTreeMap<SourceStr, T>);
impl<T> SourceMap<T> {
    #[must_use]
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }
    pub fn entry(
        &mut self,
        key: SourceStr,
    ) -> std::collections::btree_map::Entry<'_, SourceStr, T> {
        let Self(inner) = self;
        inner.entry(key)
    }
    /// Returns a single value if it is the only value
    ///
    /// # Errors
    /// Returns an error if the source is not found or is not the only value
    #[expect(clippy::missing_panics_doc)] // panic checks for logic error
    pub fn into_expect_only(mut self, source: &SourceStr) -> Result<T, Self> {
        let Self(inner) = &mut self;

        let 1 = inner.len() else {
            return Err(self);
        };

        let Some(value) = inner.remove(source) else {
            return Err(self);
        };

        assert!(inner.is_empty(), "length 1, removed 1, should be empty");
        Ok(value)
    }
    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, SourceStr, T> {
        let Self(inner) = self;
        inner.iter()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let Self(inner) = self;
        inner.is_empty()
    }
    pub fn map_nonempty<U>(self, map_fn: impl FnOnce(Self) -> U) -> Option<U> {
        if self.is_empty() {
            None
        } else {
            Some(map_fn(self))
        }
    }
}
impl<'a, T> IntoIterator for &'a SourceMap<T> {
    type Item = (&'a SourceStr, &'a T);
    type IntoIter = std::collections::btree_map::Iter<'a, SourceStr, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
impl<T> FromIterator<(SourceStr, T)> for SourceMap<T> {
    fn from_iter<U: IntoIterator<Item = (SourceStr, T)>>(iter: U) -> Self {
        Self(iter.into_iter().collect())
    }
}
