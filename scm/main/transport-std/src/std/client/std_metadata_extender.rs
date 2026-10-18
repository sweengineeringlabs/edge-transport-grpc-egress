//! `StdMetadataExtender` -- the standard [`MetadataExtender`].

use grpc_egress_transport::proto_pattern::{
    Metadata, MetadataBuilder, MetadataError, MetadataView,
};
use grpc_egress_transport::MetadataExtender;

/// Builds a new [`Metadata`] from an existing one, with entries added after the ones it holds.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StdMetadataExtender;

impl MetadataExtender for StdMetadataExtender {
    fn append(&self, source: &Metadata, key: &str, value: &str) -> Result<Metadata, MetadataError> {
        let mut builder = Self::start(source);
        builder.append(key, value)?;
        Ok(builder.build())
    }

    fn extend<'a>(
        &self,
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

    fn copy<M: MetadataView>(&self, source: &M) -> Metadata {
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
}

impl StdMetadataExtender {
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
