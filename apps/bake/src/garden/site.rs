//! The garden's site plan: the lie of its land, the shape of its pond, where its paths run and
//! where its beds are dug. Everything else in the garden asks this before it plants or builds.
//!
//! The pond is a basin in the terrain, shaped by a distance field: two lobes joined at a waist
//! that a footbridge crosses, a bay to the south-west and an inlet to the north-east. Its water
//! is a level sheet at Y = 0 over the basin's rectangle (`StillWater`), and the banks, which
//! stand above it everywhere outside the field, give it its outline. A shallow shelf runs round
//! the margin, as garden ponds are dug, for plants that stand in water.
//!
//! X runs east and Z south, in metres, and the garden is entered from the south.

use glam::{Vec2, Vec3};
use solstice_procgen::noise::fbm;

/// Edge of the terrain, in metres.
pub const SIZE: f32 = 360.0;
/// Height of the pond's water.
pub const WATER_LEVEL: f32 = 0.0;
/// How far out from the water's edge the marginal shelf runs, and how deep it lies.
pub const SHELF_WIDTH: f32 = 1.3;
pub const SHELF_DEPTH: f32 = 0.28;
/// Depth of the pond's middle.
pub const POND_DEPTH: f32 = 1.45;
/// Width of the gravel paths.
pub const PATH_WIDTH: f32 = 1.5;
/// Where the footbridge crosses the pond's waist, east to west.
pub const BRIDGE_X: f32 = 2.2;

