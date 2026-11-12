//! What is built of wood in the garden: the footbridge over the pond's waist, and the seat under
//! the great oak. Each is made board by board, as a carpenter would, of oak left to weather.

use super::site::{ground_height, Site, WATER_LEVEL};
use super::Garden;
use glam::{Mat4, Vec2, Vec3};
use solstice_asset::Material;
use solstice_builder::{Mesh, Vertex};
use solstice_procgen::field::{fbm, fbm_xy, smoothstep, Field};
use solstice_procgen::foliage::Rng;
use solstice_procgen::surface::{Shade, Surface};

/// Metres one tile of the timber's surface covers: a texel a millimetre at 1024.
const TIMBER_TILE_METRES: f32 = 1.0;

/// Oak boards a few years out of doors: silvered grey over brown, the grain raised where the
/// soft wood between has worn away, fine checks along it, and a green bloom where damp lingers.
/// The grain runs along v.
fn weathered_oak(seed: u32) -> Surface {
    let grain = fbm_xy(70, 3, 4, seed + 920);
    let rings = fbm_xy(18, 2, 3, seed + 921);
    let fibres = fbm_xy(260, 12, 2, seed + 922);
    let checks = fbm_xy(34, 2, 4, seed + 923);
    let presence = fbm(5, 3, seed + 924);
    let weather = fbm(4, 4, seed + 925).unit();
    let bloom = fbm(3, 4, seed + 926);
    let speckle = fbm(120, 2, seed + 927);
    Surface::shaped(
        move |u, v| {
            // Checks: narrow cracks along the grain, here and there.
            let line = 1.0 - (checks.at(u, v).abs() / 0.035).min(1.0);
            let check = line * line * smoothstep(0.05, 0.4, presence.at(u, v));
            let height =
                0.0006 * grain.at(u, v) + 0.0005 * rings.at(u, v) + 0.00015 * fibres.at(u, v)
                    - 0.0025 * check;
            (height, check)
        },
        move |u, v, check, relief| {
            let silver = weather.at(u, v);
            let wood =
                glam::Vec3::new(0.17, 0.125, 0.085).lerp(glam::Vec3::new(0.3, 0.275, 0.24), silver);
            // Late wood stands proud and darker; the worn early wood between is paler.
            let ridge = (relief.fine * 1800.0).clamp(-1.0, 1.0);
            let mut color = wood * (1.0 - 0.22 * ridge) * (1.0 + 0.12 * speckle.at(u, v));
            color = color.lerp(
                glam::Vec3::new(0.035, 0.028, 0.02),
                smoothstep(0.15, 0.7, check),
            );
            let green = smoothstep(0.15, 0.6, bloom.at(u, v)) * 0.4;
            color = color.lerp(glam::Vec3::new(0.085, 0.11, 0.05), green);
            Shade {
                color,
                roughness: 0.82 - 0.1 * silver,
                cavity: 1.0 - 0.7 * smoothstep(0.1, 0.8, check),
            }
        },
    )
    .with_tile_metres(TIMBER_TILE_METRES)
}

/// Builds things from sawn timbers.
struct Joiner {
    mesh: Mesh,
    rng: Rng,
}

impl Joiner {
    /// A face of four corners, in order round it, facing `normal`, with its texture coordinates
    /// in metres.
    fn face(&mut self, corners: [Vec3; 4], uvs: [Vec2; 4], normal: Vec3) {
        let first = self.mesh.vertices.len() as u32;
        for (position, uv) in corners.into_iter().zip(uvs) {
            self.mesh.vertices.push(Vertex {
                position,
                normal,
                uv,
            });
        }
        let wound = (corners[1] - corners[0])
            .cross(corners[2] - corners[0])
            .dot(normal)
            >= 0.0;
        if wound {
            self.mesh.indices.extend_from_slice(&[
                first,
                first + 1,
                first + 2,
                first,
                first + 2,
                first + 3,
            ]);
        } else {
            self.mesh.indices.extend_from_slice(&[
                first,
                first + 2,
                first + 1,
                first,
                first + 3,
                first + 2,
            ]);
        }
    }

