//! Regression fixtures from the four saved 0.6.0 jungle crowns (seed
//! 4312740467388825473), plus bounded native-column/lifecycle cases.
//! These test sparse added geometry; the existing coarse wood vote is unchanged.
use dh::{BlockInfo, BlockTable, Dir, FullDataSource, Quad, SampledRun, Stem};
use dh::{full_point, gen_step, material, pos, storage};
use distant_horizons_for_horizon_client as dh;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};

const MIN_Y: i32 = -64;
const GROUND: u32 = 9;
const LOG: u32 = 149;
const LEAVES: u32 = 378;
const SHRUB: u32 = 294;
const WATER: u32 = 89;

fn blocks() -> BlockTable {
    let air = BlockInfo {
        air: true,
        material: material::AIR,
        ..BlockInfo::default()
    };
    let mut states = vec![air; 379];
    states[GROUND as usize] = BlockInfo {
        opacity: 16,
        solid: true,
        alpha: 255,
        material: material::GRASS,
        ..BlockInfo::default()
    };
    states[LOG as usize] = BlockInfo {
        opacity: 16,
        solid: true,
        alpha: 255,
        stem: true,
        material: material::STONE,
        ..BlockInfo::default()
    };
    for id in [LEAVES, SHRUB] {
        states[id as usize] = BlockInfo {
            opacity: 1,
            alpha: 255,
            material: material::LEAVES,
            ..BlockInfo::default()
        };
    }
    states[WATER as usize] = BlockInfo {
        liquid: true,
        opacity: 1,
        alpha: 160,
        material: material::WATER,
        ..BlockInfo::default()
    };
    BlockTable { states }
}

// Each interval is an absolute, half-open Y range, with independently set light.
fn put(source: &mut FullDataSource, x: i32, z: i32, runs: &[(u32, i32, i32, u8)], step: i8) {
    let runs: Vec<_> = runs
        .iter()
        .map(|&(state, min, max, sky)| SampledRun {
            state,
            min,
            max,
            sky,
            block: 0,
            biome: 29,
        })
        .collect();
    let column = dh::column_from_runs(&mut source.mapping, &runs, MIN_Y, 320);
    let width = pos::data_width(source.pos);
    source.set_column(
        (x - pos::min_corner_block_x(source.pos)).div_euclid(width),
        (z - pos::min_corner_block_z(source.pos)).div_euclid(width),
        column,
        step,
    );
}

fn crown(source: &mut FullDataSource, x: i32, z: i32, ground: i32, bottom: i32, top: i32, step: i8) {
    put(
        source,
        x,
        z,
        &[(GROUND, MIN_Y, ground, 15), (LEAVES, bottom, top, 15)],
        step,
    );
}

fn predicted(source: &mut FullDataSource, x: i32, z: i32, ground: i32, top: i32, crown_max: i32) -> Stem {
    Stem {
        x,
        z,
        min_y: (ground - MIN_Y) as u16,
        max_y: (top - MIN_Y) as u16,
        id: source.mapping.add_if_not_present(29, LOG),
        sky: 15,
        block: 0,
        leaves: LEAVES,
        crown_max: (crown_max - MIN_Y) as u16,
    }
}

fn one_crown() -> (FullDataSource, Stem) {
    let mut source = FullDataSource::empty(pos::encode(9, 0, 0));
    crown(&mut source, 1, 1, 64, 80, 84, gen_step::SURFACE);
    let stem = predicted(&mut source, 1, 1, 64, 80, 84);
    (source, stem)
}

fn assert_one_block_faces(quads: &[Quad]) {
    for q in quads {
        match q.dir {
            Dir::North | Dir::South | Dir::West | Dir::East => assert_eq!(q.u1 - q.u0, 1),
            Dir::Up | Dir::Down => {
                assert_eq!(q.u1 - q.u0, 1);
                assert_eq!(q.v1 - q.v0, 1);
            }
        }
    }
}