pub fn smoothstep(low: f32, high: f32, x: f32) -> f32 {
    let t = ((x - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The smaller of two distances, rounded where they meet over `k` metres.
fn smooth_min(a: f32, b: f32, k: f32) -> f32 {
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

/// The distance from `p` to the segment from `a` to `b`.
pub fn to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let along = b - a;
    let t = ((p - a).dot(along) / along.length_squared().max(1.0e-6)).clamp(0.0, 1.0);
    p.distance(a + along * t)
}

/// A dug bed for garden plants: a rounded shape, mulched, with nothing but its own planting.
#[derive(Clone, Copy, Debug)]
pub struct Bed {
    pub from: Vec2,
    pub to: Vec2,
    pub radius: f32,
    pub kind: BedKind,
}

/// What a bed is planted with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BedKind {
    /// A sunny mixed border: lavender in front, taller flowers behind.
    Border,
    /// Shade planting under a tree: ferns and hostas.
    Shade,
}

/// A tree that stands alone in the garden, placed by hand.
#[derive(Clone, Copy, Debug)]
pub struct Specimen {
    pub at: Vec2,
    pub species: &'static str,
    /// Which grown copy of the species.
    pub variant: u32,
    pub scale: f32,
    pub yaw: f32,
}

/// The plan of the whole garden.
pub struct Site {
    /// The paths' centre lines, as points close together.
    paths: Vec<Vec<Vec2>>,
    pub beds: Vec<Bed>,
    pub specimens: Vec<Specimen>,
    /// Where the bridge meets each bank: south, then north.
    pub bridge: (Vec2, Vec2),
}

/// A smooth curve through `points`, as points about `spacing` apart.
fn spline(points: &[Vec2], spacing: f32) -> Vec<Vec2> {
    let mut out = Vec::new();
    let at = |i: isize| points[i.clamp(0, points.len() as isize - 1) as usize];
    for i in 0..points.len() as isize - 1 {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        let steps = (p1.distance(p2) / spacing).ceil().max(1.0) as usize;
        for k in 0..steps {
            let t = k as f32 / steps as f32;
            let (t2, t3) = (t * t, t * t * t);
            // Catmull-Rom.
            out.push(
                0.5 * ((2.0 * p1)
                    + (-p0 + p2) * t
                    + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
                    + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3),
            );
        }
    }
    out.push(*points.last().expect("a path has points"));
    out
}

impl Default for Site {
    fn default() -> Self {
        Self::new()
    }
}

impl Site {
    pub fn new() -> Self {
        let v = Vec2::new;
        // The bridge runs north to south across the waist, from bank to bank.
        let shore = |from: f32, to: f32| {
            let mut z = from;
            let step = (to - from).signum() * 0.02;
            while pond_distance(v(BRIDGE_X, z)) > 0.0 && (z - to).abs() > 0.03 {
                z += step;
            }
            z
        };
        let south = v(BRIDGE_X, shore(8.0, 0.0) + 0.55);
        let north = v(BRIDGE_X, shore(-8.0, 0.0) - 0.55);

        let paths = vec![
            // In from the south gate, between the borders, to the bridge.
            spline(
                &[
                    v(6.0, 70.0),
                    v(3.5, 52.0),
                    v(0.4, 38.0),
                    v(3.4, 25.0),
                    v(1.2, 14.5),
                    v(BRIDGE_X, south.y + 3.0),
                    south,
                ],
                0.25,
            ),
            // On from the bridge to the seat under the great oak, and away into the wood.
            spline(
                &[
                    north,
                    v(BRIDGE_X, north.y - 2.5),
                    v(3.6, -10.5),
                    v(7.2, -14.6),
                    v(13.5, -19.0),
                    v(23.0, -24.5),
                    v(36.0, -33.0),
                    v(52.0, -46.0),
                    v(70.0, -62.0),
                ],
                0.25,
            ),
            // Round the west lobe: from the south path, by the bay and the great willow, and back
            // to the path north of the bridge.
            spline(
                &[
                    v(1.5, 15.2),
                    v(-5.0, 14.6),
                    v(-12.0, 15.6),
                    v(-18.6, 12.2),
                    v(-21.4, 4.5),
                    v(-21.0, -4.5),
                    v(-17.5, -11.0),
                    v(-10.5, -14.2),
                    v(-3.0, -12.6),
                    v(3.2, -10.2),
                ],
                0.25,
            ),
        ];

        let bed = |from: Vec2, to: Vec2, radius: f32, kind: BedKind| Bed {
            from,
            to,
            radius,
            kind,
        };
        let beds = vec![
            // The long borders either side of the way in.
            bed(v(7.6, 33.0), v(7.4, 21.0), 2.3, BedKind::Border),
            bed(v(-3.6, 34.5), v(-2.6, 24.5), 2.1, BedKind::Border),
            // A drift by the bay, in the sun.
            bed(v(-8.0, 19.0), v(-13.5, 19.6), 1.7, BedKind::Border),
            // East of the bridge, between path and water.
            bed(v(6.4, 12.4), v(10.4, 11.4), 1.5, BedKind::Border),
            // Under the great oak, round the seat.
            bed(v(10.5, -17.5), v(13.5, -14.5), 2.4, BedKind::Shade),
            bed(v(2.6, -17.6), v(-1.5, -17.0), 2.0, BedKind::Shade),
            // Under the oak by the west walk.
            bed(v(-25.5, -9.0), v(-25.0, -15.0), 2.2, BedKind::Shade),
        ];

        let tree =
            |x: f32, z: f32, species: &'static str, variant: u32, scale: f32, yaw: f32| Specimen {
                at: v(x, z),
                species,
                variant,
                scale,
                yaw,
            };
        let specimens = vec![
            // Willows on the banks, their withes over the water.
            tree(-16.9, -4.0, "willow", 0, 1.0, 0.4),
            tree(17.2, -3.4, "willow", 1, 0.9, 2.3),
            tree(-9.9, 13.0, "willow", 2, 0.74, 4.1),
            // The great oak over the seat, and others standing about the lawns.
            tree(7.5, -20.5, "old_oak", 2, 1.22, 1.1),
            tree(-27.0, -12.0, "old_oak", 1, 0.9, 3.0),
            tree(27.5, 9.0, "old_oak", 2, 0.86, 5.2),
            tree(-26.0, 20.5, "old_oak", 1, 0.8, 0.3),
            tree(16.5, 30.0, "old_oak", 0, 0.72, 2.2),
            // A stand of birches east of the pond.
            tree(24.5, -12.5, "birch", 0, 1.0, 0.0),
            tree(27.0, -15.6, "birch", 1, 0.9, 1.7),
            tree(22.6, -17.2, "birch", 0, 0.82, 3.9),
            tree(29.4, -10.4, "birch", 1, 0.76, 5.0),
        ];
        Self {
            paths,
            beds,
            specimens,
            bridge: (south, north),
        }
    }

    /// How far `p` is from the middle of the nearest path.
    pub fn path_distance(&self, p: Vec2) -> f32 {
        let mut nearest = f32::MAX;
        for path in &self.paths {
            for pair in path.windows(2) {
                // Whole stretches of path are far away: only the near ones need measuring.
                if (pair[0] - p).abs().max_element() < nearest + 1.0 {
                    nearest = nearest.min(to_segment(p, pair[0], pair[1]));
                }
            }
        }
        nearest
    }

    /// How far inside the nearest bed `p` lies, in metres: negative outside them all. Each bed's
    /// edge wanders a little, as a spade leaves it.
    pub fn bed_depth(&self, p: Vec2) -> (f32, Option<BedKind>) {
        let wander = fbm(Vec3::new(p.x * 0.45, 5.0, p.y * 0.45), 3, 811) * 0.45;
        let mut best = (f32::MIN, None);
        for bed in &self.beds {
            let inside = bed.radius - to_segment(p, bed.from, bed.to) + wander;
            if inside > best.0 {
                best = (inside, Some(bed.kind));
            }
        }
        best
    }

    /// How much of the ground at `p` is gravel and how much bare soil, the rest being turf: the
    /// paths are gravel, with the shore's shingle in places; the beds, the pond's bed and the
    /// ground worn bare under the densest shade are soil.
    pub fn ground_shares(&self, p: Vec2) -> Vec2 {
        let pond = pond_distance(p);
        let ragged = fbm(Vec3::new(p.x * 0.9, 3.0, p.y * 0.9), 3, 821) * 0.22;
        let half = PATH_WIDTH * 0.5;
        let mut gravel = 1.0 - smoothstep(half - 0.22, half + 0.16, self.path_distance(p) + ragged);
        // Shingle along the water's edge, from just above the water to the shelf's rim, in
        // drifts rather than a band.
        let drift = fbm(Vec3::new(p.x * 0.11, 9.0, p.y * 0.11), 3, 823);
        // A narrow beach at the waterline only: the bed below is dark silt.
        let shingle = smoothstep(-0.1, 0.25, drift)
            * (1.0 - smoothstep(0.1, 0.8, pond))
            * smoothstep(-0.7, -0.15, pond);
        gravel = gravel.max(shingle * 0.9);
        // Bare earth: the pond's bed and the wet margin, and the beds.
        let margin = 1.0 - smoothstep(0.05, 0.55, pond + ragged);
        let bed = smoothstep(-0.12, 0.2, self.bed_depth(p).0);
        // Under the wood little grows: earth and leaf mould, with turf in the lighter places.
        let shade = wood_density(p)
            * smoothstep(
                -0.35,
                0.25,
                fbm(Vec3::new(p.x * 0.2, 15.0, p.y * 0.2), 3, 825),
            );
        let soil = margin.max(bed).max(shade * 0.85) * (1.0 - gravel);
        Vec2::new(gravel, soil)
    }

    /// Whether lawn or meadow may grow at `p`, `reach` metres across: on dry land, off the
    /// paths, and out of the beds.
    pub fn may_grow_grass(&self, p: Vec2, reach: f32) -> bool {
        pond_distance(p) > 0.35 + reach
            && self.path_distance(p) > PATH_WIDTH * 0.5 + 0.05 + reach
            && self.bed_depth(p).0 < -0.1 - reach
    }
}

/// How far `p` is outside the pond's edge, in metres: negative over water.
pub fn pond_distance(p: Vec2) -> f32 {
    let v = Vec2::new;
    let circle = |centre: Vec2, radius: f32| p.distance(centre) - radius;
    let west = circle(v(-6.6, 0.4), 8.3);
    let east = circle(v(9.2, 0.2), 6.1);
    let bay = circle(v(-11.6, 7.2), 4.3);
    let inlet = circle(v(13.4, -5.2), 3.1);
    let waist = to_segment(p, v(-1.5, 0.3), v(5.5, 0.3)) - 2.1;
    let mut d = smooth_min(west, east, 2.5);
    d = smooth_min(d, bay, 2.5);
    d = smooth_min(d, inlet, 2.0);
    d = smooth_min(d, waist, 1.5);
    // The bank wanders, broadly and finely, so no stretch of it is an arc.
    d + fbm(Vec3::new(p.x * 0.16, 0.0, p.y * 0.16), 3, 801) * 1.1
        + fbm(Vec3::new(p.x * 0.7, 2.0, p.y * 0.7), 3, 803) * 0.28
}

/// How thick the wood is at a spot, from none in the garden to full beyond its rim: the rim
/// wanders in and out, in bays and points, and the glade is longer north to south than wide.
pub fn wood_density(p: Vec2) -> f32 {
    let wander = fbm(Vec3::new(p.x * 0.02, 13.0, p.y * 0.02), 3, 851) * 9.0;
    let far = p.length() + wander;
    let rim = 43.0 + 9.0 * (p.y / p.length().max(1.0)).abs();
    smoothstep(rim, rim + 16.0, far)
}

/// The rectangle the pond's water is laid over: least x and z, greatest x and z.
pub fn pond_bounds() -> [f32; 4] {
    [-22.0, -14.0, 23.0, 17.0]
}

/// The land's own roll, before the pond is dug into it: lawns that rise and fall a little, and
/// low hills beyond them that close the garden in.
fn land(x: f32, z: f32) -> f32 {
    let p = Vec3::new(x, 0.0, z);
    let lawn = fbm(p * 0.022, 4, 831) * 1.1 + fbm(p * 0.075, 3, 833) * 0.22;
    let far = (x * x + z * z).sqrt();
    let hills = smoothstep(46.0, 150.0, far) * (8.5 + 6.0 * fbm(p * 0.011 + Vec3::Y * 4.0, 4, 835));
    // The ground falls a little towards the south gate, so the pond is seen from above on the
    // way in.
    let fall = smoothstep(10.0, 70.0, z) * 0.9;
    lawn.max(-0.25) + hills + fall
}

/// The height of the ground at a spot, with the pond dug and the banks made.
pub fn ground_height(x: f32, z: f32) -> f32 {
    let p = Vec2::new(x, z);
    let pond = pond_distance(p);
    // Fine unevenness everywhere, so nothing is ruled flat.
    let rough = fbm(Vec3::new(x * 0.9, 0.0, z * 0.9), 3, 837) * 0.018;
    if pond <= 0.0 {
        let d = -pond;
        // A shallow margin for the plants that stand in water, deepening from the first step
        // in and evenly on down to the middle: no ledge, so the bed fades from sight gradually.
        let shelf = SHELF_DEPTH * smoothstep(0.0, 0.55, d);
        let deep = (POND_DEPTH - SHELF_DEPTH) * smoothstep(0.25, SHELF_WIDTH + 4.6, d);
        let bed = fbm(Vec3::new(x * 0.5, 7.0, z * 0.5), 3, 839) * 0.06 * smoothstep(0.3, 2.0, d);
        WATER_LEVEL - shelf - deep + bed + rough * smoothstep(0.0, 0.5, d)
    } else {
        // A bank that rises quickly from the water, then eases into the lawn.
        let bank = 0.34 * smoothstep(0.0, 1.5, pond) + 0.06 * smoothstep(0.0, 0.25, pond);
        WATER_LEVEL + bank + land(x, z) * smoothstep(1.0, 16.0, pond) + rough
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_banks_stand_above_the_water_everywhere_inside_its_rectangle() {
        let [x0, z0, x1, z1] = pond_bounds();
        let mut wet = 0;
        for i in 0..400 {
            for j in 0..400 {
                let x = x0 + (x1 - x0) * i as f32 / 399.0;
                let z = z0 + (z1 - z0) * j as f32 / 399.0;
                let outside = pond_distance(Vec2::new(x, z));
                let height = ground_height(x, z);
                if outside > 0.3 {
                    assert!(
                        height > WATER_LEVEL + 0.03,
                        "a puddle at {x}, {z}: {height}"
                    );
                }
                if outside < 0.0 {
                    wet += 1;
                }
                // The water never reaches the rectangle's edge, where it would end in a line.
                if i == 0 || j == 0 || i == 399 || j == 399 {
                    assert!(outside > 1.0, "the pond reaches its rectangle at {x}, {z}");
                }
            }
        }
        assert!(
            wet > 400 * 400 / 5,
            "the pond is small: {wet} of 160000 samples"
        );
    }

    #[test]
    fn the_bridge_spans_the_waist_from_bank_to_bank() {
        let site = Site::new();
        let (south, north) = site.bridge;
        assert!(pond_distance(south) > 0.2 && pond_distance(north) > 0.2);
        let span = south.distance(north);
        assert!((3.5..9.0).contains(&span), "the bridge is {span} m long");
        assert!(pond_distance((south + north) * 0.5) < -1.0);
    }

    #[test]
    fn nothing_planted_by_hand_stands_in_the_water_or_on_a_path() {
        let site = Site::new();
        for tree in &site.specimens {
            println!(
                "{} at {}: {:.2} m from the water, {:.2} m from a path, ground {:.2}",
                tree.species,
                tree.at,
                pond_distance(tree.at),
                site.path_distance(tree.at),
                ground_height(tree.at.x, tree.at.y)
            );
        }
        println!("bridge {:?}", site.bridge);
        for tree in &site.specimens {
            assert!(pond_distance(tree.at) > 1.2, "{} in the pond", tree.species);
            assert!(
                site.path_distance(tree.at) > PATH_WIDTH * 0.5 + 0.6,
                "{} at {} on a path",
                tree.species,
                tree.at
            );
        }
    }
}
