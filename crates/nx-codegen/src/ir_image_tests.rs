//! Tests of the NX IR image that need the emitter: the corpus images through the writer and the
//! validating reader of `nx-ir`.

use crate::ir_corpus_tests::{emit, load_programs};
use nx_ir::{
    explain_nx_ir, kinds, section, write_nx_ir_image, Cells, NxIrArtifact, NxIrImage, NxIrImageBuf,
    NxIrImageError, Table, NONE,
};

fn cell_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// Every corpus artifact, with and without debug data, as the model and its image.
fn corpus_images() -> Vec<(String, NxIrArtifact, Vec<u8>)> {
    let mut images = Vec::new();
    for program in load_programs() {
        for debug in [false, true] {
            for artifact in emit(&program, debug) {
                let label = format!(
                    "{}/{}{}",
                    program.name,
                    artifact.modules[0].identity,
                    if debug { " +debug" } else { "" }
                );
                let bytes = write_nx_ir_image(&artifact).expect("image");
                images.push((label, artifact, bytes));
            }
        }
    }
    images
}

fn snippet_input() -> NxIrArtifact {
    let program = load_programs()
        .into_iter()
        .find(|program| program.name == "snippet")
        .expect("the snippet program");
    emit(&program, false)
        .into_iter()
        .find(|artifact| artifact.modules[0].identity == "input.nx")
        .expect("input.nx")
}

/// The image of the corpus's `snippet` program, checked cell by cell against the layout
/// `docs/nx-ir-format.md` gives.
#[test]
fn the_snippet_image_is_laid_out_as_the_document_says() {
    let artifact = snippet_input();
    let bytes = write_nx_ir_image(&artifact).expect("image");

    assert_eq!(&bytes[0..4], b"NXIR");
    assert_eq!(cell_at(&bytes, 4), 5, "schema version");
    assert_eq!(cell_at(&bytes, 8) as usize, bytes.len(), "total length");
    assert_eq!(cell_at(&bytes, 12), 6, "six sections without debug");

    // The directory: kinds 0 to 5, contiguous, each four-byte aligned.
    let mut expected_offset = 16 + 6 * 12;
    let mut sections = Vec::new();
    for entry in 0..6 {
        let at = 16 + entry * 12;
        assert_eq!(cell_at(&bytes, at), entry as u32, "section kind");
        assert_eq!(
            cell_at(&bytes, at + 4) as usize,
            expected_offset,
            "section offset"
        );
        let length = cell_at(&bytes, at + 8) as usize;
        assert_eq!(length % 4, 0);
        sections.push((expected_offset, length));
        expected_offset += length;
    }
    assert_eq!(expected_offset, bytes.len());

    // Strings: count, count + 1 offsets, blob. The first string is "title".
    let (strings, _) = sections[0];
    assert_eq!(cell_at(&bytes, strings) as usize, artifact.strings.len());
    assert_eq!(cell_at(&bytes, strings + 4), 0);
    assert_eq!(cell_at(&bytes, strings + 8), 5);
    let blob = strings + 4 + 4 * (artifact.strings.len() + 1);
    assert_eq!(&bytes[blob..blob + 5], b"title");

    // Module: runtime ABI, no features, two modules, entrypoints [1] and [].
    let (module, module_len) = sections[1];
    let cells = Cells::from_bytes(&bytes[module..module + module_len]).to_vec();
    let string_index =
        |value: &str| artifact.strings.iter().position(|s| s == value).unwrap() as u32;
    let abi = string_index("nx-ir-runtime-v2");
    let drawnui = artifact.modules[1].fingerprint;
    assert_eq!(
        cells,
        vec![
            abi,
            0,
            2,
            string_index("input.nx"),
            string_index(""),
            artifact.modules[0].fingerprint as u32,
            (artifact.modules[0].fingerprint >> 32) as u32,
            string_index("drawnui.nx"),
            string_index("9"),
            drawnui as u32,
            (drawnui >> 32) as u32,
            1,
            1,
            0,
        ]
    );

    // Declarations: `[1, 0, 0, -1]` (value title = node 0, no declared type) and
    // `[0, 2, [], 13, -1, 0]` (function root, no declared result, not optional).
    let (declarations, declarations_len) = sections[5];
    let cells = Cells::from_bytes(&bytes[declarations..declarations + declarations_len]).to_vec();
    assert_eq!(
        cells,
        vec![2, 0, 4, 10, 1, 0, 0, NONE, 0, 2, 0, 13, NONE, 0]
    );

    // Node 13, `[18, 1, 14, [[3, 1], [5, 2]], [5, 7, 9, 12]]`: the component descriptor of
    // `SkiaLayout` with two properties and four children.
    let image = NxIrImage::open(&bytes).expect("valid");
    let node = image.entry(Table::Nodes, 13).expect("node 13").to_vec();
    assert_eq!(node, vec![18, 1, 14, 2, 3, 1, 5, 2, 4, 5, 7, 9, 12]);
    assert_eq!(image.string(14), Some("SkiaLayout"));
}

