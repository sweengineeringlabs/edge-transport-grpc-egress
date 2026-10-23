//! `StdMetadataExtender` -- the standard [`MetadataExtender`].

use grpc_egress_transport_api::proto_pattern::{
    GetBinaryEntriesRequest, GetEntriesRequest, GetValuesRequest, Metadata, MetadataBuilder,
    MetadataError, MetadataView,
};
use grpc_egress_transport_api::MetadataExtender;

/// Builds a new [`Metadata`] from an existing one, preserving text and binary entries.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StdMetadataExtender;

impl MetadataExtender for StdMetadataExtender {
    fn append(&self, source: &Metadata, key: &str, value: &str) -> Result<Metadata, MetadataError> {
        let mut builder = Self::start(source)?;
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

    fn copy<M: MetadataView>(&self, source: &M) -> Result<Metadata, MetadataError> {
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

impl StdMetadataExtender {
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
