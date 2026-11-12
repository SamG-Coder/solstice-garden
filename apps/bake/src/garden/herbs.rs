//! The garden's flowers and foliage plants, grown as real geometry like the engine's grass and
//! trees: every leaf and petal a few triangles of its own shape.
//!
//! Each kind of plant is a small recipe built from the same few parts: a leaf (which is also a
//! petal), a stem, a bell, a flower head. All of them share one atlas of sixteen columns, each a
//! leaf or petal from its base (v = 0) to its tip (v = 1), so the whole planting is one texture
//! and two materials: one that sways, and one for the lily pads, which float.

use glam::{Quat, Vec2, Vec3};
use solstice_builder::{Mesh, Vertex};
use solstice_procgen::foliage::Rng;
use solstice_procgen::noise::tiling_fbm;
use solstice_procgen::surface::{Surface, Texel};
use std::f32::consts::{PI, TAU};

/// Columns in the atlas.
pub const COLUMNS: u32 = 16;

/// The atlas's columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paint {
    Leaf = 0,
    GreyLeaf = 1,
    Hosta = 2,
    Fern = 3,
    Sword = 4,
    Pad = 5,
    Stem = 6,
    Violet = 7,
    Blue = 8,
    Foxglove = 9,
    White = 10,
    Red = 11,
    Yellow = 12,
    Purple = 13,
    Pink = 14,
    Brown = 15,
}

