//! `MetadataExtender` -- a `Metadata` made from another one, with entries added.

use proto_pattern::{Metadata, MetadataError, MetadataView};

/// Builds a new [`Metadata`] from an existing one: the entries of the source, text and binary, and the entries added
/// after them. `Metadata` is immutable, so adding an entry builds a new value; interceptors and views that add a
/// header pay one copy of the headers of the call, and a call that adds none pays nothing.
pub trait MetadataExtender {
    /// A copy of `source` with the text entry `key: value` after its own entries.
    ///
    /// # Errors
    ///
    /// [`MetadataError`] when `key` or `value` is not valid metadata.
    fn append(&self, source: &Metadata, key: &str, value: &str) -> Result<Metadata, MetadataError>;

    /// A copy of `source` with `added` text entries after its own. A key that `source` already holds is not added:
    /// a header the caller set is the caller's.
    ///
    /// # Errors
    ///
    /// [`MetadataError`] for the first added key or value that is not valid metadata.
    fn extend<'a>(
        &self,
        source: &Metadata,
        added: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Metadata, MetadataError>;

    /// A copy of any view of metadata, propagating query and binary decoding failures.
    ///
    /// # Errors
    ///
    /// [`MetadataError`] when the source cannot be queried or a binary entry cannot be decoded.
    fn copy<M: MetadataView>(&self, source: &M) -> Result<Metadata, MetadataError>;
}