#[test]
fn every_corpus_image_reads_back_as_its_model() {
    for (label, artifact, bytes) in corpus_images() {
        let image = NxIrImage::open(&bytes).unwrap_or_else(|error| panic!("{label}: {error}"));
        assert_eq!(image.to_artifact(), artifact, "{label}");
        assert_eq!(image.has_debug(), artifact.debug.is_some(), "{label}");
    }
}

#[test]
fn a_stripped_image_and_a_debug_image_differ_only_in_the_debug_section() {
    for program in load_programs() {
        let stripped = emit(&program, false);
        let with_debug = emit(&program, true);
        for (stripped, with_debug) in stripped.iter().zip(&with_debug) {
            let stripped_bytes = write_nx_ir_image(stripped).expect("image");
            let debug_bytes = write_nx_ir_image(with_debug).expect("image");
            // Past the header and directory, the debug image is the stripped image's sections
            // followed by the debug section.
            let stripped_body = &stripped_bytes[16 + 6 * 12..];
            let debug_body = &debug_bytes[16 + 7 * 12..];
            assert!(debug_body.starts_with(stripped_body), "{}", program.name);
        }
    }
}

#[test]
fn an_unknown_section_is_skipped() {
    let bytes = write_nx_ir_image(&snippet_input()).expect("image");
    // Rebuild the image with an extra directory entry of kind 99 pointing at four zero bytes
    // appended to the end.
    let count = 7u32;
    let directory_len = 16 + 7 * 12;
    let mut image = Vec::new();
    image.extend_from_slice(&bytes[..12]);
    image.extend_from_slice(&count.to_le_bytes());
    for entry in 0..6 {
        let at = 16 + entry * 12;
        image.extend_from_slice(&bytes[at..at + 4]);
        image.extend_from_slice(&(cell_at(&bytes, at + 4) + 12).to_le_bytes());
        image.extend_from_slice(&bytes[at + 8..at + 12]);
    }
    image.extend_from_slice(&99u32.to_le_bytes());
    image.extend_from_slice(&((bytes.len() + 12) as u32).to_le_bytes());
    image.extend_from_slice(&4u32.to_le_bytes());
    image.extend_from_slice(&bytes[16 + 6 * 12..]);
    image.extend_from_slice(&[0, 0, 0, 0]);
    assert_eq!(image.len(), bytes.len() + 16);
    assert_eq!(directory_len, 16 + 7 * 12);
    let total = image.len() as u32;
    image[8..12].copy_from_slice(&total.to_le_bytes());

    let opened = NxIrImage::open(&image).expect("the unknown section is skipped");
    assert_eq!(opened.to_artifact(), snippet_input());
}

#[test]
fn bytes_that_are_not_an_image_are_refused() {
    assert_eq!(
        NxIrImage::open(b"").unwrap_err(),
        NxIrImageError::NotAnImage
    );
    assert_eq!(
        NxIrImage::open(b"{\"format\":\"nx-ir-json\"}").unwrap_err(),
        NxIrImageError::NotAnImage
    );
    assert_eq!(
        NxIrImage::open(b"NXI").unwrap_err(),
        NxIrImageError::NotAnImage
    );
}

