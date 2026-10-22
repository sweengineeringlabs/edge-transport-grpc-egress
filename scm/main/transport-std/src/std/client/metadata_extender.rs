//! `MetadataExtender` -- a `Metadata` made from another one, with entries added.

use proto_pattern::{
    GetBinaryEntriesRequest, GetEntriesRequest, GetValuesRequest, Metadata, MetadataBuilder,
    MetadataError, MetadataView,
};

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
        let mut builder = Self::start(source)?;
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
        let mut builder = Self::start(source)?;
        for (key, value) in added {
            if source
                .get_values(GetValuesRequest::new(key))?
                .into_values()
                .is_empty()
            {
                builder.append(key, value)?;
            }
        }
        Ok(builder.build())
    }

    /// Copy metadata, propagating query and binary decoding failures.
    pub(crate) fn copy<M: MetadataView>(source: &M) -> Result<Metadata, MetadataError> {
        let mut builder = MetadataBuilder::create();
        for entry in source.get_entries(GetEntriesRequest::new())?.into_entries() {
            builder.append(entry.get_key(), entry.get_value())?;
        }
        for entry in source
            .get_binary_entries(GetBinaryEntriesRequest::new())?
            .into_entries()
        {
            let entry = entry?;
            builder.append_binary(entry.get_key(), entry.get_value())?;
        }
        Ok(builder.build())
    }

    fn start(source: &Metadata) -> Result<MetadataBuilder, MetadataError> {
        let mut builder = MetadataBuilder::create();
        for entry in source.get_entries(GetEntriesRequest::new())?.into_entries() {
            builder.append(entry.get_key(), entry.get_value())?;
        }
        for entry in source
            .get_binary_entries(GetBinaryEntriesRequest::new())?
            .into_entries()
        {
            let entry = entry?;
            builder.append_binary(entry.get_key(), entry.get_value())?;
        }
        Ok(builder)
    }
}
