// Bounded water tint interpolation for the dormyu DH adaptation.
// Copyright (C) 2026 dormyu. LGPL-3.0-only; see LICENSE for the full terms.
// This module does not query/generate terrain or change stored world data.

use std::collections::HashMap;

/// A rectangle in LOD-cell coordinates. Colours follow the upward face's
/// SW, SE, NE, NW corner order. Ordinary uniform rectangles can still merge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TintedRect {
    pub bounds: [usize; 4],
    pub colors: [u32; 4],
}

/// Vertex colours from a one-cell halo of existing column colours. Equal-detail
/// neighbours evaluate the same four cells at a shared corner, including at a
/// four-section junction. Unknown data contributes nothing, never black.
pub struct WaterTintGrid {
    side: usize,
    colors: Vec<Option<u32>>,
}

/// A selected coarser tile's tint field, already stitched to any still-coarser
/// neighbours. Coordinates are world blocks; only its perimeter is sampled.
pub struct WaterTintSource {
    pub origin: [i32; 2],
    pub stride: i32,
    pub grid: WaterTintGrid,
}

impl WaterTintSource {
    fn edge_color(&self, [x, z]: [i32; 2]) -> Option<u32> {
        let dx = x - self.origin[0];
        let dz = z - self.origin[1];
        let end = self.stride * self.grid.side as i32;
        if !(0..=end).contains(&dx) || !(0..=end).contains(&dz)
            || (dx != 0 && dx != end && dz != 0 && dz != end)
        {
            return None;
        }
        let vertex = |x: i32, z: i32| self.grid.colors[z as usize * (self.grid.side + 1) + x as usize];
        if dx == 0 || dx == end {
            let cell = dz / self.stride;
            let offset = dz % self.stride;
            let a = vertex(dx / self.stride, cell)?;
            if offset == 0 { return Some(a); }
            Some(lerp_rgb(a, vertex(dx / self.stride, cell + 1)?, offset, self.stride))
        } else {
            let cell = dx / self.stride;
            let offset = dx % self.stride;
            let a = vertex(cell, dz / self.stride)?;
            if offset == 0 { return Some(a); }
            Some(lerp_rgb(a, vertex(cell + 1, dz / self.stride)?, offset, self.stride))
        }
    }
}

fn lerp_rgb(a: u32, b: u32, numerator: i32, denominator: i32) -> u32 {
    let (n, d) = (numerator as u32, denominator as u32);
    let channel = |shift: u32| {
        ((((a >> shift) & 255) * (d - n) + ((b >> shift) & 255) * n + d / 2) / d) << shift
    };
    channel(16) | channel(8) | channel(0)
}

impl WaterTintGrid {
    /// `sample` is called once per halo cell, in local cell coordinates -1..side.
    /// At most (side+2)^2 reads; storage is (side+1)^2 optional packed colours.
    pub fn new(side: usize, mut sample: impl FnMut(i32, i32) -> Option<u32>) -> Self {
        assert!((1..=64).contains(&side));
        let pitch = side + 2;
        let mut cells = Vec::with_capacity(pitch * pitch);
        for z in -1..=side as i32 {
            for x in -1..=side as i32 {
                cells.push(sample(x, z));
            }
        }
        let mut colors = Vec::with_capacity((side + 1) * (side + 1));
        for z in 0..=side {
            for x in 0..=side {
                colors.push(mean([
                    cells[z * pitch + x],
                    cells[z * pitch + x + 1],
                    cells[(z + 1) * pitch + x],
                    cells[(z + 1) * pitch + x + 1],
                ]));
            }
        }
        Self { side, colors }
    }

    pub fn vertex(&self, x: usize, z: usize, fallback: u32) -> u32 {
        assert!(x <= self.side && z <= self.side);
        self.colors[z * (self.side + 1) + x].unwrap_or(fallback & 0xFF_FFFF)
    }

    /// Match a finer boundary to the actual coarser mesh's piecewise-linear
    /// edge. Sources must be ordered coarsest first, with their own edges
    /// already stitched. Only boundary vertices change: no wider blur, terrain
    /// query or extra geometry. Packed RGB differs from the ideal GPU edge by
    /// at most half of one 8-bit unit at the inserted finer vertices.
    pub fn stitch(&mut self, origin: [i32; 2], stride: i32, coarser: &[WaterTintSource]) {
        for z in 0..=self.side {
            for x in 0..=self.side {
                if x != 0 && x != self.side && z != 0 && z != self.side { continue; }
                let point = [origin[0] + x as i32 * stride, origin[1] + z as i32 * stride];
                if let Some(color) = coarser.iter().filter(|s| s.stride > stride).find_map(|s| s.edge_color(point)) {
                    self.colors[z * (self.side + 1) + x] = Some(color);
                }
            }
        }
    }

