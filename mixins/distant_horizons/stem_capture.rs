// Renderer-only, per-call metadata. No world chunks or native Column layout change.
// Part of Distant Horizons for Horizon Client, LGPL-3.0-only (see runtime.rs).
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

pub fn is_stem(name: &str) -> bool {
    let name = name.strip_prefix("minecraft:").unwrap_or(name);
    name.ends_with("_log") || name.ends_with("_wood") || name.ends_with("_hyphae")
        || matches!(name, "crimson_stem" | "warped_stem" | "stripped_crimson_stem"
            | "stripped_warped_stem" | "mushroom_stem")
}

#[derive(Clone, Copy, Debug)]
pub struct Candidate {
    pub x: i32,
    pub z: i32,
    pub min: i32,
    pub max: i32,
    pub crown_max: i32,
    pub log: u32,
    pub leaves: u32,
    pub biome: u16,
    pub sky: u8,
    pub block: u8,
    pub exact: bool,
}

thread_local! {
    static CAPTURE: RefCell<Option<Vec<Candidate>>> = const { RefCell::new(None) };
}

/// Must stay on its originating worker. Drop cancels metadata after a panic/error.
pub struct Capture(PhantomData<Rc<()>>);
impl Capture {
    pub fn start() -> Self {
        CAPTURE.with(|slot| {
            assert!(slot.borrow().is_none(), "nested DH stem capture");
            *slot.borrow_mut() = Some(Vec::new());
        });
        Self(PhantomData)
    }
    pub fn finish(self) -> Vec<Candidate> {
        CAPTURE.with(|slot| slot.borrow_mut().take().unwrap_or_default())
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        CAPTURE.with(|slot| *slot.borrow_mut() = None);
    }
}
pub fn enabled() -> bool {
    CAPTURE.with(|slot| slot.borrow().is_some())
}
pub fn record(candidate: Candidate) {
    CAPTURE.with(|slot| {
        if let Some(out) = slot.borrow_mut().as_mut() {
            out.push(candidate);
        }
    });
}