    /// A timber through `points`, `width` across and `depth` deep, its depth towards `up`. Each
    /// has its grain along its length, begun somewhere of its own in the wood.
    fn timber(&mut self, points: &[Vec3], up: Vec3, width: f32, depth: f32) {
        let start = Vec2::new(self.rng.range(0.0, 8.0), self.rng.range(0.0, 8.0));
        let count = points.len();
        // Each cross-section: its four corners, and how far along the timber it lies.
        let mut sections = Vec::with_capacity(count);
        let mut distance = 0.0;
        for i in 0..count {
            let tangent = if i == 0 {
                points[1] - points[0]
            } else if i + 1 == count {
                points[i] - points[i - 1]
            } else {
                points[i + 1] - points[i - 1]
            }
            .normalize();
            if i > 0 {
                distance += points[i].distance(points[i - 1]);
            }
            let side = tangent.cross(up).normalize();
            let top = side.cross(tangent).normalize();
            let (w, d) = (side * (width * 0.5), top * (depth * 0.5));
            sections.push((
                [
                    points[i] - w - d,
                    points[i] + w - d,
                    points[i] + w + d,
                    points[i] - w + d,
                ],
                [-top, side, top, -side],
                distance,
            ));
        }
        // The four long faces, each wrapping the grain on round the timber.
        let girth = [
            0.0,
            width,
            width + depth,
            2.0 * width + depth,
            2.0 * (width + depth),
        ];
        for pair in sections.windows(2) {
            let ((a, normals, va), (b, _, vb)) = (&pair[0], &pair[1]);
            for k in 0..4 {
                let next = (k + 1) % 4;
                self.face(
                    [a[k], a[next], b[next], b[k]],
                    [
                        start + Vec2::new(girth[k], *va),
                        start + Vec2::new(girth[k + 1], *va),
                        start + Vec2::new(girth[k + 1], *vb),
                        start + Vec2::new(girth[k], *vb),
                    ],
                    normals[k],
                );
            }
        }
        // The sawn ends.
        for (section, outward) in [
            (&sections[0], (points[0] - points[1]).normalize()),
            (
                &sections[count - 1],
                (points[count - 1] - points[count - 2]).normalize(),
            ),
        ] {
            let c = section.0;
            self.face(
                c,
                [
                    start,
                    start + Vec2::new(width, 0.0),
                    start + Vec2::new(width, depth),
                    start + Vec2::new(0.0, depth),
                ],
                outward,
            );
        }
    }

    /// A straight timber from `a` to `b`.
    fn beam(&mut self, a: Vec3, b: Vec3, up: Vec3, width: f32, depth: f32) {
        self.timber(&[a, b], up, width, depth);
    }
}

/// Half the width of the bridge's deck, and how far its middle rises above its ends.
const HALF_WIDTH: f32 = 0.68;
const RISE: f32 = 0.4;

/// The line of the bridge's deck: from where it lands on one bank to the other, a little way
/// onto each, and the height of its top at each end.
struct Span {
    south: Vec2,
    north: Vec2,
    heights: (f32, f32),
}

impl Span {
    fn new(site: &Site) -> Self {
        let (south, north) = site.bridge;
        let along = (north - south).normalize();
        let (south, north) = (south - along * 0.5, north + along * 0.5);
        Self {
            south,
            north,
            heights: (
                ground_height(south.x, south.y) + 0.07,
                ground_height(north.x, north.y) + 0.07,
            ),
        }
    }

    /// The deck's top at `t` of the way across.
    fn deck(&self, t: f32) -> Vec3 {
        let at = self.south.lerp(self.north, t);
        let arch = RISE * (1.0 - (2.0 * t - 1.0).powi(2));
        Vec3::new(
            at.x,
            self.heights.0 + (self.heights.1 - self.heights.0) * t + arch,
            at.y,
        )
    }
}