    fn cell(&self, x: usize, z: usize, fallback: u32) -> [u32; 4] {
        [
            self.vertex(x, z + 1, fallback),
            self.vertex(x + 1, z + 1, fallback),
            self.vertex(x + 1, z, fallback),
            self.vertex(x, z, fallback),
        ]
    }

    /// Keep uniform runs as rectangles; only nonuniform transition cells need
    /// their own coloured face. Output never exceeds the input cell area.
    /// This prevents a large single-biome ocean from becoming thousands of quads.
    pub fn rectangles(&self, [x0, z0, x1, z1]: [usize; 4], fallback: u32) -> Vec<TintedRect> {
        assert!(x0 < x1 && x1 <= self.side && z0 < z1 && z1 <= self.side);
        let mut out: Vec<TintedRect> = Vec::new();
        let mut previous: HashMap<(usize, usize, u32), usize> = HashMap::new();
        for z in z0..z1 {
            let mut current = HashMap::new();
            let mut x = x0;
            while x < x1 {
                let colors = self.cell(x, z, fallback);
                let start = x;
                x += 1;
                if colors.iter().all(|&c| c == colors[0]) {
                    while x < x1 && self.cell(x, z, fallback) == colors {
                        x += 1;
                    }
                    let key = (start, x, colors[0]);
                    let index = if let Some(&index) = previous.get(&key) {
                        out[index].bounds[3] = z + 1;
                        index
                    } else {
                        out.push(TintedRect { bounds: [start, z, x, z + 1], colors });
                        out.len() - 1
                    };
                    current.insert(key, index);
                } else {
                    out.push(TintedRect { bounds: [start, z, x, z + 1], colors });
                }
            }
            previous = current;
        }
        out
    }
}

