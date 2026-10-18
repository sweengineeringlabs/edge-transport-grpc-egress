//! `MetadataExtender` -- a `Metadata` made from another one, with entries added.

use proto_pattern::{Metadata, MetadataBuilder, MetadataError, MetadataView};

/// Builds a new [`Metadata`] from an existing one: the entries of the source, text and binary, and the entries added
/// after them. `Metadata` is immutable, so adding an entry builds a new value; interceptors and views that add a
/// header pay one copy of the headers of the call, and a call that adds none pays nothing.
pub(crate) struct MetadataExtender;

impl MetadataExtender {
    /// A copy of `source` with the text entry `key: value` after its own entries.
    ///
    /// # Errors
    ///
    /// [`MetadataError`] when `key` or `value` is not valid metadata.
    pub(crate) fn append(
        source: &Metadata,
        key: &str,
        value: &str,
    ) -> Result<Metadata, MetadataError> {
        let mut builder = Self::start(source);
        builder.append(key, value)?;
        Ok(builder.build())
    }

    /// A copy of `source` with `added` text entries after its own. A key that `source` already holds is not added:
    /// a header the caller set is the caller's.
    ///
    /// # Errors
    ///
    /// [`MetadataError`] for the first added key or value that is not valid metadata.
    pub(crate) fn extend<'a>(
        source: &Metadata,
        added: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Metadata, MetadataError> {
        let mut builder = Self::start(source);
        for (key, value) in added {
            if source.get_values(key).is_empty() {
                builder.append(key, value)?;
            }
        }
        Ok(builder.build())
    }

    /// A copy of any view of metadata. An entry that cannot be copied (a binary entry that cannot be read) is
    /// left out: the copy is for an interceptor to look at.
    pub(crate) fn copy<M: MetadataView>(source: &M) -> Metadata {
        let mut builder = MetadataBuilder::create();
        for entry in source.get_entries() {
            // A key and value read from valid metadata are valid; if the check ever disagrees the entry is left out.
            let _ = builder.append(entry.get_key(), entry.get_value());
        }
        for entry in source.get_binary_entries().flatten() {
            let _ = builder.append_binary(entry.get_key(), entry.get_value());
        }
        builder.build()
    }

    fn start(source: &Metadata) -> MetadataBuilder {
        let mut builder = MetadataBuilder::create();
        for entry in source.get_entries() {
            // Entries of a `Metadata` are valid by construction.
            let _ = builder.append(entry.get_key(), entry.get_value());
        }
        for entry in MetadataView::get_binary_entries(source).flatten() {
            let _ = builder.append_binary(entry.get_key(), entry.get_value());
        }
        builder
    }
}