/// The height of the bridge's deck over a spot, for walking on, if the bridge crosses it.
pub fn deck_height(site: &Site, p: Vec2) -> Option<f32> {
    let span = Span::new(site);
    let along = span.north - span.south;
    let t = (p - span.south).dot(along) / along.length_squared();
    let off = (p - span.south.lerp(span.north, t)).length();
    ((0.0..=1.0).contains(&t) && off <= HALF_WIDTH - 0.06).then(|| span.deck(t).y)
}

/// The footbridge: a low arch on piles, decked with boards, with a rail either side.
fn bridge(garden: &Garden) -> Mesh {
    let span = Span::new(&garden.site);
    let along = (span.north - span.south).normalize();
    let length = span.south.distance(span.north);
    let across = Vec3::new(-along.y, 0.0, along.x);
    let deck = |t: f32| span.deck(t);
    let run = |t: f32| (deck((t + 0.01).min(1.0)) - deck((t - 0.01).max(0.0))).normalize();
    let mut joiner = Joiner {
        mesh: Mesh::default(),
        rng: Rng::new(17, 23),
    };

    // Two bearers carry the deck from bank to bank.
    for side in [-1.0, 1.0] {
        let line: Vec<Vec3> = (0..=16)
            .map(|k| deck(k as f32 / 16.0) + across * (side * 0.46) - Vec3::Y * 0.135)
            .collect();
        joiner.timber(&line, Vec3::Y, 0.09, 0.2);
    }
    // The boards, each laid square across, a finger's gap between, none quite like the next.
    let boards = (length / 0.128) as u32;
    for k in 0..boards {
        let t = (k as f32 + 0.5) / boards as f32;
        let centre = deck(t) - Vec3::Y * 0.016;
        let skew = across + run(t) * joiner.rng.signed() * 0.006;
        let half = HALF_WIDTH + joiner.rng.signed() * 0.008;
        let lift = Vec3::Y * joiner.rng.signed() * 0.0012;
        let up = skew.cross(run(t)).normalize();
        let up = if up.y < 0.0 { -up } else { up };
        joiner.beam(
            centre - skew * half + lift,
            centre + skew * half + lift,
            up,
            0.12,
            0.032,
        );
    }
    // Posts either side: on the banks they stand in the ground, over the water they are piles
    // driven into the bed.
    let posts = 7;
    for side in [-1.0, 1.0] {
        let mut tops = Vec::new();
        let mut mids = Vec::new();
        for k in 0..posts {
            let t = k as f32 / (posts - 1) as f32;
            let at = deck(t) + across * (side * (HALF_WIDTH + 0.055));
            let foot = ground_height(at.x, at.z).min(WATER_LEVEL + 0.3) - 0.35;
            let top = at + Vec3::Y * 0.98;
            joiner.beam(Vec3::new(at.x, foot, at.z), top, across, 0.09, 0.09);
            tops.push(top + Vec3::Y * 0.025);
            mids.push(at + Vec3::Y * 0.52 - across * (side * 0.01));
        }
        // The rails run on past the end posts by a hand.
        let extend = |line: &mut Vec<Vec3>| {
            let (first, last) = (line[0], line[line.len() - 1]);
            let (a, b) = (
                (first - line[1]).normalize(),
                (last - line[line.len() - 2]).normalize(),
            );
            line.insert(0, first + a * 0.12);
            line.push(last + b * 0.12);
        };
        // Smooth the handrail over the arch, between the posts.
        let smooth = |line: &[Vec3]| -> Vec<Vec3> {
            let mut out = Vec::new();
            for pair in line.windows(2) {
                for step in 0..3 {
                    out.push(pair[0].lerp(pair[1], step as f32 / 3.0));
                }
            }
            out.push(*line.last().expect("a rail has posts"));
            out
        };
        extend(&mut tops);
        extend(&mut mids);
        joiner.timber(&smooth(&tops), Vec3::Y, 0.11, 0.05);
        joiner.timber(&smooth(&mids), Vec3::Y, 0.045, 0.07);
    }
    joiner.mesh
}