fn mean(values: [Option<u32>; 4]) -> Option<u32> {
    let mut sum = [0u32; 3];
    let mut count = 0;
    for value in values.into_iter().flatten() {
        for (i, shift) in [16, 8, 0].into_iter().enumerate() {
            sum[i] += (value >> shift) & 255;
        }
        count += 1;
    }
    (count > 0).then(|| ((sum[0] / count) << 16) | ((sum[1] / count) << 8) | (sum[2] / count))
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLD: u32 = 0x45_ADF2;
    const WARM: u32 = 0x43_D5EE;

    #[test]
    fn whole_uniform_ocean_remains_one_quad_without_rounding_drift() {
        let mut calls = 0;
        let grid = WaterTintGrid::new(64, |_, _| { calls += 1; Some(COLD) });
        assert_eq!(calls, 66 * 66);
        assert_eq!(grid.rectangles([0, 0, 64, 64], 0), vec![TintedRect {
            bounds: [0, 0, 64, 64], colors: [COLD; 4],
        }]);
    }

    #[test]
    fn known_ocean_boundary_has_a_local_midpoint_not_a_long_quad_gradient() {
        let grid = WaterTintGrid::new(64, |x, _| Some(if x < 32 { COLD } else { WARM }));
        assert_eq!(grid.vertex(31, 19, 0), COLD);
        assert_eq!(grid.vertex(32, 19, 0), 0x44_C1F0);
        assert_eq!(grid.vertex(33, 19, 0), WARM);
        let rectangles = grid.rectangles([0, 0, 64, 64], 0);
        // Two transition strips, one cell wide; two large uniform regions.
        assert_eq!(rectangles.len(), 64 * 2 + 2);
        assert_eq!(rectangles.iter().map(|r| (r.bounds[2]-r.bounds[0])*(r.bounds[3]-r.bounds[1])).sum::<usize>(), 4096);
    }

    #[test]
    fn neighboring_sections_and_four_way_negative_corner_share_exact_colors() {
        let at = |x: i32, z: i32| Some(if x - 2 * z < -17 { COLD } else { WARM });
        let grid = |bx, bz| WaterTintGrid::new(64, |x, z| at(bx + x, bz + z));
        let nw = grid(-128, -128);
        let ne = grid(-64, -128);
        let sw = grid(-128, -64);
        let se = grid(-64, -64);
        for i in 0..=64 {
            assert_eq!(nw.vertex(64, i, 0), ne.vertex(0, i, 0));
            assert_eq!(nw.vertex(i, 64, 0), sw.vertex(i, 0, 0));
        }
        assert_eq!(nw.vertex(64, 64, 0), se.vertex(0, 0, 0));
        assert_eq!(ne.vertex(0, 64, 0), sw.vertex(64, 0, 0));
    }

    #[test]
    fn unavailable_neighbors_do_not_blacken_valid_water_and_empty_uses_fallback() {
        let grid = WaterTintGrid::new(8, |x, z| ((0..8).contains(&x) && (0..8).contains(&z)).then_some(WARM));
        assert_eq!(grid.vertex(0, 0, 0), WARM);
        assert_eq!(grid.vertex(8, 8, 0), WARM);
        assert_eq!(grid.rectangles([0, 0, 8, 8], 0).len(), 1);
        let unknown = WaterTintGrid::new(8, |_, _| None);
        assert_eq!(unknown.vertex(3, 4, COLD), COLD);
    }

    #[test]
    fn arbitrary_boundaries_cover_every_cell_once_with_bounded_quad_count() {
        let grid = WaterTintGrid::new(64, |x, z| Some(((x.wrapping_mul(127) ^ z.wrapping_mul(97)) as u32) & 0xFF_FFFF));
        let rectangles = grid.rectangles([3, 7, 59, 63], COLD);
        assert!(rectangles.len() <= 56 * 56);
        let mut coverage = [0u8; 4096];
        for rectangle in rectangles {
            let [x0, z0, x1, z1] = rectangle.bounds;
            for z in z0..z1 {
                for x in x0..x1 { coverage[z*64+x] += 1; }
            }
        }
        for z in 0..64 {
            for x in 0..64 {
                assert_eq!(coverage[z*64+x], u8::from((3..59).contains(&x) && (7..63).contains(&z)));
            }
        }
    }

    #[test]
    fn fine_edge_matches_coarse_piecewise_linear_color_without_widening_the_blur() {
        let coarse = WaterTintSource {
            origin: [-64, -64], stride: 8,
            grid: WaterTintGrid::new(8, |_, z| Some(if z < 3 { COLD } else { WARM })),
        };
        let mut fine = WaterTintGrid::new(8, |_, z| Some(if z < 6 { COLD } else { WARM }));
        let interior: Vec<_> = (0..=8).map(|z| fine.vertex(1, z, 0)).collect();
        fine.stitch([0, -64], 4, std::slice::from_ref(&coarse));
        for z in 0..=8 {
            assert_eq!(fine.vertex(0, z, 0), coarse.edge_color([0, -64 + z as i32*4]).unwrap());
            assert_eq!(fine.vertex(1, z, 0), interior[z]);
        }
        // Half-way through a 5-unit change is rounded to an 8-bit value;
        // the bounded half-unit error is not a 5/10-unit coarse/fine seam.
        assert_eq!(lerp_rgb(0x44_C1F0, 0x43_D5EE, 1, 2), 0x44_CBEF);
    }

    #[test]
    fn recursive_corner_uses_coarsest_visible_neighbor_and_missing_data_is_not_black() {
        let largest = WaterTintSource {
            origin: [-128, -128], stride: 16,
            grid: WaterTintGrid::new(8, |_, _| Some(WARM)),
        };
        let mut middle = WaterTintSource {
            origin: [0, -64], stride: 8,
            grid: WaterTintGrid::new(8, |_, _| Some(COLD)),
        };
        middle.grid.stitch(middle.origin, middle.stride, std::slice::from_ref(&largest));
        let mut fine = WaterTintGrid::new(8, |_, _| Some(COLD));
        fine.stitch([0, 0], 4, &[largest, middle]);
        assert_eq!(fine.vertex(0, 0, 0), WARM);
        assert_eq!(fine.vertex(1, 1, 0), COLD);
        let unknown = WaterTintSource { origin: [-64, 0], stride: 8, grid: WaterTintGrid::new(8, |_, _| None) };
        fine.stitch([0, 0], 4, &[unknown]);
        assert_eq!(fine.vertex(0, 0, 0), WARM);
    }
}