#[test]
fn another_schema_version_is_refused_naming_both() {
    let mut bytes = write_nx_ir_image(&snippet_input()).expect("image");
    bytes[4..8].copy_from_slice(&3u32.to_le_bytes());
    assert_eq!(
        NxIrImage::open(&bytes).unwrap_err(),
        NxIrImageError::SchemaVersion {
            found: 3,
            supported: 5
        }
    );
}

/// A `seq` type whose occurrence cell spells no suffix, and one whose item is itself a `seq`,
/// are refused, as the TypeScript reader refuses them.
#[test]
fn a_malformed_seq_type_is_refused() {
    use nx_ir::IrItem;
    let base = snippet_input();
    let int = base.types.len() as i64;
    let seq = int + 1;
    for (extra, needle) in [
        (IrItem::ints([kinds::ty::SEQ, int, 0]), "occurrence cell 0"),
        (IrItem::ints([kinds::ty::SEQ, int, 4]), "occurrence cell 4"),
        (
            IrItem::ints([kinds::ty::SEQ, seq, kinds::ty::OCCURRENCE_EMPTY]),
            "is itself a seq type",
        ),
    ] {
        let mut artifact = base.clone();
        let name = artifact.strings.len() as i64;
        artifact.strings.push("int".to_string());
        artifact
            .types
            .push(IrItem::ints([kinds::ty::PRIMITIVE, name]));
        artifact.types.push(IrItem::ints([
            kinds::ty::SEQ,
            int,
            kinds::ty::OCCURRENCE_MANY,
        ]));
        artifact.types.push(extra);
        let bytes = write_nx_ir_image(&artifact).expect("image");
        match NxIrImage::open(&bytes) {
            Err(NxIrImageError::Malformed(message)) => {
                assert!(message.contains(needle), "{message}")
            }
            other => panic!("expected a malformed image, got {other:?}"),
        }
    }
}

/// Cut at every four-byte boundary, an image is refused rather than read short.
#[test]
fn a_truncated_image_is_refused_at_every_boundary() {
    for (label, _, bytes) in corpus_images() {
        for end in (0..bytes.len()).step_by(4) {
            let error = NxIrImage::open(&bytes[..end]).err().unwrap_or_else(|| {
                panic!(
                    "{label}: an image cut at {end} of {} bytes opened",
                    bytes.len()
                )
            });
            assert!(
                matches!(
                    error,
                    NxIrImageError::NotAnImage | NxIrImageError::Malformed(_)
                ),
                "{label} at {end}: {error}"
            );
        }
    }
}

/// The image whose header says it is longer than it is, and the one that says it is shorter.
#[test]
fn a_wrong_total_length_is_refused() {
    let bytes = write_nx_ir_image(&snippet_input()).expect("image");
    let mut longer = bytes.clone();
    longer[8..12].copy_from_slice(&((bytes.len() + 4) as u32).to_le_bytes());
    assert!(NxIrImage::open(&longer).is_err());
    let mut shorter = bytes.clone();
    shorter[8..12].copy_from_slice(&((bytes.len() - 4) as u32).to_le_bytes());
    assert!(NxIrImage::open(&shorter).is_err());
    let mut padded = bytes.clone();
    padded.extend_from_slice(&[0, 0, 0, 0]);
    assert!(NxIrImage::open(&padded).is_err());
}

/// Opens damaged bytes and, when they open, reads everything the view answers, so that a
/// damaged image is either refused or explained and never panics.
fn open_damaged(damaged: &[u8]) -> bool {
    match NxIrImage::open(damaged) {
        Ok(image) => {
            // Everything the view answers must come from inside the image.
            let artifact = image.to_artifact();
            let _ = explain_nx_ir(&artifact);
            true
        }
        Err(_) => false,
    }
}

