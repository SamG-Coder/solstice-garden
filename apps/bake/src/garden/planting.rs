//! Where the garden's flowers grow: the borders along the way in, the shade beds under the oaks,
//! the pond's margin and the water itself.
//!
//! A border is planted as gardeners plant one: low mounds of lavender and catmint along its
//! edge, daisies, poppies, salvias and alliums behind them, and foxgloves and delphiniums at the
//! back, each kind in drifts rather than dotted about. Shade beds hold ferns and hostas. Flag
//! iris and reedmace stand in the shallows of the pond's shelf, where the plan left room for
//! them, and rafts of water lilies float over the deeper water.

use super::herbs::{self, leaf, petal, Blade, Paint, Spiked};
use super::site::{
    ground_height, pond_bounds, pond_distance, to_segment, wood_density, BedKind, PATH_WIDTH,
    WATER_LEVEL,
};
use super::Garden;
use glam::{Mat4, Vec2, Vec3};
use solstice_asset::Material;
use solstice_builder::Mesh;
use solstice_procgen::foliage::Rng;
use solstice_procgen::noise::fbm;

/// The plants, in the order of [`Planting::meshes`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Plant {
    Lavender,
    Catmint,
    Salvia,
    Delphinium,
    Foxglove,
    WhiteFoxglove,
    Daisy,
    Poppy,
    Allium,
    Iris,
    Reedmace,
    Fern,
    Hosta,
    WhiteLily,
    PinkLily,
}

const PLANTS: [Plant; 15] = [
    Plant::Lavender,
    Plant::Catmint,
    Plant::Salvia,
    Plant::Delphinium,
    Plant::Foxglove,
    Plant::WhiteFoxglove,
    Plant::Daisy,
    Plant::Poppy,
    Plant::Allium,
    Plant::Iris,
    Plant::Reedmace,
    Plant::Fern,
    Plant::Hosta,
    Plant::WhiteLily,
    Plant::PinkLily,
];

/// Grown copies of each plant.
const COPIES: u32 = 3;

fn narrow(length: f32, width: f32, paint: Paint) -> Blade {
    Blade {
        shape: (0.3, 0.5),
        rows: 1,
        arch: 0.3,
        fold: 0.3,
        ..leaf(length, width, paint)
    }
}

impl Plant {
    fn grow(self, seed: u32) -> Mesh {
        match self {
            Plant::Lavender => Spiked {
                radius: 0.3,
                height: 0.26,
                shoots: 46,
                leaves: 9,
                leaf: narrow(0.038, 0.13, Paint::GreyLeaf),
                stems: (70, 110),
                stem_length: (0.38, 0.58),
                splay: 0.75,
                spike: 0.065,
                core: 0.0042,
                stalk: 0.0,
                whorls: 7,
                florets: 4,
                floret: petal(0.0065, 0.55, Paint::Violet),
            }
            .grow(seed),
            Plant::Catmint => Spiked {
                radius: 0.4,
                height: 0.3,
                shoots: 40,
                leaves: 10,
                leaf: narrow(0.03, 0.7, Paint::GreyLeaf),
                stems: (50, 80),
                stem_length: (0.35, 0.6),
                splay: 1.0,
                spike: 0.16,
                core: 0.003,
                stalk: 0.0,
                whorls: 12,
                florets: 4,
                floret: petal(0.008, 0.55, Paint::Blue),
            }
            .grow(seed),
            Plant::Salvia => Spiked {
                radius: 0.22,
                height: 0.3,
                shoots: 26,
                leaves: 8,
                leaf: leaf(0.07, 0.4, Paint::Leaf),
                stems: (22, 36),
                stem_length: (0.5, 0.75),
                splay: 0.3,
                spike: 0.26,
                core: 0.0038,
                stalk: 0.0,
                whorls: 22,
                florets: 4,
                floret: petal(0.008, 0.5, Paint::Purple),
            }
            .grow(seed),
            Plant::Delphinium => Spiked {
                radius: 0.25,
                height: 0.45,
                shoots: 14,
                leaves: 6,
                leaf: leaf(0.13, 0.9, Paint::Leaf),
                stems: (3, 6),
                stem_length: (1.2, 1.8),
                splay: 0.12,
                spike: 0.6,
                core: 0.003,
                stalk: 0.026,
                whorls: 15,
                florets: 5,
                floret: Blade {
                    arch: -0.25,
                    ..petal(0.017, 0.8, Paint::Blue)
                },
            }
            .grow(seed),
            Plant::Foxglove => herbs::foxglove(seed, Paint::Foxglove),
            Plant::WhiteFoxglove => herbs::foxglove(seed + 50, Paint::White),
            Plant::Daisy => herbs::daisy(seed),
            Plant::Poppy => herbs::poppy(seed),
            Plant::Allium => herbs::allium(seed),
            Plant::Iris => herbs::iris(seed),
            Plant::Reedmace => herbs::reedmace(seed),
            Plant::Fern => herbs::fern(seed),
            Plant::Hosta => herbs::hosta(seed),
            Plant::WhiteLily => herbs::water_lily(seed, Paint::White),
            Plant::PinkLily => herbs::water_lily(seed + 50, Paint::Pink),
        }
    }

