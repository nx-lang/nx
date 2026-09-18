//! Several emitted artifacts in one payload, for a boundary that returns one buffer per call.
//!
//! <para>The wasm ABI and the C FFI each answer a call with one pointer and length. `generateNxIr`
//! may emit several modules at once, so those boundaries carry a *bundle*: a little-endian `u32`
//! header length, a JSON header `[{ identity, metadata, offset, length }]`, zero padding to a
//! four-byte boundary, then the images back to back at the offsets the header gives, each offset
//! measured from the start of the bundle and a multiple of four. The Node binding has a byte type
//! of its own and does not use the bundle.</para>

use crate::ir::{GeneratedNxIr, NxIrMetadata};
use serde::{Deserialize, Serialize};

/// One entry of a bundle's header.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxIrBundleEntry {
    pub identity: String,
    pub metadata: NxIrMetadata,
    /// Where the image starts, from the start of the bundle.
    pub offset: u32,
    /// The image's length in bytes.
    pub length: u32,
}

/// Writes the artifacts as a bundle.
pub fn write_nx_ir_bundle(artifacts: &[GeneratedNxIr]) -> Result<Vec<u8>, String> {
    // The header names offsets, which depend on the header's own length, so it is built with the
    // offsets an empty header would give and rebuilt once its length is known. Every offset is a
    // decimal in JSON, so the second header can be longer than the first; a third pass settles it.
    let mut header_len = 0usize;
    loop {
        let images_start = (4 + header_len).div_ceil(4) * 4;
        let mut offset = images_start;
        let entries = artifacts
            .iter()
            .map(|artifact| {
                let entry = NxIrBundleEntry {
                    identity: artifact.identity.clone(),
                    metadata: artifact.metadata.clone(),
                    offset: u32::try_from(offset).map_err(|_| "the bundle exceeds 4 GiB")?,
                    length: u32::try_from(artifact.bytes.len())
                        .map_err(|_| "an image exceeds 4 GiB")?,
                };
                offset += artifact.bytes.len().div_ceil(4) * 4;
                Ok(entry)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let header = serde_json::to_vec(&entries).map_err(|error| error.to_string())?;
        if header.len() != header_len {
            header_len = header.len();
            continue;
        }
        let mut bundle = Vec::with_capacity(offset);
        bundle.extend_from_slice(&(header.len() as u32).to_le_bytes());
        bundle.extend_from_slice(&header);
        bundle.resize(images_start, 0);
        for artifact in artifacts {
            bundle.extend_from_slice(&artifact.bytes);
            let padded = bundle.len().div_ceil(4) * 4;
            bundle.resize(padded, 0);
        }
        return Ok(bundle);
    }
}

/// Reads a bundle back into artifacts, copying each image out of it.
pub fn read_nx_ir_bundle(bytes: &[u8]) -> Result<Vec<GeneratedNxIr>, String> {
    let header_len = bytes
        .get(..4)
        .map(|len| u32::from_le_bytes([len[0], len[1], len[2], len[3]]) as usize)
        .ok_or("the bundle has no header length")?;
    let header = bytes
        .get(4..4 + header_len)
        .ok_or("the bundle's header does not fit")?;
    let entries: Vec<NxIrBundleEntry> = serde_json::from_slice(header)
        .map_err(|error| format!("the bundle's header is not valid: {error}"))?;
    entries
        .into_iter()
        .map(|entry| {
            let start = entry.offset as usize;
            let image = bytes
                .get(start..start + entry.length as usize)
                .ok_or_else(|| {
                    format!("the image of {} lies outside the bundle", entry.identity)
                })?;
            Ok(GeneratedNxIr {
                identity: entry.identity,
                bytes: image.to_vec(),
                metadata: entry.metadata,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(identity: &str, bytes: Vec<u8>) -> GeneratedNxIr {
        GeneratedNxIr {
            identity: identity.to_string(),
            bytes,
            metadata: NxIrMetadata {
                identity: identity.to_string(),
                fingerprint: 7,
                schema_version: 4,
                runtime_abi: "nx-ir-runtime-v2".to_string(),
                required_features: Vec::new(),
                function_entrypoints: vec!["root".to_string()],
                component_entrypoints: Vec::new(),
            },
        }
    }

    #[test]
    fn a_bundle_of_two_artifacts_round_trips() {
        let artifacts = vec![
            artifact("app/main.nx", vec![1, 2, 3, 4, 5, 6, 7, 8]),
            artifact("shared/model.nx", vec![9, 10, 11]),
        ];
        let bundle = write_nx_ir_bundle(&artifacts).expect("bundle");
        assert_eq!(bundle.len() % 4, 0);
        let header_len = u32::from_le_bytes([bundle[0], bundle[1], bundle[2], bundle[3]]) as usize;
        let entries: Vec<NxIrBundleEntry> =
            serde_json::from_slice(&bundle[4..4 + header_len]).unwrap();
        assert_eq!(entries[0].offset % 4, 0);
        assert_eq!(entries[1].offset % 4, 0);
        assert_eq!(entries[1].length, 3);
        assert_eq!(read_nx_ir_bundle(&bundle).expect("read back"), artifacts);
    }

    #[test]
    fn an_empty_bundle_round_trips() {
        let bundle = write_nx_ir_bundle(&[]).expect("bundle");
        assert_eq!(read_nx_ir_bundle(&bundle).expect("read back"), Vec::new());
    }

    #[test]
    fn a_cut_bundle_is_refused() {
        let bundle = write_nx_ir_bundle(&[artifact("main.nx", vec![1, 2, 3, 4])]).expect("bundle");
        assert!(read_nx_ir_bundle(&bundle[..bundle.len() - 4]).is_err());
        assert!(read_nx_ir_bundle(&bundle[..2]).is_err());
    }
}
