//! `StdMetadataExtender` -- the standard [`MetadataExtender`].

<<<<<<< HEAD:scm/main/transport-std/src/std/client/std_metadata_extender.rs
use grpc_egress_transport::proto_pattern::{
    Metadata, MetadataBuilder, MetadataError, MetadataView,
};
use grpc_egress_transport::MetadataExtender;
=======
use proto_pattern::{
    GetBinaryEntriesRequest, GetEntriesRequest, GetValuesRequest, Metadata, MetadataBuilder,
    MetadataError, MetadataView,
};
>>>>>>> fix/18-deny-allow-git:scm/main/transport-std/src/std/client/metadata_extender.rs

/// Builds a new [`Metadata`] from an existing one, with entries added after the ones it holds.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StdMetadataExtender;

<<<<<<< HEAD:scm/main/transport-std/src/std/client/std_metadata_extender.rs
impl MetadataExtender for StdMetadataExtender {
    fn append(&self, source: &Metadata, key: &str, value: &str) -> Result<Metadata, MetadataError> {
        let mut builder = Self::start(source);
=======
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
>>>>>>> fix/18-deny-allow-git:scm/main/transport-std/src/std/client/metadata_extender.rs
        builder.append(key, value)?;
        Ok(builder.build())
    }

    fn extend<'a>(
        &self,
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

<<<<<<< HEAD:scm/main/transport-std/src/std/client/std_metadata_extender.rs
    fn copy<M: MetadataView>(&self, source: &M) -> Metadata {
=======
    /// Copy metadata, propagating query and binary decoding failures.
    pub(crate) fn copy<M: MetadataView>(source: &M) -> Result<Metadata, MetadataError> {
>>>>>>> fix/18-deny-allow-git:scm/main/transport-std/src/std/client/metadata_extender.rs
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
}

<<<<<<< HEAD:scm/main/transport-std/src/std/client/std_metadata_extender.rs
impl StdMetadataExtender {
    fn start(source: &Metadata) -> MetadataBuilder {
=======
    fn start(source: &Metadata) -> Result<MetadataBuilder, MetadataError> {
>>>>>>> fix/18-deny-allow-git:scm/main/transport-std/src/std/client/metadata_extender.rs
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