    /// Whether it floats on the water rather than standing in the wind.
    fn floats(self) -> bool {
        matches!(self, Plant::WhiteLily | Plant::PinkLily)
    }
}

/// The plants grown, and their materials.
struct Planting {
    /// For each plant of [`PLANTS`], its copies.
    meshes: Vec<Vec<u32>>,
    sways: u32,
    floats: u32,
    counts: Vec<u32>,
}

impl Planting {
    fn place(&mut self, garden: &mut Garden, rng: &mut Rng, plant: Plant, at: Vec2, scale: f32) {
        let index = PLANTS
            .iter()
            .position(|&p| p == plant)
            .expect("a listed plant");
        let copies = &self.meshes[index];
        let mesh = copies[rng.index(copies.len())];
        let height = if plant.floats() {
            WATER_LEVEL
        } else {
            ground_height(at.x, at.y) - 0.01
        };
        let transform = Mat4::from_translation(Vec3::new(at.x, height, at.y))
            * Mat4::from_rotation_y(rng.range(0.0, std::f32::consts::TAU))
            * Mat4::from_scale(Vec3::splat(scale));
        let material = if plant.floats() {
            self.floats
        } else {
            self.sways
        };
        garden.place(mesh, material, transform);
        self.counts[index] += 1;
    }
}

/// Spots within the rectangle from `low` to `high` that `accept` takes, none nearer another
/// than `spacing`: thrown at random and kept where there is room, as plants space themselves.
fn scatter(
    rng: &mut Rng,
    low: Vec2,
    high: Vec2,
    spacing: f32,
    accept: &dyn Fn(Vec2) -> bool,
) -> Vec<Vec2> {
    let area = (high.x - low.x) * (high.y - low.y);
    let tries = (area / (spacing * spacing) * 12.0) as u32;
    let mut spots: Vec<Vec2> = Vec::new();
    for _ in 0..tries {
        let at = Vec2::new(rng.range(low.x, high.x), rng.range(low.y, high.y));
        if accept(at)
            && spots
                .iter()
                .all(|s| s.distance_squared(at) > spacing * spacing)
        {
            spots.push(at);
        }
    }
    spots
}