#[test]
fn seven_observed_roots_require_their_own_retained_high_crown() {
    // root XZ, predictor ground/end/crown-max, displayed ground/crown-min/max.
    let cases = [
        (-2450, -1052, 63, 88, 90, 63, 88, 90),
        (-2399, -1075, 67, 95, 99, 67, 95, 96),
        (-2394, -1075, 67, 92, 96, 67, 95, 96),
        (-2409, -1094, 66, 89, 93, 66, 90, 93),
        (-2415, -1090, 67, 90, 94, 66, 90, 93),
        (-2409, -1109, 66, 88, 92, 66, 89, 91),
    ];
    let blocks = blocks();
    for (x, z, base, end, cap, ground, bottom, top) in cases {
        let mut source = FullDataSource::empty(pos::containing_block(9, x, z));
        crown(&mut source, x, z, ground, bottom, top, gen_step::SURFACE);
        let stem = predicted(&mut source, x, z, base, end, cap);
        assert_eq!(
            dh::stem_bounds(&source, &stem, &blocks),
            Some((ground - MIN_Y, bottom - MIN_Y)),
            "root {x},{z}"
        );
        source.stems.push(stem);
        let mut quads = Vec::new();
        dh::append_stem_quads(&source, &blocks, MIN_Y, |_, _| false, &mut quads);
        assert!(!quads.is_empty());
        assert!(quads.len() <= 6);
        assert_one_block_faces(&quads);
    }
    // Its crown contributes to the neighboring cell, but disappears in its own.
    let mut source = FullDataSource::empty(pos::containing_block(9, -2408, -1106));
    put(
        &mut source,
        -2408,
        -1106,
        &[(GROUND, MIN_Y, 66, 15), (SHRUB, 67, 69, 15)],
        gen_step::SURFACE,
    );
    crown(&mut source, -2409, -1109, 66, 89, 91, gen_step::SURFACE);
    let stem = predicted(&mut source, -2408, -1106, 66, 89, 91);
    assert_eq!(dh::stem_bounds(&source, &stem, &blocks), None);
    source.stems.push(stem);
    let mut quads = Vec::new();
    dh::append_stem_quads(&source, &blocks, MIN_Y, |_, _| false, &mut quads);
    assert!(
        quads.is_empty(),
        "do not attach the rejected root to a neighbor or low shrub"
    );
}

#[test]
fn anchoring_adjusts_at_most_one_cell_at_either_end() {
    let (source, mut stem) = one_crown();
    let blocks = blocks();
    stem.min_y = (56 - MIN_Y) as u16;
    stem.max_y = (72 - MIN_Y) as u16;
    assert_eq!(dh::stem_bounds(&source, &stem, &blocks), Some((64 - MIN_Y, 80 - MIN_Y)));
    stem.min_y -= 1;
    assert!(dh::stem_bounds(&source, &stem, &blocks).is_none());
    stem.min_y += 1;
    stem.max_y -= 1;
    assert!(dh::stem_bounds(&source, &stem, &blocks).is_none());
}

#[test]
fn predicted_roots_refuse_water_unknown_ground_and_wrong_foliage() {
    let blocks = blocks();
    let (mut source, stem) = one_crown();
    put(
        &mut source,
        1,
        1,
        &[(GROUND, MIN_Y, 64, 15), (WATER, 64, 70, 15), (LEAVES, 80, 84, 15)],
        gen_step::SURFACE,
    );
    assert!(dh::stem_bounds(&source, &stem, &blocks).is_none());
    put(&mut source, 1, 1, &[(LEAVES, 80, 84, 15)], gen_step::SURFACE);
    assert!(dh::stem_bounds(&source, &stem, &blocks).is_none());
    put(
        &mut source,
        1,
        1,
        &[(GROUND, MIN_Y, 64, 15), (SHRUB, 80, 84, 15)],
        gen_step::SURFACE,
    );
    assert!(dh::stem_bounds(&source, &stem, &blocks).is_none());
    source.set_column(0, 0, Vec::new(), gen_step::EMPTY);
    assert!(dh::stem_bounds(&source, &stem, &blocks).is_none());
}

fn native_two_by_two(split_light: bool) -> FullDataSource {
    let mut source = FullDataSource::empty(pos::encode(6, 0, 0));
    for x in 0..8 {
        for z in 0..8 {
            let mut runs = vec![(GROUND, MIN_Y, 64, 15)];
            if (3..=4).contains(&x) && (3..=4).contains(&z) {
                if split_light {
                    runs.extend([(LOG, 64, 77, 7), (LOG, 77, 90, 15)]);
                } else {
                    runs.push((LOG, 64, 90, 15));
                }
            }
            runs.push((LEAVES, 90, 92, 15));
            put(&mut source, x, z, &runs, gen_step::LIGHT);
        }
    }
    source
}

fn reduce_to_eight(mut source: FullDataSource, blocks: &BlockTable) -> FullDataSource {
    for detail in 7..=9 {
        let mut parent = FullDataSource::empty(pos::encode(detail, 0, 0));
        assert!(parent.update_from(&source, blocks));
        source = parent;
    }
    source
}