/// Every cell of `bytes`, overwritten with each of four values: `open` either refuses the image
/// or answers with a view whose every accessor is in range, and never panics.
fn damage_every_cell(label: &str, bytes: &[u8]) {
    let mut opened = 0;
    let mut refused = 0;
    for offset in (0..bytes.len()).step_by(4) {
        for value in [0u32, 1, NONE, 0x7FFF_FFF0] {
            let mut damaged = bytes.to_vec();
            damaged[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            if open_damaged(&damaged) {
                opened += 1;
            } else {
                refused += 1;
            }
        }
    }
    println!("{label}: {opened} damaged images opened, {refused} refused");
    assert!(refused > 0);
}

/// The smallest corpus image, which is always a stripped one, the smallest that carries a
/// debug section, so that span offsets and the source length are damaged too, and the smallest
/// that carries an action handler, so that its node and a non-empty `emits` list are too.
#[test]
fn every_cell_can_be_damaged_without_a_panic() {
    let images = corpus_images();
    let (label, _, bytes) = images
        .iter()
        .filter(|(_, artifact, _)| {
            artifact.nodes.iter().any(|node| {
                node.as_list().and_then(|entry| entry[0].as_int())
                    == Some(kinds::node::ACTION_HANDLER)
            })
        })
        .min_by_key(|(_, _, bytes)| bytes.len())
        .expect("a corpus image with an action handler");
    damage_every_cell(label, bytes);
    let (label, _, bytes) = images
        .iter()
        .min_by_key(|(_, _, bytes)| bytes.len())
        .expect("a corpus image");
    damage_every_cell(label, bytes);
    let (label, _, bytes) = images
        .iter()
        .filter(|(_, artifact, _)| artifact.debug.is_some())
        .min_by_key(|(_, _, bytes)| bytes.len())
        .expect("a corpus image with a debug section");
    damage_every_cell(label, bytes);
}

/// The byte offset of cell `cell` of entry `index` in `table`, read from the directory.
fn table_cell_offset(bytes: &[u8], table: Table, index: usize, cell: usize) -> usize {
    let kind = match table {
        Table::Types => section::TYPES,
        Table::Constants => section::CONSTANTS,
        Table::Nodes => section::NODES,
        Table::Declarations => section::DECLARATIONS,
    };
    let section_offset = cell_at(bytes, 16 + kind as usize * 12 + 4) as usize;
    let count = cell_at(bytes, section_offset) as usize;
    let start = cell_at(bytes, section_offset + 4 + 4 * index) as usize;
    section_offset + 4 + 4 * (count + 1) + 4 * (start + cell)
}

/// A node that names itself as a child is refused, since the explainer recurses over children
/// and the format promises that a child precedes its parent.
#[test]
fn a_self_referencing_node_is_refused() {
    let artifact = snippet_input();
    let bytes = write_nx_ir_image(&artifact).expect("image");
    let image = NxIrImage::open(&bytes).expect("valid");
    // Node 13 is `[18, 1, 14, [[3, 1], [5, 2]], [5, 7, 9, 12]]`; its first child is cell 9.
    assert_eq!(image.entry(Table::Nodes, 13).unwrap().get(9), Some(5));
    let offset = table_cell_offset(&bytes, Table::Nodes, 13, 9);
    let mut cyclic = bytes.clone();
    cyclic[offset..offset + 4].copy_from_slice(&13u32.to_le_bytes());
    let error = NxIrImage::open(&cyclic).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("node 13: node index 13 is out of range"),
        "{error}"
    );
    // A child that follows its parent is refused the same way.
    let mut forward = bytes.clone();
    forward[offset..offset + 4].copy_from_slice(&14u32.to_le_bytes());
    assert!(NxIrImage::open(&forward).is_err());
}

/// Every node and type cell of every corpus image, overwritten with its own entry index: the
/// image is refused or explained, and the explainer's walk over children terminates.
#[test]
fn every_node_and_type_cell_can_name_its_own_entry_without_a_panic() {
    for (label, _, bytes) in corpus_images() {
        let image = NxIrImage::open(&bytes).expect("valid");
        for table in [Table::Nodes, Table::Types] {
            for index in 0..image.entry_count(table) {
                let len = image.entry(table, index as u32).unwrap().len();
                for cell in 0..len {
                    let offset = table_cell_offset(&bytes, table, index, cell);
                    let mut damaged = bytes.clone();
                    damaged[offset..offset + 4].copy_from_slice(&(index as u32).to_le_bytes());
                    let _ = open_damaged(&damaged);
                }
            }
        }
        println!("{label}: every node and type cell probed with its own index");
    }
}