fn smoothstep(low: f32, high: f32, x: f32) -> f32 {
    let t = ((x - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The atlas: what each column looks like at `across` (-1..1) and `v` from base to tip.
pub fn atlas(seed: u32) -> Surface {
    Surface::new(0.004, move |u, v| {
        let column = ((u * COLUMNS as f32) as u32).min(COLUMNS - 1);
        let across = ((u * COLUMNS as f32).fract() - 0.5) / 0.45;
        let d = across.abs().min(1.0);
        let mottle = tiling_fbm(u, v, 48, 3, seed + column);
        let grain = tiling_fbm(u, v, 160, 2, seed + 31 + column);
        let midrib = (-(across * 10.0).powi(2)).exp();
        // Side veins leaving the midrib towards the tip, `count` pairs along the leaf.
        let side_veins = |count: f32| {
            let phase = v * count - d * 2.0;
            (-((phase - phase.round()) * 7.0).powi(2)).exp() * (1.0 - d)
        };
        // Veins running the leaf's length, as on a blade or a hosta.
        let ribs = |count: f32| 0.5 + 0.5 * (across * count * PI).cos();
        let leaf = |color: Vec3, veins: f32, roughness: f32| {
            let lines = side_veins(veins).max(midrib);
            let mut c = color * (1.0 + 0.2 * mottle + 0.08 * grain);
            c = c.lerp(c * Vec3::new(1.55, 1.45, 1.2), lines * 0.45);
            c *= 1.0 - 0.15 * smoothstep(0.7, 1.0, d);
            Texel {
                color: c,
                height: -lines,
                roughness,
                cavity: 0.7 + 0.3 * v,
            }
        };
        // A petal: one colour at its base, another at its tip, silky, faintly lined.
        let petal = |base: Vec3, tip: Vec3, sheen: f32| {
            let lines = 0.5 + 0.5 * (across * 9.0 * PI).cos();
            let c = base.lerp(tip, smoothstep(0.05, 0.6, v))
                * (0.92 + 0.1 * lines)
                * (1.0 + 0.1 * mottle);
            Texel {
                color: c,
                height: lines * 0.3,
                roughness: sheen,
                cavity: 0.75 + 0.25 * smoothstep(0.0, 0.4, v),
            }
        };
        match column {
            0 => leaf(Vec3::new(0.045, 0.1, 0.022), 7.0, 0.5),
            1 => leaf(Vec3::new(0.095, 0.125, 0.085), 5.0, 0.7),
            2 => {
                // Hosta: blue-green, deeply ribbed along its length, with a paler margin.
                let rib = ribs(9.0);
                let mut c =
                    Vec3::new(0.04, 0.095, 0.06) * (0.85 + 0.25 * rib) * (1.0 + 0.15 * mottle);
                c = c.lerp(Vec3::new(0.2, 0.26, 0.12), smoothstep(0.78, 0.98, d) * 0.7);
                Texel {
                    color: c,
                    height: rib,
                    roughness: 0.45,
                    cavity: 0.7 + 0.3 * rib,
                }
            }
            3 => leaf(Vec3::new(0.05, 0.125, 0.024), 12.0, 0.55),
            4 => {
                // A sword leaf: parallel veins, a pale base and a tip that dries tan.
                let rib = ribs(7.0);
                let mut c = Vec3::new(0.055, 0.115, 0.035)
                    * (0.9 + 0.15 * rib + 0.3 * midrib)
                    * (1.0 + 0.25 * mottle);
                c = c.lerp(
                    Vec3::new(0.13, 0.15, 0.07),
                    (1.0 - smoothstep(0.0, 0.12, v)) * 0.7,
                );
                c = c.lerp(Vec3::new(0.24, 0.19, 0.09), smoothstep(0.9, 1.0, v) * 0.7);
                Texel {
                    color: c,
                    height: rib * 0.5 + midrib,
                    roughness: 0.5,
                    cavity: 0.5 + 0.5 * smoothstep(0.0, 0.5, v),
                }
            }
            5 => {
                // A lily pad, read round its middle: veins radiating from the stalk, a waxy dark
                // green, bronze at the rim. `across` is the angle and `v` the radius.
                let spokes = 0.5 + 0.5 * (across * 11.0 * PI).cos();
                let vein = smoothstep(0.82, 1.0, spokes) * smoothstep(0.05, 0.3, v);
                let mut c = Vec3::new(0.03, 0.085, 0.02) * (1.0 + 0.22 * mottle);
                c = c.lerp(Vec3::new(0.09, 0.16, 0.05), vein * 0.6);
                c = c.lerp(Vec3::new(0.11, 0.07, 0.03), smoothstep(0.86, 1.0, v) * 0.55);
                Texel {
                    color: c,
                    height: -vein,
                    roughness: 0.22,
                    cavity: 1.0,
                }
            }
            6 => Texel {
                color: Vec3::new(0.06, 0.11, 0.03) * (1.0 + 0.2 * mottle),
                height: 0.0,
                roughness: 0.6,
                cavity: 0.6 + 0.4 * v,
            },
            7 => petal(Vec3::new(0.2, 0.15, 0.4), Vec3::new(0.33, 0.24, 0.62), 0.6),
            8 => petal(
                Vec3::new(0.16, 0.2, 0.55),
                Vec3::new(0.26, 0.33, 0.78),
                0.55,
            ),
            9 => {
                // A foxglove bell from its neck to its mouth: rose outside, paler within the
                // lip and spotted with maroon ringed in white.
                let mut t = petal(Vec3::new(0.42, 0.1, 0.3), Vec3::new(0.62, 0.22, 0.48), 0.5);
                let spots = smoothstep(0.45, 0.62, tiling_fbm(u, v, 96, 2, seed + 77))
                    * smoothstep(0.55, 0.85, v);
                t.color = t
                    .color
                    .lerp(Vec3::new(0.85, 0.72, 0.78), smoothstep(0.6, 1.0, v) * 0.45);
                t.color = t.color.lerp(Vec3::new(0.2, 0.02, 0.08), spots * 0.8);
                t
            }
            10 => petal(Vec3::new(0.7, 0.72, 0.6), Vec3::new(0.86, 0.86, 0.82), 0.5),
            11 => {
                // A poppy's petal: scarlet, crumpled silk, black at the base.
                let mut t = petal(
                    Vec3::new(0.5, 0.03, 0.015),
                    Vec3::new(0.7, 0.07, 0.025),
                    0.42,
                );
                t.color = t.color.lerp(
                    Vec3::new(0.02, 0.012, 0.02),
                    1.0 - smoothstep(0.08, 0.26, v),
                );
                t.height += grain * 0.6;
                t
            }
            12 => petal(Vec3::new(0.6, 0.4, 0.01), Vec3::new(0.82, 0.66, 0.03), 0.5),
            13 => petal(
                Vec3::new(0.26, 0.09, 0.42),
                Vec3::new(0.44, 0.2, 0.66),
                0.55,
            ),
            14 => petal(
                Vec3::new(0.82, 0.62, 0.66),
                Vec3::new(0.78, 0.36, 0.5),
                0.45,
            ),
            _ => Texel {
                // A reedmace's spike: brown velvet.
                color: Vec3::new(0.085, 0.042, 0.018) * (1.0 + 0.3 * grain + 0.15 * mottle),
                height: grain,
                roughness: 0.95,
                cavity: 0.8,
            },
        }
    })
}

/// Builds a plant's mesh from its parts.
#[derive(Default)]
pub struct Builder {
    pub mesh: Mesh,
}

/// Where on the atlas a point of a part lies: its column, and how far across it (-1..1).
fn u(paint: Paint, across: f32) -> f32 {
    (paint as u32 as f32 + 0.5 + across.clamp(-1.0, 1.0) * 0.45) / COLUMNS as f32
}

/// A unit vector perpendicular to `axis`, turned `angle` around it.
fn perpendicular(axis: Vec3, angle: f32) -> Vec3 {
    let reference = if axis.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    Quat::from_axis_angle(axis, angle) * axis.cross(reference).normalize()
}

/// A leaf's or a petal's outline and bearing.
#[derive(Clone, Copy)]
pub struct Blade {
    pub length: f32,
    /// Width as a fraction of length.
    pub width: f32,
    /// The outline: half the width at `s` along goes as `s^a (1 - s)^b`.
    pub shape: (f32, f32),
    /// Full cross-sections between base and tip.
    pub rows: usize,
    /// How far it bends over from base to tip, in radians: away from its face if positive, as
    /// a leaf droops; towards it if negative, as a petal cups.
    pub arch: f32,
    /// Deepest of the fold along the midrib, as a fraction of half the width.
    pub fold: f32,
    pub paint: Paint,
}

impl Builder {
    /// A leaf or petal from `base` along `axis`, its upper face turned as nearly towards `face`
    /// as its axis allows.
    pub fn blade(&mut self, base: Vec3, axis: Vec3, face: Vec3, blade: &Blade) {
        let axis = axis.normalize_or(Vec3::Y);
        let face = (face - axis * axis.dot(face)).normalize_or(perpendicular(axis, 0.0));
        let side = axis.cross(face).normalize();
        let (a, b) = blade.shape;
        let peak = a / (a + b);
        let most = peak.powf(a) * (1.0 - peak).powf(b);
        let half = |s: f32| blade.length * blade.width * 0.5 * s.powf(a) * (1.0 - s).powf(b) / most;
        // The midrib's curve: marched in short steps, turning about the leaf's side.
        let steps = blade.rows * 2 + 2;
        let frame = |s: f32| {
            let turn = Quat::from_axis_angle(side, -blade.arch * s);
            (turn * axis, turn * face)
        };
        let mut point = base;
        let mut along = vec![(base, axis, face)];
        for k in 1..=steps {
            let s = k as f32 / steps as f32;
            let (direction, _) = frame(s - 0.5 / steps as f32);
            point += direction * (blade.length / steps as f32);
            let (direction, normal) = frame(s);
            along.push((point, direction, normal));
        }
        // Lit as a mass rather than leaf by leaf: normals lean towards straight up.
        let lit = |n: Vec3| (n + Vec3::Y * 0.3).normalize();
        let first = self.mesh.vertices.len() as u32;
        self.mesh.vertices.push(Vertex {
            position: base,
            normal: lit(face),
            uv: Vec2::new(u(blade.paint, 0.0), 0.0),
        });
        for row in 1..=blade.rows {
            let (centre, _, normal) = along[row * 2];
            let s = row as f32 * 2.0 / steps as f32;
            let w = half(s);
            for across in [-1.0, 0.0, 1.0] {
                let position = centre + side * (across * w)
                    - normal * (w * blade.fold * (1.0 - across * across));
                self.mesh.vertices.push(Vertex {
                    position,
                    normal: lit((normal - side * (across * 0.35)).normalize()),
                    uv: Vec2::new(u(blade.paint, across), s),
                });
            }
        }
        let (tip, _, normal) = along[steps];
        self.mesh.vertices.push(Vertex {
            position: tip,
            normal: lit(normal),
            uv: Vec2::new(u(blade.paint, 0.0), 1.0),
        });
        // Counter-clockwise seen from the face.
        let row = |k: u32| (first + 1 + k * 3, first + 2 + k * 3, first + 3 + k * 3);
        let count = blade.rows as u32;
        let point = first + 1 + count * 3;
        let (l0, c0, r0) = row(0);
        self.mesh
            .indices
            .extend_from_slice(&[first, c0, l0, first, r0, c0]);
        for k in 0..count - 1 {
            let (l0, c0, r0) = row(k);
            let (l1, c1, r1) = row(k + 1);
            self.mesh
                .indices
                .extend_from_slice(&[l0, c0, c1, l0, c1, l1, c0, r0, r1, c0, r1, c1]);
        }
        let (l1, c1, r1) = row(count - 1);
        self.mesh
            .indices
            .extend_from_slice(&[l1, c1, point, c1, r1, point]);
    }

    /// A stem through `points`, three-sided, tapering from `radius` to `tip_radius`.
    pub fn stem(&mut self, points: &[Vec3], radius: f32, tip_radius: f32, paint: Paint) {
        self.tube(points, &|t| radius + (tip_radius - radius) * t, 3, paint);
    }

    /// A tube through `points`, `sides` round, whose radius at `t` along is `radius(t)`.
    pub fn tube(&mut self, points: &[Vec3], radius: &dyn Fn(f32) -> f32, sides: u32, paint: Paint) {
        let first = self.mesh.vertices.len() as u32;
        let rings = points.len();
        let mut normal = perpendicular((points[1] - points[0]).normalize_or(Vec3::Y), 0.0);
        for (i, &point) in points.iter().enumerate() {
            let tangent = if i + 1 < rings {
                points[i + 1] - point
            } else {
                point - points[i - 1]
            }
            .normalize_or(Vec3::Y);
            normal =
                (normal - tangent * normal.dot(tangent)).normalize_or(perpendicular(tangent, 0.0));
            let binormal = tangent.cross(normal);
            let t = i as f32 / (rings - 1) as f32;
            for j in 0..=sides {
                let theta = j as f32 / sides as f32 * TAU;
                let out = normal * theta.cos() + binormal * theta.sin();
                self.mesh.vertices.push(Vertex {
                    position: point + out * radius(t),
                    normal: (out + Vec3::Y * 0.2).normalize(),
                    uv: Vec2::new(u(paint, j as f32 / sides as f32 * 2.0 - 1.0), t),
                });
            }
        }
        let row = sides + 1;
        for i in 0..rings as u32 - 1 {
            for j in 0..sides {
                let a = first + i * row + j;
                let (b, c, d) = (a + 1, a + row + 1, a + row);
                self.mesh.indices.extend_from_slice(&[a, b, c, a, c, d]);
            }
        }
    }

    /// A flower's middle: a low dome facing `facing`, `v` running from its top to its rim.
    pub fn dome(&mut self, centre: Vec3, facing: Vec3, radius: f32, height: f32, paint: Paint) {
        const SEGMENTS: u32 = 8;
        let turn = Quat::from_rotation_arc(Vec3::Y, facing.normalize_or(Vec3::Y));
        let first = self.mesh.vertices.len() as u32;
        self.mesh.vertices.push(Vertex {
            position: centre + facing * height,
            normal: facing,
            uv: Vec2::new(u(paint, 0.0), 0.75),
        });
        for i in 0..=SEGMENTS {
            let a = i as f32 / SEGMENTS as f32 * TAU;
            let out = turn * Vec3::new(a.cos(), 0.0, a.sin());
            self.mesh.vertices.push(Vertex {
                position: centre + out * radius,
                normal: (facing + out).normalize(),
                uv: Vec2::new(u(paint, i as f32 / SEGMENTS as f32 * 2.0 - 1.0), 1.0),
            });
        }
        for i in 0..SEGMENTS {
            self.mesh
                .indices
                .extend_from_slice(&[first, first + 2 + i, first + 1 + i]);
        }
    }

    /// Petals in a ring about `centre`, round the axis `facing`, each rising `lift` radians
    /// from flat.
    #[allow(clippy::too_many_arguments)]
    pub fn ring(
        &mut self,
        rng: &mut Rng,
        centre: Vec3,
        facing: Vec3,
        count: u32,
        lift: f32,
        start: f32,
        petal: &Blade,
    ) {
        let facing = facing.normalize_or(Vec3::Y);
        let turn = rng.range(0.0, TAU);
        for k in 0..count {
            let angle = turn + (k as f32 + rng.signed() * 0.18) / count as f32 * TAU;
            let out = perpendicular(facing, angle);
            let axis = out * lift.cos() + facing * lift.sin();
            let blade = Blade {
                length: petal.length * rng.range(0.88, 1.1),
                ..*petal
            };
            self.blade(centre + out * start, axis, facing, &blade);
        }
    }

    /// A bell hanging from `neck` along `axis`: a tube that opens from its neck to its mouth.
    pub fn bell(&mut self, neck: Vec3, axis: Vec3, length: f32, mouth: f32, paint: Paint) {
        let axis = axis.normalize_or(Vec3::NEG_Y);
        let points = [neck, neck + axis * (length * 0.45), neck + axis * length];
        self.tube(
            &points,
            &|t| mouth * (0.3 + 0.55 * t.sqrt() + 0.25 * t * t),
            5,
            paint,
        );
    }

    /// A lily pad floating at `centre`, with its notch towards `turn`.
    pub fn pad(&mut self, centre: Vec3, radius: f32, turn: f32, rng: &mut Rng) {
        const SEGMENTS: u32 = 18;
        // The notch takes a twelfth of the round.
        let gap = TAU / 12.0;
        let first = self.mesh.vertices.len() as u32;
        self.mesh.vertices.push(Vertex {
            position: centre,
            normal: Vec3::Y,
            uv: Vec2::new(u(Paint::Pad, 0.0), 0.0),
        });
        let wave = rng.range(0.0, TAU);
        for ring in 0..2 {
            let far = if ring == 0 { 0.6 } else { 1.0 };
            for i in 0..=SEGMENTS {
                let f = i as f32 / SEGMENTS as f32;
                let a = turn + gap * 0.5 + f * (TAU - gap);
                // The rim is wavy and turned up a little, more beside the notch.
                let lift = if ring == 1 {
                    radius
                        * (0.035
                            + 0.03 * (a * 5.0 + wave).sin()
                            + 0.06 * (1.0 - (f * 2.0 - 1.0).abs()).powi(8))
                } else {
                    0.0
                };
                let out = Vec3::new(a.cos(), 0.0, a.sin());
                let wobble = 1.0 + 0.05 * (a * 3.0 + wave * 1.7).sin();
                self.mesh.vertices.push(Vertex {
                    position: centre + out * (radius * far * wobble) + Vec3::Y * lift,
                    normal: (Vec3::Y - out * (lift / radius) * 4.0).normalize(),
                    uv: Vec2::new(u(Paint::Pad, f * 2.0 - 1.0), far),
                });
            }
        }
        let row = SEGMENTS + 1;
        for i in 0..SEGMENTS {
            let (a, b) = (first + 1 + i, first + 2 + i);
            self.mesh.indices.extend_from_slice(&[first, b, a]);
            let (c, d) = (a + row, b + row);
            self.mesh.indices.extend_from_slice(&[a, b, d, a, d, c]);
        }
    }
}

/// A direction `elevation` radians above level, towards `azimuth`.
fn aim(azimuth: f32, elevation: f32) -> Vec3 {
    Vec3::new(
        azimuth.cos() * elevation.cos(),
        elevation.sin(),
        azimuth.sin() * elevation.cos(),
    )
}

/// A stem's line from `base`: `length` long, leaving at `direction` and bowing over by `bow`
/// radians towards level as it goes, in `segments` pieces.
fn stalk(base: Vec3, direction: Vec3, length: f32, bow: f32, segments: usize) -> Vec<Vec3> {
    let direction = direction.normalize_or(Vec3::Y);
    let level = Vec3::new(direction.x, 0.0, direction.z).normalize_or(Vec3::X);
    let side = level.cross(Vec3::Y).normalize_or(Vec3::Z);
    let mut points = vec![base];
    let mut point = base;
    for k in 0..segments {
        let s = (k as f32 + 0.5) / segments as f32;
        let turned = Quat::from_axis_angle(side, -bow * s * s) * direction;
        point += turned * (length / segments as f32);
        points.push(point);
    }
    points
}

/// The point and direction `t` of the way along a line of points.
fn along(points: &[Vec3], t: f32) -> (Vec3, Vec3) {
    let segments = points.len() - 1;
    let x = (t.clamp(0.0, 1.0) * segments as f32).min(segments as f32 - 1.0e-4);
    let i = x as usize;
    (
        points[i].lerp(points[i + 1], x - i as f32),
        (points[i + 1] - points[i]).normalize_or(Vec3::Y),
    )
}

/// A plant that is a mound of foliage carrying spikes of small flowers: lavender, catmint,
/// salvia, and, with few tall stems, delphinium.
#[derive(Clone, Copy)]
pub struct Spiked {
    /// How far out the mound's foliage reaches, and how high.
    pub radius: f32,
    pub height: f32,
    /// Leafy shoots in the mound, and leaves on each.
    pub shoots: u32,
    pub leaves: u32,
    pub leaf: Blade,
    /// Flowering stems: how many, how long, and how far they splay from upright (radians).
    pub stems: (u32, u32),
    pub stem_length: (f32, f32),
    pub splay: f32,
    /// The spike at each stem's end: its length, its whorls, the florets in each and their size.
    /// The spike itself is a slender body of buds and calyces, `core` thick at its middle,
    /// which the open florets stand out from.
    pub spike: f32,
    pub core: f32,
    /// For flowers large enough to see each one, as a delphinium's: the length of the stalk
    /// each stands out on, where it opens as a ring of `florets` petals about a pale eye, three
    /// such flowers to a whorl. 0 for small flowers, whose `florets` petals leave the stem.
    pub stalk: f32,
    pub whorls: u32,
    pub florets: u32,
    pub floret: Blade,
}

impl Spiked {
    pub fn grow(&self, seed: u32) -> Mesh {
        let mut rng = Rng::new(seed, 41);
        let mut b = Builder::default();
        for _ in 0..self.shoots {
            let azimuth = rng.range(0.0, TAU);
            let reach = rng.unit().sqrt();
            // Shoots at the rim lie out; those in the middle stand.
            let elevation = (1.35 - reach * 0.95) * rng.range(0.85, 1.1);
            let base = Vec3::new(azimuth.cos(), 0.0, azimuth.sin()) * (reach * self.radius * 0.35);
            let line = stalk(
                base,
                aim(azimuth, elevation),
                self.height * rng.range(0.7, 1.15),
                0.5,
                3,
            );
            b.stem(&line, 0.0022, 0.001, Paint::Stem);
            let mut spin = rng.range(0.0, TAU);
            for k in 0..self.leaves {
                let (at, direction) = along(&line, (k as f32 + 0.6) / self.leaves as f32);
                spin += 2.4;
                let out = perpendicular(direction, spin);
                let blade = Blade {
                    length: self.leaf.length * rng.range(0.75, 1.2),
                    ..self.leaf
                };
                b.blade(at, (direction * 0.7 + out).normalize(), direction, &blade);
            }
        }
        let stems = self.stems.0 + rng.index((self.stems.1 - self.stems.0 + 1) as usize) as u32;
        for _ in 0..stems {
            let azimuth = rng.range(0.0, TAU);
            let lean = rng.unit().sqrt() * self.splay;
            let base = Vec3::new(azimuth.cos(), 0.0, azimuth.sin())
                * (lean / self.splay.max(0.01) * self.radius * 0.3);
            let length = rng.range(self.stem_length.0, self.stem_length.1);
            let line = stalk(base, aim(azimuth, FRAC_UP - lean), length, 0.25, 4);
            b.stem(&line, 0.001 + length * 0.0007, 0.0007, Paint::Stem);
            // The spike: whorls of florets up the stem's last stretch, closest at the top.
            let share = (self.spike / length).min(0.8);
            if self.core > 0.0 {
                let body: Vec<Vec3> = (0..=5)
                    .map(|k| along(&line, 1.0 - share * (1.0 - k as f32 / 5.0)).0)
                    .collect();
                let core = self.core * rng.range(0.85, 1.15);
                // Closed at its foot, fullest a third of the way up, and drawn to a point.
                b.tube(
                    &body,
                    &|t| core * (t * 4.0).min(1.0).sqrt() * (1.0 - t * t).max(0.0).sqrt(),
                    5,
                    self.floret.paint,
                );
            }
            let mut spin = rng.range(0.0, TAU);
            for w in 0..self.whorls {
                let f = (w as f32 + 0.5) / self.whorls as f32;
                let (at, direction) = along(&line, 1.0 - share * (1.0 - f.powf(0.85)));
                spin += 0.9;
                // Florets shrink towards the spike's tip, where they are still in bud.
                let size = 1.0 - 0.45 * f * f;
                if self.stalk > 0.0 {
                    // Whole flowers, each on its own stalk, facing out and a little up.
                    for k in 0..3 {
                        let out = perpendicular(
                            direction,
                            spin + k as f32 / 3.0 * TAU + rng.signed() * 0.3,
                        );
                        let facing = (out + direction * 0.35).normalize();
                        let centre = at + facing * (self.stalk * size);
                        b.stem(&[at, centre], 0.0008, 0.0006, Paint::Stem);
                        let one = Blade {
                            length: self.floret.length * size,
                            ..self.floret
                        };
                        b.ring(&mut rng, centre, facing, self.florets, 0.3, 0.001, &one);
                        b.dome(
                            centre,
                            facing,
                            self.floret.length * 0.22 * size,
                            0.002,
                            Paint::White,
                        );
                    }
                    continue;
                }
                for k in 0..self.florets {
                    let out = perpendicular(direction, spin + k as f32 / self.florets as f32 * TAU);
                    let blade = Blade {
                        length: self.floret.length * size * rng.range(0.85, 1.15),
                        ..self.floret
                    };
                    b.blade(at, (out + direction * 0.55).normalize(), direction, &blade);
                }
            }
        }
        b.mesh
    }
}

/// Straight up, as an elevation.
const FRAC_UP: f32 = std::f32::consts::FRAC_PI_2;

/// A simple leaf of the given size.
pub fn leaf(length: f32, width: f32, paint: Paint) -> Blade {
    Blade {
        length,
        width,
        shape: (0.6, 0.9),
        rows: 2,
        arch: 0.5,
        fold: 0.25,
        paint,
    }
}

/// A small flat petal or floret.
pub fn petal(length: f32, width: f32, paint: Paint) -> Blade {
    Blade {
        length,
        width,
        shape: (0.7, 0.6),
        rows: 1,
        arch: 0.3,
        fold: 0.15,
        paint,
    }
}

/// A basal tuft: `count` leaves in a ring from the ground, rising `lift` radians.
fn tuft(b: &mut Builder, rng: &mut Rng, count: u32, lift: (f32, f32), blade: &Blade) {
    let turn = rng.range(0.0, TAU);
    for k in 0..count {
        let azimuth = turn + (k as f32 + rng.signed() * 0.3) / count as f32 * TAU;
        let axis = aim(azimuth, rng.range(lift.0, lift.1));
        let one = Blade {
            length: blade.length * rng.range(0.7, 1.15),
            ..*blade
        };
        b.blade(Vec3::ZERO, axis, Vec3::Y, &one);
    }
}

/// A foxglove: a rosette of broad leaves and one or more tall spires hung with bells down one
/// side. `paint` is the bells' colour.
pub fn foxglove(seed: u32, paint: Paint) -> Mesh {
    let mut rng = Rng::new(seed, 43);
    let mut b = Builder::default();
    tuft(
        &mut b,
        &mut rng,
        9,
        (0.15, 0.6),
        &leaf(0.22, 0.36, Paint::Leaf),
    );
    let spires = 1 + rng.index(3) as u32;
    for n in 0..spires {
        let azimuth = rng.range(0.0, TAU);
        let height = rng.range(0.95, 1.55) * if n == 0 { 1.0 } else { 0.75 };
        let base = Vec3::new(azimuth.cos(), 0.0, azimuth.sin()) * 0.04 * n as f32;
        let line = stalk(
            base,
            aim(azimuth, FRAC_UP - rng.range(0.03, 0.14)),
            height,
            0.12,
            6,
        );
        b.stem(&line, 0.006, 0.002, Paint::Stem);
        // Smaller leaves up the lower stem.
        let mut spin = rng.range(0.0, TAU);
        for k in 0..6 {
            let (at, direction) = along(&line, 0.08 + 0.07 * k as f32);
            spin += 2.4;
            let out = perpendicular(direction, spin);
            b.blade(
                at,
                (out + direction * 0.5).normalize(),
                direction,
                &leaf(0.13 - 0.012 * k as f32, 0.32, Paint::Leaf),
            );
        }
        // The bells face one way, opening from the bottom of the spire up.
        let front = rng.range(0.0, TAU);
        let bells = (height * 0.5 / 0.024) as u32;
        for k in 0..bells {
            let f = k as f32 / bells as f32;
            let (at, direction) = along(&line, 0.5 + 0.49 * f);
            let out = perpendicular(direction, front + rng.signed() * 1.1);
            // Open bells hang; the buds above them still point up.
            let hang = 0.5 - 1.3 * smoothstep(0.7, 1.0, f);
            let size = 1.0 - 0.75 * smoothstep(0.55, 1.0, f);
            b.bell(
                at + out * 0.006,
                out - direction * hang,
                0.046 * size,
                0.0105 * size,
                paint,
            );
        }
    }
    b.mesh
}

/// A clump of daisies: a tuft of leaves, and stems each carrying a head of white rays about a
/// yellow middle.
pub fn daisy(seed: u32) -> Mesh {
    let mut rng = Rng::new(seed, 47);
    let mut b = Builder::default();
    tuft(
        &mut b,
        &mut rng,
        14,
        (0.3, 1.0),
        &leaf(0.12, 0.2, Paint::Leaf),
    );
    let stems = 9 + rng.index(9) as u32;
    for _ in 0..stems {
        let azimuth = rng.range(0.0, TAU);
        let lean = rng.unit().sqrt() * 0.42;
        let line = stalk(
            Vec3::ZERO,
            aim(azimuth, FRAC_UP - lean),
            rng.range(0.42, 0.78),
            0.15,
            4,
        );
        b.stem(&line, 0.0024, 0.0014, Paint::Stem);
        let mut spin = rng.range(0.0, TAU);
        for k in 0..4 {
            let (at, direction) = along(&line, 0.15 + 0.17 * k as f32);
            spin += 2.4;
            b.blade(
                at,
                (perpendicular(direction, spin) + direction).normalize(),
                direction,
                &leaf(0.05, 0.22, Paint::Leaf),
            );
        }
        let (top, direction) = along(&line, 1.0);
        // Heads look up and a little the way their stem leans.
        let facing = (Vec3::Y * 0.8 + direction).normalize();
        let ray = Blade {
            arch: 0.25,
            shape: (0.5, 0.35),
            ..petal(rng.range(0.022, 0.03), 0.3, Paint::White)
        };
        b.ring(&mut rng, top, facing, 17, 0.08, 0.006, &ray);
        b.dome(top + facing * 0.001, facing, 0.0085, 0.005, Paint::Yellow);
    }
    b.mesh
}

/// A clump of poppies: divided leaves below, and on thin stems cups of four scarlet petals.
pub fn poppy(seed: u32) -> Mesh {
    let mut rng = Rng::new(seed, 53);
    let mut b = Builder::default();
    tuft(
        &mut b,
        &mut rng,
        16,
        (0.35, 1.1),
        &leaf(0.16, 0.28, Paint::GreyLeaf),
    );
    let stems = 6 + rng.index(8) as u32;
    for _ in 0..stems {
        let azimuth = rng.range(0.0, TAU);
        let lean = rng.unit().sqrt() * 0.5;
        let line = stalk(
            Vec3::ZERO,
            aim(azimuth, FRAC_UP - lean),
            rng.range(0.45, 0.85),
            0.2,
            4,
        );
        b.stem(&line, 0.002, 0.0013, Paint::Stem);
        let (top, direction) = along(&line, 1.0);
        let facing = (Vec3::Y + direction * 0.6).normalize();
        if rng.unit() < 0.8 {
            // Open: four broad petals, two a little inside the others, cupped.
            let cup = Blade {
                length: rng.range(0.04, 0.052),
                width: 1.15,
                shape: (0.55, 0.3),
                rows: 2,
                arch: -0.7,
                fold: -0.3,
                paint: Paint::Red,
            };
            b.ring(&mut rng, top, facing, 4, 0.35, 0.002, &cup);
            b.dome(top, facing, 0.007, 0.008, Paint::Brown);
        } else {
            // In bud, nodding.
            let (neck, _) = along(&line, 0.97);
            b.bell(
                neck,
                Vec3::NEG_Y + direction * 0.3,
                0.022,
                0.006,
                Paint::GreyLeaf,
            );
        }
    }
    b.mesh
}

/// Alliums: strap leaves at the ground and straight bare stems, each ending in a globe of
/// small purple stars.
pub fn allium(seed: u32) -> Mesh {
    let mut rng = Rng::new(seed, 59);
    let mut b = Builder::default();
    let strap = Blade {
        length: 0.32,
        width: 0.09,
        shape: (0.3, 0.6),
        rows: 3,
        arch: 1.3,
        fold: 0.4,
        paint: Paint::Sword,
    };
    tuft(&mut b, &mut rng, 7, (0.6, 1.2), &strap);
    let stems = 3 + rng.index(4) as u32;
    for n in 0..stems {
        let azimuth = rng.range(0.0, TAU);
        let base = Vec3::new(azimuth.cos(), 0.0, azimuth.sin()) * (0.05 * n as f32);
        let line = stalk(
            base,
            aim(azimuth, FRAC_UP - rng.range(0.0, 0.09)),
            rng.range(0.75, 1.1),
            0.05,
            3,
        );
        b.stem(&line, 0.004, 0.003, Paint::Stem);
        let (top, _) = along(&line, 1.0);
        let radius = rng.range(0.04, 0.055);
        let centre = top + Vec3::Y * radius * 0.6;
        let star = petal(0.014, 0.3, Paint::Purple);
        for k in 0..46 {
            // Spread evenly over the globe by the golden angle.
            let y = 1.0 - (k as f32 + 0.5) / 46.0 * 1.9;
            let ring = (1.0 - y * y).max(0.0).sqrt();
            let turn = k as f32 * 2.4;
            let out = Vec3::new(turn.cos() * ring, y, turn.sin() * ring);
            let at = centre + out * radius;
            let across = perpendicular(out, rng.range(0.0, TAU));
            b.blade(at, (across + out * 0.4).normalize(), out, &star);
            b.blade(at, (-across + out * 0.4).normalize(), out, &star);
        }
    }
    b.mesh
}

/// A clump of flag iris at the water's edge: fans of sword leaves, and stems carrying yellow
/// flowers of three falls and three standards.
pub fn iris(seed: u32) -> Mesh {
    let mut rng = Rng::new(seed, 61);
    let mut b = Builder::default();
    let fans = 4 + rng.index(4);
    for _ in 0..fans {
        let at = Vec3::new(rng.signed(), 0.0, rng.signed()) * 0.14;
        // Each fan's leaves lie in one plane, the outer ones leaning out.
        let plane = rng.range(0.0, TAU);
        let leaves = 5 + rng.index(4);
        for k in 0..leaves {
            let f = k as f32 / (leaves - 1) as f32 * 2.0 - 1.0;
            let axis = aim(
                plane + if f < 0.0 { PI } else { 0.0 },
                FRAC_UP - f.abs() * 0.38,
            );
            let sword = Blade {
                length: rng.range(0.7, 1.1) * (1.0 - 0.25 * f.abs()),
                width: 0.03,
                shape: (0.12, 0.55),
                rows: 4,
                arch: rng.range(0.1, 0.55),
                fold: 0.5,
                paint: Paint::Sword,
            };
            let face = aim(plane + if f < 0.0 { 0.0 } else { PI }, 0.2);
            b.blade(
                at + Vec3::new(plane.cos(), 0.0, plane.sin()) * (f * 0.03),
                axis,
                face,
                &sword,
            );
        }
    }
    let flowers = 2 + rng.index(4);
    for _ in 0..flowers {
        let azimuth = rng.range(0.0, TAU);
        let base = Vec3::new(rng.signed(), 0.0, rng.signed()) * 0.1;
        let line = stalk(
            base,
            aim(azimuth, FRAC_UP - rng.range(0.02, 0.16)),
            rng.range(0.75, 1.0),
            0.1,
            4,
        );
        b.stem(&line, 0.0045, 0.003, Paint::Stem);
        let (top, direction) = along(&line, 1.0);
        let fall = Blade {
            length: 0.062,
            width: 0.6,
            shape: (0.9, 0.45),
            rows: 2,
            arch: 1.7,
            fold: 0.2,
            paint: Paint::Yellow,
        };
        b.ring(&mut rng, top, direction, 3, 0.45, 0.003, &fall);
        let standard = Blade {
            length: 0.04,
            width: 0.4,
            arch: -0.4,
            ..fall
        };
        b.ring(&mut rng, top, direction, 3, 1.15, 0.002, &standard);
    }
    b.mesh
}

/// A stand of reedmace: tall narrow blades from the water, and stiff stems each with its brown
/// velvet spike.
pub fn reedmace(seed: u32) -> Mesh {
    let mut rng = Rng::new(seed, 67);
    let mut b = Builder::default();
    let blades = 22 + rng.index(14);
    for _ in 0..blades {
        let at = Vec3::new(rng.signed(), 0.0, rng.signed()) * 0.2;
        let azimuth = rng.range(0.0, TAU);
        let blade = Blade {
            length: rng.range(1.2, 2.0),
            width: 0.011,
            shape: (0.08, 0.5),
            rows: 4,
            arch: rng.range(0.1, 0.7),
            fold: 0.45,
            paint: Paint::Sword,
        };
        b.blade(
            at,
            aim(azimuth, FRAC_UP - rng.range(0.03, 0.22)),
            aim(azimuth + PI, 0.1),
            &blade,
        );
    }
    let spikes = 3 + rng.index(5);
    for _ in 0..spikes {
        let at = Vec3::new(rng.signed(), 0.0, rng.signed()) * 0.16;
        let azimuth = rng.range(0.0, TAU);
        let height = rng.range(1.3, 1.85);
        let line = stalk(
            at,
            aim(azimuth, FRAC_UP - rng.range(0.0, 0.08)),
            height,
            0.04,
            4,
        );
        b.stem(&line, 0.005, 0.0035, Paint::Stem);
        let (top, direction) = along(&line, 1.0);
        let length = rng.range(0.13, 0.2);
        let spike: Vec<Vec3> = (0..=4)
            .map(|k| top + direction * (length * k as f32 / 4.0))
            .collect();
        // Rounded at both ends.
        b.tube(
            &spike,
            &|t| 0.0115 * (1.0 - (t * 2.0 - 1.0).powi(6)).max(0.15),
            6,
            Paint::Brown,
        );
        let tip = [top + direction * length, top + direction * (length + 0.07)];
        b.stem(&tip, 0.0018, 0.0006, Paint::Stem);
    }
    b.mesh
}

/// A shuttlecock of fern fronds, each a midrib arching over, set with pairs of leaflets that
/// lengthen to a third of the way along and dwindle to the tip.
pub fn fern(seed: u32) -> Mesh {
    let mut rng = Rng::new(seed, 71);
    let mut b = Builder::default();
    let fronds = 9 + rng.index(6) as u32;
    let turn = rng.range(0.0, TAU);
    for n in 0..fronds {
        let azimuth = turn + (n as f32 + rng.signed() * 0.3) / fronds as f32 * TAU;
        let length = rng.range(0.55, 0.95);
        let line = stalk(
            Vec3::ZERO,
            aim(azimuth, rng.range(0.95, 1.3)),
            length,
            rng.range(1.2, 1.8),
            10,
        );
        b.stem(&line, 0.003, 0.0008, Paint::Stem);
        let pairs = (length / 0.028) as u32;
        for k in 0..pairs {
            let f = 0.14 + 0.85 * (k as f32 + 0.5) / pairs as f32;
            let (at, direction) = along(&line, f);
            // The frond's own plane: across it, and its upper face.
            let across = direction.cross(Vec3::Y).normalize_or(Vec3::X);
            let face = across.cross(direction).normalize();
            let s = (f - 0.14) / 0.86;
            let reach = 0.13 * length / 0.8 * (s.powf(0.45) * (1.0 - s).powf(0.8) / 0.56).min(1.0);
            let leaflet = Blade {
                length: reach.max(0.008),
                width: 0.2,
                shape: (0.25, 0.6),
                rows: 1,
                arch: 0.35,
                fold: 0.2,
                paint: Paint::Fern,
            };
            for sign in [1.0, -1.0] {
                b.blade(
                    at,
                    (across * sign + direction * 0.3).normalize(),
                    face,
                    &leaflet,
                );
            }
        }
    }
    b.mesh
}

/// A hosta: a mound of broad ribbed leaves on long stalks, and a few stems of pale lilac bells.
pub fn hosta(seed: u32) -> Mesh {
    let mut rng = Rng::new(seed, 73);
    let mut b = Builder::default();
    let leaves = 18 + rng.index(12) as u32;
    for k in 0..leaves {
        let azimuth = k as f32 * 2.4 + rng.signed() * 0.3;
        // The outer leaves lie out; the newest stand in the middle.
        let reach = ((k as f32 + 0.5) / leaves as f32).sqrt();
        let elevation = 1.25 - reach * 0.85;
        let line = stalk(
            Vec3::ZERO,
            aim(azimuth, elevation),
            rng.range(0.14, 0.26) * (0.6 + 0.6 * reach),
            0.3,
            3,
        );
        b.stem(&line, 0.004, 0.003, Paint::Stem);
        let (top, direction) = along(&line, 1.0);
        let blade = Blade {
            length: rng.range(0.16, 0.25),
            width: 0.7,
            shape: (0.55, 0.85),
            rows: 3,
            arch: rng.range(0.7, 1.2),
            fold: 0.3,
            paint: Paint::Hosta,
        };
        b.blade(top, direction, Vec3::Y, &blade);
    }
    for _ in 0..rng.index(4) {
        let azimuth = rng.range(0.0, TAU);
        let line = stalk(
            Vec3::ZERO,
            aim(azimuth, FRAC_UP - rng.range(0.05, 0.3)),
            rng.range(0.55, 0.8),
            0.3,
            5,
        );
        b.stem(&line, 0.003, 0.0015, Paint::Stem);
        for k in 0..9 {
            let (at, direction) = along(&line, 0.62 + 0.042 * k as f32);
            let out = perpendicular(direction, azimuth + rng.signed());
            b.bell(at, out - Vec3::Y * 0.5, 0.032, 0.008, Paint::Pink);
        }
    }
    b.mesh
}

/// A raft of water-lily pads, with a flower or two among them. Built floating at Y = 0.
pub fn water_lily(seed: u32, paint: Paint) -> Mesh {
    let mut rng = Rng::new(seed, 79);
    let mut b = Builder::default();
    let mut pads: Vec<(Vec2, f32)> = Vec::new();
    for _ in 0..60 {
        if pads.len() >= 9 {
            break;
        }
        let at = Vec2::from_angle(rng.range(0.0, TAU)) * (rng.unit().sqrt() * 0.62);
        let radius = rng.range(0.085, 0.16);
        // Pads crowd, and overlap a little at their rims, but none lies on another.
        if pads
            .iter()
            .all(|&(p, r)| p.distance(at) > (r + radius) * 0.86)
        {
            pads.push((at, radius));
        }
    }
    for (index, &(at, radius)) in pads.iter().enumerate() {
        // Each a hair higher than the last, so where rims overlap one is on top.
        let height = 0.004 + 0.0015 * index as f32;
        b.pad(
            Vec3::new(at.x, height, at.y),
            radius,
            rng.range(0.0, TAU),
            &mut rng,
        );
    }
    for _ in 0..1 + rng.index(2) {
        let at = Vec2::from_angle(rng.range(0.0, TAU)) * (rng.unit().sqrt() * 0.45);
        let centre = Vec3::new(at.x, 0.022, at.y);
        let facing = Vec3::Y;
        let outer = Blade {
            length: rng.range(0.05, 0.062),
            width: 0.4,
            shape: (0.6, 0.55),
            rows: 2,
            arch: -0.35,
            fold: 0.35,
            paint,
        };
        b.ring(&mut rng, centre, facing, 9, 0.22, 0.006, &outer);
        b.ring(
            &mut rng,
            centre + Vec3::Y * 0.004,
            facing,
            8,
            0.7,
            0.005,
            &Blade {
                length: outer.length * 0.85,
                ..outer
            },
        );
        b.ring(
            &mut rng,
            centre + Vec3::Y * 0.007,
            facing,
            6,
            1.15,
            0.004,
            &Blade {
                length: outer.length * 0.65,
                ..outer
            },
        );
        b.dome(
            centre + Vec3::Y * 0.008,
            facing,
            0.011,
            0.012,
            Paint::Yellow,
        );
    }
    b.mesh
}
