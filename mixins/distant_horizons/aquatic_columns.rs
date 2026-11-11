// Part of Distant Horizons for Horizon Client, LGPL-3.0-only (see runtime.rs).
// Preserve native aquatic model states in fluid columns, without changing
// their water volume, surface metadata, biome or already calculated lighting.
use crate::lod::{Column, Span};
use horizon_world::registry::BlockStateId;
use std::{cell::Cell, marker::PhantomData, rc::Rc};

thread_local! {
    static IN_NATIVE: Cell<u8> = const { Cell::new(0) };
}

/// A cancellable HEAD calls the untouched native body once, then postprocesses
/// it. The recursive HEAD sees its bit and lets the body run normally. Different
/// hooks may nest, and unwinding must not leave a thread permanently bypassed.
pub struct Guard {
    flag: u8,
    // The guard must drop on the same thread whose native call it guards.
    _thread: PhantomData<Rc<()>>,
}

pub fn enter(bit: u8) -> Option<Guard> {
    assert!(bit < 6, "aquatic native hook bit out of range");
    let flag = 1 << bit;
    IN_NATIVE.with(|active| {
        if active.get() & flag != 0 {
            return None;
        }
        active.set(active.get() | flag);
        Some(Guard {
            flag,
            _thread: PhantomData,
        })
    })
}

impl Drop for Guard {
    fn drop(&mut self) {
        IN_NATIVE.with(|active| active.set(active.get() & !self.flag));
    }
}

pub const SUMMARY_REVISION: i32 = 1;
pub const SUMMARY_KEY: &str = "DormyuAquaticColumns";

/// These four blocks have a native plant model in addition to their fluid.
/// Bubble columns and arbitrary waterlogged blocks deliberately stay unchanged.
pub fn is_aquatic(name: &str) -> bool {
    matches!(
        name,
        "minecraft:kelp" | "minecraft:kelp_plant" | "minecraft:seagrass" | "minecraft:tall_seagrass"
    )
}

/// Only inspect cells already represented as water. No allocation or callback
/// on dry columns; the empty Vec does not allocate on plant-free water either.
pub fn changes(
    column: &Column,
    water: BlockStateId,
    mut original: impl FnMut(i32) -> Option<BlockStateId>,
) -> Vec<(i16, BlockStateId)> {
    let mut changes = Vec::new();
    for span in &column.spans {
        if span.state != water {
            continue;
        }
        for y in span.min..span.max {
            if let Some(state) = original(i32::from(y))
                && state != water
            {
                changes.push((y, state));
            }
        }
    }
    changes
}

/// Apply sorted, unique native cells to water spans. Splitting retains the
/// precise light/biome of each original span, even at a section/light boundary.
/// The original surface still describes the floor plus continuous fluid top.
pub fn restore(column: &mut Column, water: BlockStateId, changes: &[(i16, BlockStateId)]) {
    if changes.is_empty() {
        return;
    }
    let mut rebuilt = Column {
        surface: column.surface,
        ..Column::default()
    };
    let mut cursor = 0;
    for span in &column.spans {
        while cursor < changes.len() && changes[cursor].0 < span.min {
            cursor += 1;
        }
        if span.state != water {
            rebuilt.push(*span);
            continue;
        }
        let mut min = span.min;
        while cursor < changes.len() && changes[cursor].0 < span.max {
            let (y, state) = changes[cursor];
            if y > min {
                rebuilt.push(Span { min, max: y, ..*span });
            }
            rebuilt.push(Span {
                min: y,
                max: y + 1,
                state,
                ..*span
            });
            min = y + 1;
            cursor += 1;
        }
        if min < span.max {
            rebuilt.push(Span { min, ..*span });
        }
    }
    *column = rebuilt;
}

/// Salt only the native derived-column cache, never real saved chunks. This
/// rejects plantless .hlod entries made before these preservation hooks.
pub fn cache_key(value: u64) -> u64 {
    // A bijection: different original engine/registry keys remain different.
    value ^ 0x6468_6171_7561_0001
}

/// f42ef42's lower nibble is KEPT | FLUID | AIR | WATERY. Clear only WATERY
/// for these four names: Column::from_chunk then retains their original IDs,
/// while both native fluid/surface processing and DH still see fluid coverage.
pub fn retain_kinds(facts: &crate::blocks::BlockFacts, kinds: &mut [u8]) {
    for (index, kind) in kinds.iter_mut().enumerate() {
        if *kind & 8 != 0 && is_aquatic(&facts.get(BlockStateId(index as u32)).name) {
            *kind &= !8;
        }
    }
}
