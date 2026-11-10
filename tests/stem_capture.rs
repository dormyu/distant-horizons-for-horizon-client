#[path = "../mixins/distant_horizons/stem_capture.rs"]
mod capture;

fn root(x: i32) -> capture::Candidate {
    capture::Candidate { x, z: 0, min: 64, max: 80, crown_max: 84, log: 1,
        leaves: 2, biome: 0, sky: 15, block: 0, exact: false }
}

#[test]
fn wood_capture_excludes_crops_with_similar_names() {
    for name in ["minecraft:oak_log", "stripped_jungle_log", "birch_wood", "warped_stem",
        "minecraft:stripped_crimson_stem", "warped_hyphae", "mushroom_stem"] {
        assert!(capture::is_stem(name), "{name}");
    }
    for name in ["minecraft:melon_stem", "pumpkin_stem", "attached_melon_stem",
        "attached_pumpkin_stem", "jungle_leaves", "stone", "kelp_plant"] {
        assert!(!capture::is_stem(name), "{name}");
    }
}

#[test]
fn finished_or_cancelled_batch_never_leaks_into_a_later_sampler() {
    capture::record(root(-1));
    let batch = capture::Capture::start();
    assert!(capture::enabled());
    capture::record(root(12));
    let records = batch.finish();
    assert_eq!(records.len(), 1);
    let got = records[0];
    assert_eq!((got.x, got.z, got.min, got.max, got.crown_max), (12, 0, 64, 80, 84));
    assert_eq!((got.log, got.leaves, got.biome, got.sky, got.block, got.exact), (1, 2, 0, 15, 0, false));
    assert!(!capture::enabled());
    {
        let _cancelled = capture::Capture::start();
        capture::record(root(99));
    }
    assert!(capture::Capture::start().finish().is_empty());
}

#[test]
fn panic_and_parallel_workers_preserve_capture_ownership() {
    let failure = std::panic::catch_unwind(|| {
        let _cancelled = capture::Capture::start();
        capture::record(root(99));
        panic!("cancel this sampler");
    });
    assert!(failure.is_err());
    let batch = capture::Capture::start();
    capture::record(root(1));
    let other = std::thread::spawn(|| {
        assert!(!capture::enabled());
        let batch = capture::Capture::start();
        capture::record(root(2));
        batch.finish()[0].x
    }).join().unwrap();
    assert_eq!(other, 2);
    assert_eq!(batch.finish()[0].x, 1);
}