#[test]
fn native_two_by_two_survives_reduction_without_added_eight_block_faces() {
    let blocks = blocks();
    let source = reduce_to_eight(native_two_by_two(false), &blocks);
    let roots: BTreeSet<_> = source.stems.iter().map(|s| (s.x, s.z)).collect();
    assert_eq!(roots, BTreeSet::from([(3, 3), (3, 4), (4, 3), (4, 4)]));
    assert!(
        source
            .stems
            .iter()
            .all(|s| s.leaves == 0 && source.mapping.state(s.id) == LOG)
    );
    // This fixture deliberately makes each initial 2x2 vote contain only one log.
    assert!(
        !source
            .column(0, 0)
            .iter()
            .any(|&p| source.mapping.state(full_point::id(p)) == LOG)
    );
    let mut quads = Vec::new();
    dh::append_stem_quads(&source, &blocks, MIN_Y, |_, _| false, &mut quads);
    assert!(!quads.is_empty());
    assert!(quads.len() <= 24);
    assert_one_block_faces(&quads);
    // Ordinary coarse geometry is intentionally not asserted globally wood-free.
}

#[test]
fn lighting_segments_do_not_spend_the_physical_footprint_budget() {
    let source = reduce_to_eight(native_two_by_two(true), &blocks());
    let mut by_root: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for s in &source.stems {
        by_root.entry((s.x, s.z)).or_default().push((s.min_y, s.max_y));
    }
    assert_eq!(
        by_root.len(),
        4,
        "two light runs must not discard two physical trunk columns"
    );
    for ranges in by_root.values_mut() {
        ranges.sort_unstable();
        let mut end = (64 - MIN_Y) as u16;
        for &(lo, hi) in ranges.iter() {
            assert!(lo <= end, "a light split opened a hole in the stem");
            end = end.max(hi);
        }
        assert_eq!(end, (90 - MIN_Y) as u16);
    }
}

fn visible_column(source: &FullDataSource, x: i32, z: i32) -> Vec<(u32, i32, i32)> {
    source
        .column(x, z)
        .iter()
        .map(|&p| {
            (
                source.mapping.state(full_point::id(p)),
                full_point::bottom_y(p),
                full_point::top_y(p),
            )
        })
        .collect()
}

#[test]
fn metadata_only_arrival_and_removal_signal_appearance_changes() {
    let blocks = blocks();
    let (mut stored, stem) = one_crown();
    let before = visible_column(&stored, 0, 0);
    let mut input = stored.clone();
    input.stems.push(stem);
    assert!(
        stored.update_from(&input, &blocks),
        "stem-only arrival must invalidate the mesh"
    );
    assert_eq!(visible_column(&stored, 0, 0), before);
    assert_eq!(stored.stems.len(), 1);
    assert!(
        !stored.update_from(&input, &blocks),
        "identical metadata must not churn meshes"
    );
    input.stems.clear();
    assert!(
        stored.update_from(&input, &blocks),
        "stem-only removal must invalidate the mesh"
    );
    assert!(stored.stems.is_empty());
    assert_eq!(visible_column(&stored, 0, 0), before);
}

#[test]
fn newer_actual_data_removes_prediction_and_late_same_level_preview_cannot_restore_it() {
    let blocks = blocks();
    for step in [gen_step::FEATURES, gen_step::LIGHT] {
        let (mut stored, stem) = one_crown();
        stored.stems.push(stem);
        let old_preview = stored.clone();
        let mut actual = FullDataSource::empty(stored.pos);
        put(&mut actual, 1, 1, &[(GROUND, MIN_Y, 64, 15)], step);
        assert!(stored.update_from(&actual, &blocks));
        assert!(stored.stems.is_empty());
        let before = visible_column(&stored, 0, 0);
        assert!(!stored.update_from(&old_preview, &blocks));
        assert!(stored.stems.is_empty());
        assert_eq!(stored.gen_steps[dh::index(0, 0)], step);
        assert_eq!(visible_column(&stored, 0, 0), before);
    }
}

#[test]
fn late_preview_child_cannot_replace_an_actual_parent_or_resurrect_predicted_stems() {
    let blocks = blocks();
    let mut actual = FullDataSource::empty(pos::encode(6, 0, 0));
    let mut preview = FullDataSource::empty(actual.pos);
    for x in 0..2 {
        for z in 0..2 {
            let mut runs = vec![(GROUND, MIN_Y, 64, 15)];
            if x == 0 && z == 0 {
                runs.push((LOG, 64, 80, 15));
            }
            runs.push((LEAVES, 80, 84, 15));
            put(&mut actual, x, z, &runs, gen_step::LIGHT);
            crown(&mut preview, x, z, 64, 80, 84, gen_step::SURFACE);
        }
    }
    let predicted = predicted(&mut preview, 0, 0, 64, 80, 84);
    preview.stems.push(predicted);
    let mut parent = FullDataSource::empty(pos::encode(7, 0, 0));
    assert!(parent.update_from(&actual, &blocks));
    assert_eq!(parent.gen_steps[dh::index(0, 0)], gen_step::LIGHT);
    assert!(parent.stems.iter().all(|s| s.leaves == 0));
    let before = visible_column(&parent, 0, 0);
    let stems = parent.stems.clone();
    assert!(
        !parent.update_from(&preview, &blocks),
        "late SURFACE child must not overwrite LIGHT parent"
    );
    assert_eq!(parent.gen_steps[dh::index(0, 0)], gen_step::LIGHT);
    assert_eq!(visible_column(&parent, 0, 0), before);
    assert_eq!(parent.stems, stems);
}