/// A garden seat: slatted, with arms, built about its own origin facing +Z.
fn seat() -> Mesh {
    const LENGTH: f32 = 1.7;
    const SEAT: f32 = 0.43;
    let mut joiner = Joiner {
        mesh: Mesh::default(),
        rng: Rng::new(19, 29),
    };
    let half = LENGTH * 0.5;
    // The back leans away from the sitter.
    let lean = Vec3::new(0.0, 1.0, -0.2).normalize();
    for x in [-half + 0.04, half - 0.04] {
        let front = Vec3::new(x, -0.03, 0.47);
        let back = Vec3::new(x, -0.03, 0.02);
        // Front leg to the arm; back leg on up as the back's post.
        joiner.beam(front, front + Vec3::Y * 0.67, Vec3::X, 0.06, 0.06);
        joiner.beam(back, back + Vec3::Y * 0.44, Vec3::X, 0.06, 0.06);
        let post = back + Vec3::Y * 0.44;
        joiner.beam(post, post + lean * 0.5, Vec3::X, 0.06, 0.055);
        // The seat's rail, a stretcher below it, and the arm.
        joiner.beam(
            back + Vec3::Y * 0.4,
            front + Vec3::Y * 0.4,
            Vec3::Y,
            0.045,
            0.07,
        );
        joiner.beam(
            back + Vec3::Y * 0.16,
            front + Vec3::Y * 0.16,
            Vec3::Y,
            0.04,
            0.045,
        );
        let arm_back = post + lean * 0.2;
        joiner.beam(
            arm_back - Vec3::Z * 0.04,
            Vec3::new(x, 0.655, 0.53),
            Vec3::Y,
            0.07,
            0.03,
        );
    }
    // Rails front and back under the seat.
    for z in [0.04, 0.45] {
        joiner.beam(
            Vec3::new(-half + 0.07, 0.385, z),
            Vec3::new(half - 0.07, 0.385, z),
            Vec3::Y,
            0.035,
            0.07,
        );
    }
    // The seat's slats, the front one rounded down a little.
    for k in 0..6 {
        let z = 0.075 + 0.076 * k as f32;
        let dip = if k == 5 { 0.006 } else { 0.0 };
        let wobble = joiner.rng.signed() * 0.001;
        joiner.beam(
            Vec3::new(-half, SEAT + 0.011 - dip + wobble, z),
            Vec3::new(half, SEAT + 0.011 - dip + wobble, z),
            Vec3::Y,
            0.064,
            0.022,
        );
    }
    // The back: a top rail and slats below it, along the lean.
    let back = Vec3::new(0.0, 0.41, 0.02);
    for (k, up) in [0.13, 0.22, 0.31, 0.4].into_iter().enumerate() {
        let at = back + lean * up + Vec3::Z * 0.035;
        let depth = if k == 3 { 0.075 } else { 0.06 };
        joiner.beam(
            at - Vec3::X * (half - 0.07),
            at + Vec3::X * (half - 0.07),
            lean.cross(Vec3::X).normalize(),
            depth,
            0.022,
        );
    }
    joiner.mesh
}

/// Builds the garden's woodwork.
pub fn add(garden: &mut Garden) {
    println!("building the bridge and the seat...");
    let layer = garden.layer("weathered oak", || weathered_oak(0));
    let wood = garden.material(Material {
        base_color: [0.95, 0.95, 0.95, 1.0],
        roughness: 1.0,
        uv_scale: 1.0 / TIMBER_TILE_METRES,
        normal_strength: 1.0,
        texture_layer: layer,
        ..Default::default()
    });
    let mesh = bridge(garden);
    println!("  bridge: {} triangles", mesh.triangle_count());
    let mesh = garden.mesh(&mesh);
    garden.place(mesh, wood, Mat4::IDENTITY);

    // The seat under the great oak, looking out over the pond.
    let at = Vec2::new(7.3, -17.3);
    let seat = garden.mesh(&seat());
    let transform = Mat4::from_translation(Vec3::new(at.x, ground_height(at.x, at.y), at.y))
        * Mat4::from_rotation_y(-0.12);
    garden.place(seat, wood, transform);
}