#[test]
fn an_index_past_a_table_is_refused_rather_than_followed() {
    let artifact = snippet_input();
    let bytes = write_nx_ir_image(&artifact).expect("image");
    let image = NxIrImage::open(&bytes).expect("valid");
    // Node 0 is `[2, 1]`, the string literal "Conformance"; point it past the string table.
    let node_section = 16 + 4 * 12;
    let nodes_offset = cell_at(&bytes, node_section + 4) as usize;
    let node_count = cell_at(&bytes, nodes_offset) as usize;
    let pool = nodes_offset + 4 + 4 * (node_count + 1);
    assert_eq!(image.entry(Table::Nodes, 0).unwrap().to_vec(), vec![2, 1]);
    let mut damaged = bytes.clone();
    damaged[pool + 4..pool + 8].copy_from_slice(&(artifact.strings.len() as u32).to_le_bytes());
    let error = NxIrImage::open(&damaged).unwrap_err();
    assert!(
        error.to_string().contains("node 0: string index"),
        "{error}"
    );
}

#[test]
fn an_owned_image_rebuilds_the_view_validation_produced() {
    for (label, artifact, bytes) in corpus_images() {
        let opened = NxIrImage::open(&bytes).unwrap_or_else(|error| panic!("{label}: {error}"));
        let owned =
            NxIrImageBuf::open(bytes.clone()).unwrap_or_else(|error| panic!("{label}: {error}"));
        let rebuilt = owned.image();

        assert_eq!(owned.bytes(), bytes.as_slice(), "{label}");
        assert_eq!(rebuilt.to_artifact(), artifact, "{label}");
        assert_eq!(rebuilt.schema_version(), opened.schema_version(), "{label}");
        assert_eq!(rebuilt.runtime_abi(), opened.runtime_abi(), "{label}");
        assert_eq!(
            rebuilt.required_features().collect::<Vec<_>>(),
            opened.required_features().collect::<Vec<_>>(),
            "{label}"
        );
        assert_eq!(
            rebuilt.modules().collect::<Vec<_>>(),
            opened.modules().collect::<Vec<_>>(),
            "{label}"
        );
        assert_eq!(
            rebuilt.function_entrypoints().to_vec(),
            opened.function_entrypoints().to_vec(),
            "{label}"
        );
        assert_eq!(
            rebuilt.component_entrypoints().to_vec(),
            opened.component_entrypoints().to_vec(),
            "{label}"
        );
        assert_eq!(rebuilt.string_count(), opened.string_count(), "{label}");
        for index in 0..=opened.string_count() as u32 {
            assert_eq!(rebuilt.string(index), opened.string(index), "{label}");
        }
        for table in [
            Table::Types,
            Table::Constants,
            Table::Nodes,
            Table::Declarations,
        ] {
            assert_eq!(
                rebuilt.entry_count(table),
                opened.entry_count(table),
                "{label}"
            );
            for index in 0..=opened.entry_count(table) as u32 {
                assert_eq!(
                    rebuilt.entry(table, index).map(|cells| cells.to_vec()),
                    opened.entry(table, index).map(|cells| cells.to_vec()),
                    "{label}"
                );
            }
        }
        assert_eq!(rebuilt.has_debug(), opened.has_debug(), "{label}");
        assert_eq!(rebuilt.source(), opened.source(), "{label}");
        for index in 0..=opened.entry_count(Table::Declarations) as u32 {
            assert_eq!(
                rebuilt.declaration_name(index),
                opened.declaration_name(index)
            );
            assert_eq!(
                rebuilt.declaration_span(index),
                opened.declaration_span(index)
            );
        }
        for index in 0..=opened.entry_count(Table::Nodes) as u32 {
            assert_eq!(rebuilt.node_span(index), opened.node_span(index), "{label}");
        }
    }
}

#[test]
fn an_owned_image_refuses_what_the_reader_refuses() {
    let (_, _, bytes) = corpus_images().remove(0);
    assert!(NxIrImageBuf::open(bytes[..bytes.len() - 4].to_vec()).is_err());
    assert!(NxIrImageBuf::open(b"nope".to_vec()).is_err());
}