#[test]
fn partial_updates_preserve_other_columns_stems() {
    let blocks = blocks();
    let (mut source, a) = one_crown();
    crown(&mut source, 9, 1, 64, 80, 84, gen_step::SURFACE);
    let b = predicted(&mut source, 9, 1, 64, 80, 84);
    source.stems.extend([a, b]);
    let mut input = FullDataSource::empty(source.pos);
    put(&mut input, 1, 1, &[(GROUND, MIN_Y, 64, 15)], gen_step::LIGHT);
    assert!(source.update_from(&input, &blocks));
    assert_eq!(source.stems.len(), 1);
    assert_eq!((source.stems[0].x, source.stems[0].z), (9, 1));
    assert_eq!(source.mapping.state(source.stems[0].id), LOG);
}

#[test]
fn duplicate_roots_and_input_order_do_not_change_the_footprint_limit() {
    let (mut a, template) = one_crown();
    for x in 0..8 {
        let s = Stem {
            x,
            z: 1,
            max_y: (70 + x - MIN_Y) as u16,
            ..template
        };
        a.stems.extend([s, s]);
    }
    let mut b = a.clone();
    b.stems.reverse();
    a.normalize_stems();
    b.normalize_stems();
    assert_eq!(a.stems, b.stems);
    assert_eq!(a.stems.len(), 4);
    assert_eq!(
        a.stems.iter().map(|s| s.x).collect::<BTreeSet<_>>(),
        BTreeSet::from([4, 5, 6, 7])
    );
}

#[test]
fn near_cutout_hides_only_the_covered_root_cell() {
    let blocks = blocks();
    let (mut source, a) = one_crown();
    crown(&mut source, 9, 1, 64, 80, 84, gen_step::SURFACE);
    let b = predicted(&mut source, 9, 1, 64, 80, 84);
    source.stems.extend([a, b]);
    let mut quads = Vec::new();
    dh::append_stem_quads(&source, &blocks, MIN_Y, |x, z| x == 0 && z == 0, &mut quads);
    assert!(!quads.is_empty());
    assert_one_block_faces(&quads);
    for q in &quads {
        match q.dir {
            Dir::West | Dir::East => assert!((9..=10).contains(&q.plane)),
            _ => assert!(q.u0 >= 9 && q.u1 <= 10),
        }
    }
    quads.clear();
    dh::append_stem_quads(&source, &blocks, MIN_Y, |_, _| true, &mut quads);
    assert!(quads.is_empty());
}

#[test]
fn codec_roundtrip_keeps_stem_metadata_and_rejects_corruption() {
    let (mut source, stem) = one_crown();
    source.stems.push(stem);
    let bytes = storage::encode(&source, 73);
    assert_eq!(storage::decode(&bytes, 73), Some(source.clone()));
    assert!(storage::decode(&bytes, 74).is_none());
    assert!(storage::decode(&bytes[..bytes.len() / 2], 73).is_none());
    let mut old = bytes.clone();
    old[..4].copy_from_slice(b"DHH1");
    assert!(storage::decode(&old, 73).is_none());
    for bad in [
        Stem { id: u32::MAX, ..stem },
        Stem {
            min_y: stem.max_y,
            ..stem
        },
        Stem { sky: 16, ..stem },
        Stem { x: 512, ..stem },
        Stem {
            max_y: 6200,
            crown_max: 6201,
            ..stem
        },
        Stem {
            crown_max: stem.max_y,
            ..stem
        },
    ] {
        let mut invalid = source.clone();
        invalid.stems = vec![bad];
        assert!(
            storage::decode(&storage::encode(&invalid, 73), 73).is_none(),
            "invalid stem {bad:?}"
        );
    }
    let mut raw = Vec::new();
    flate2::read::ZlibDecoder::new(&bytes[12..])
        .read_to_end(&mut raw)
        .unwrap();
    raw.push(123);
    let mut encoder = flate2::write::ZlibEncoder::new(bytes[..12].to_vec(), flate2::Compression::fast());
    encoder.write_all(&raw).unwrap();
    assert!(
        storage::decode(&encoder.finish().unwrap(), 73).is_none(),
        "unexpected payload trailing bytes"
    );
}