/// Plants the garden's flowers.
pub fn add(garden: &mut Garden) {
    println!("planting the flowers...");
    let layer = garden.layer("flowers", || herbs::atlas(0));
    let material = |flags: u32| Material {
        base_color: [1.0; 4],
        roughness: 1.0,
        normal_strength: 1.0,
        texture_layer: layer,
        flags: Material::TWO_SIDED | Material::FOLIAGE | flags,
        ..Default::default()
    };
    let sways = garden.material(material(Material::SWAYS));
    let floats = garden.material(material(0));
    let mut planting = Planting {
        meshes: Vec::new(),
        sways,
        floats,
        counts: vec![0; PLANTS.len()],
    };
    for plant in PLANTS {
        let copies: Vec<u32> = (0..COPIES)
            .map(|seed| {
                let mesh = plant.grow(seed);
                if seed == 0 {
                    println!("  {plant:?}: {} triangles", mesh.triangle_count());
                }
                garden.mesh(&mesh)
            })
            .collect();
        planting.meshes.push(copies);
    }

    let mut rng = Rng::new(53, 11);
    let drift = |p: Vec2, seed: u32| {
        fbm(
            Vec3::new(p.x * 0.38, seed as f32, p.y * 0.38),
            3,
            880 + seed,
        )
    };

    // The beds.
    let beds = garden.site.beds.clone();
    for bed in &beds {
        let low = bed.from.min(bed.to) - Vec2::splat(bed.radius + 1.0);
        let high = bed.from.max(bed.to) + Vec2::splat(bed.radius + 1.0);
        let spacing = match bed.kind {
            BedKind::Border => 0.36,
            BedKind::Shade => 0.46,
        };
        let spots = {
            let site = &garden.site;
            scatter(&mut rng, low, high, spacing, &|p| {
                let (depth, kind) = site.bed_depth(p);
                depth > 0.14
                    && kind == Some(bed.kind)
                    && to_segment(p, bed.from, bed.to) < bed.radius + 0.6
            })
        };
        for at in spots {
            let depth = garden.site.bed_depth(at).0;
            let (a, b) = (drift(at, 1), drift(at, 2));
            let plant = match bed.kind {
                BedKind::Border if depth < 0.6 => {
                    if a > 0.0 {
                        Plant::Lavender
                    } else {
                        Plant::Catmint
                    }
                }
                BedKind::Border if depth < 1.2 => {
                    if b > 0.22 {
                        Plant::Salvia
                    } else if b > -0.04 {
                        Plant::Daisy
                    } else if b > -0.3 {
                        Plant::Poppy
                    } else {
                        Plant::Allium
                    }
                }
                BedKind::Border => {
                    if a > 0.12 {
                        Plant::Delphinium
                    } else if a > -0.2 {
                        Plant::Foxglove
                    } else if b > 0.0 {
                        Plant::WhiteFoxglove
                    } else {
                        Plant::Daisy
                    }
                }
                BedKind::Shade if depth < 0.55 => Plant::Hosta,
                BedKind::Shade => {
                    if rng.unit() < 0.09 {
                        Plant::WhiteFoxglove
                    } else if a > -0.05 {
                        Plant::Fern
                    } else {
                        Plant::Hosta
                    }
                }
            };
            let scale = rng.range(0.85, 1.2);
            planting.place(garden, &mut rng, plant, at, scale);
        }
    }

    // Ferns in the dappled shade along the paths through the wood's rim.
    let spots = {
        let site = &garden.site;
        scatter(&mut rng, Vec2::splat(-80.0), Vec2::splat(80.0), 1.5, &|p| {
            let wood = wood_density(p);
            let path = site.path_distance(p);
            (0.2..0.95).contains(&wood)
                && path > PATH_WIDTH * 0.5 + 0.4
                && path < 5.5
                && fbm(Vec3::new(p.x * 0.25, 31.0, p.y * 0.25), 2, 887) > -0.1
        })
    };
    for at in spots {
        let scale = rng.range(0.8, 1.3);
        planting.place(garden, &mut rng, Plant::Fern, at, scale);
    }

    // The pond's margin: flag iris in the shallows, reedmace a little further out, each in
    // stands, and neither where the bridge lands.
    let [x0, z0, x1, z1] = pond_bounds();
    let (low, high) = (Vec2::new(x0, z0), Vec2::new(x1, z1));
    let (south, north) = garden.site.bridge;
    let clear_of_bridge = |p: Vec2, by: f32| to_segment(p, south, north) > by;
    let spots = scatter(&mut rng, low, high, 0.55, &|p| {
        let pond = pond_distance(p);
        (-0.75..0.12).contains(&pond)
            && fbm(Vec3::new(p.x * 0.2, 33.0, p.y * 0.2), 3, 871) > 0.1
            && clear_of_bridge(p, 2.6)
    });
    for at in spots {
        let scale = rng.range(0.8, 1.15);
        planting.place(garden, &mut rng, Plant::Iris, at, scale);
    }
    let spots = scatter(&mut rng, low, high, 0.5, &|p| {
        let pond = pond_distance(p);
        (-1.3..-0.3).contains(&pond)
            && fbm(Vec3::new(p.x * 0.17, 35.0, p.y * 0.17), 3, 873) > 0.2
            && clear_of_bridge(p, 3.2)
    });
    for at in spots {
        let scale = rng.range(0.85, 1.2);
        planting.place(garden, &mut rng, Plant::Reedmace, at, scale);
    }

    // Water lilies, in colonies over the deeper water: white, and pink in the east lobe.
    let colonies = [
        (Vec2::new(-8.5, -1.8), 3.4, Plant::WhiteLily),
        (Vec2::new(-3.6, 3.4), 2.6, Plant::WhiteLily),
        (Vec2::new(-11.6, 7.0), 2.2, Plant::WhiteLily),
        (Vec2::new(9.6, 1.6), 2.8, Plant::PinkLily),
        (Vec2::new(11.2, -3.6), 2.0, Plant::PinkLily),
    ];
    for (centre, radius, plant) in colonies {
        let spots = scatter(
            &mut rng,
            centre - Vec2::splat(radius),
            centre + Vec2::splat(radius),
            1.0,
            &|p| {
                // Ragged at its rim, and clear of the banks and the bridge.
                let reach =
                    radius * (0.8 + 0.3 * fbm(Vec3::new(p.x * 0.6, 37.0, p.y * 0.6), 2, 875));
                p.distance(centre) < reach && pond_distance(p) < -1.5 && clear_of_bridge(p, 1.4)
            },
        );
        for at in spots {
            let scale = rng.range(0.85, 1.25);
            planting.place(garden, &mut rng, plant, at, scale);
        }
    }

    let planted: Vec<String> = PLANTS
        .iter()
        .zip(&planting.counts)
        .map(|(plant, count)| format!("{count} {plant:?}"))
        .collect();
    println!("  planted {}", planted.join(", "));
}
