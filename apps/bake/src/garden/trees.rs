//! The garden's trees: the willows and oaks placed by hand in the plan, and the wood that closes
//! the garden in on every side.

use super::site::{ground_height, pond_distance, wood_density, PATH_WIDTH};
use super::Garden;
use glam::{Mat4, Quat, Vec2, Vec3};
use solstice_asset::Material;
use solstice_procgen::foliage::bark::BARK_TILE_METRES;
use solstice_procgen::foliage::tree::Species;
use solstice_procgen::foliage::Rng;
use solstice_procgen::noise::fbm;

/// The meshes and materials of one species: wood, leaves and, for some, twigs, for each grown
/// copy.
struct Grown {
    /// For each copy: its wood, its leaves, and its twigs if it has them.
    copies: Vec<(u32, u32, Option<u32>)>,
    bark: u32,
    leaves: u32,
    twigs: u32,
}

/// Grows `copies` of a species, from the given seeds on.
fn grow(garden: &mut Garden, name: &str, seeds: &[u32]) -> Grown {
    let species = Species::by_name(name).unwrap_or_else(|| panic!("no species {name}"));
    let bark = species.bark;
    let leaf = species.leaves.kind;
    let bark_layer = garden.layer(bark.name(), || bark.surface(0));
    let leaf_layer = garden.layer(leaf.name(), || leaf.atlas(0));
    let bark = garden.material(Material {
        base_color: [1.0; 4],
        roughness: 1.0,
        uv_scale: 1.0 / BARK_TILE_METRES,
        normal_strength: 1.0,
        texture_layer: bark_layer,
        flags: Material::SWAYS | Material::TREE,
        ..Default::default()
    });
    let leaves = garden.material(Material {
        base_color: [1.0; 4],
        roughness: 1.0,
        normal_strength: 1.0,
        texture_layer: leaf_layer,
        flags: Material::TWO_SIDED | Material::FOLIAGE | Material::SWAYS | Material::TREE,
        ..Default::default()
    });
    let [r, g, b] = species
        .twigs
        .map_or([0.2, 0.17, 0.08], |(_, colour)| colour);
    let twigs = garden.material(Material {
        base_color: [r, g, b, 1.0],
        roughness: 0.62,
        flags: Material::SWAYS | Material::TREE,
        ..Default::default()
    });
    let copies = seeds
        .iter()
        .map(|&seed| {
            let plant = species.grow(seed);
            println!(
                "  {name} {seed}: {} wood, {} leaf and {} twig triangles",
                plant.wood.triangle_count(),
                plant.leaves.triangle_count(),
                plant.twigs.triangle_count()
            );
            let wood = garden.mesh(&plant.wood);
            let leaf = garden.mesh(&plant.leaves);
            let twig = (!plant.twigs.indices.is_empty()).then(|| garden.mesh(&plant.twigs));
            (wood, leaf, twig)
        })
        .collect();
    Grown {
        copies,
        bark,
        leaves,
        twigs,
    }
}

impl Grown {
    /// Stands a copy at `at` on the ground, turned by `yaw`, `scale` times its grown size, and
    /// leaning the way `lean` points over the ground, by its length in radians.
    fn place(&self, garden: &mut Garden, copy: u32, at: Vec2, yaw: f32, scale: f32, lean: Vec2) {
        let (wood, leaves, twigs) = self.copies[copy as usize % self.copies.len()];
        // A leaning tree turns about a level axis square to the way it leans.
        let axis = Vec3::new(lean.y, 0.0, -lean.x).normalize_or(Vec3::X);
        let tilt = Quat::from_axis_angle(axis, lean.length());
        let transform = Mat4::from_translation(Vec3::new(
            at.x,
            ground_height(at.x, at.y) - 0.06 * scale,
            at.y,
        )) * Mat4::from_quat(tilt)
            * Mat4::from_rotation_y(yaw)
            * Mat4::from_scale(Vec3::splat(scale));
        garden.place(wood, self.bark, transform);
        garden.place(leaves, self.leaves, transform);
        if let Some(twigs) = twigs {
            garden.place(twigs, self.twigs, transform);
        }
    }
}

/// Which way the water lies from `at`: down the slope of the distance to it.
fn towards_water(at: Vec2) -> Vec2 {
    let step = 0.5;
    Vec2::new(
        pond_distance(at - Vec2::X * step) - pond_distance(at + Vec2::X * step),
        pond_distance(at - Vec2::Y * step) - pond_distance(at + Vec2::Y * step),
    )
    .normalize_or(Vec2::X)
}

/// Plants the trees.
pub fn add(garden: &mut Garden) {
    println!("growing the trees...");
    let willow = grow(garden, "willow", &[0, 1]);
    let old_oak = grow(garden, "old_oak", &[0, 1, 2]);
    let birch = grow(garden, "birch", &[0, 1]);
    let oak = grow(garden, "oak", &[0, 1]);
    let fir = grow(garden, "fir", &[0, 1]);
    let bush = grow(garden, "bush", &[0, 1]);

    let specimens = garden.site.specimens.clone();
    for tree in &specimens {
        let (grown, lean) = match tree.species {
            // Willows lean out over the water they grow beside.
            "willow" => (&willow, 0.1),
            "old_oak" => (&old_oak, 0.0),
            "birch" => (&birch, 0.03),
            other => panic!("the plan names a species with no trees: {other}"),
        };
        grown.place(
            garden,
            tree.variant,
            tree.at,
            tree.yaw,
            tree.scale,
            towards_water(tree.at) * lean,
        );
    }

    // The wood: trees scattered by dart throwing, each its own space, closer together the
    // deeper in. Great oaks and birches stand along its rim, where there is light; firs darken
    // its depths, so the garden's leaves are seen against shade.
    let mut rng = Rng::new(31, 9);
    let mut stands: Vec<Vec2> = specimens.iter().map(|tree| tree.at).collect();
    let planted = stands.len();
    let half = super::site::SIZE * 0.5 - 14.0;
    for _ in 0..90_000 {
        let at = Vec2::new(rng.range(-half, half), rng.range(-half, half));
        let density = wood_density(at);
        if density <= 0.0 || rng.unit() > density {
            continue;
        }
        if garden.site.path_distance(at) < PATH_WIDTH * 0.5 + 2.2 {
            continue;
        }
        let spacing = 10.5 - 4.5 * density;
        if stands
            .iter()
            .all(|s| s.distance_squared(at) > spacing * spacing)
        {
            stands.push(at);
        }
    }
    let mut counts = [0; 4];
    for (index, &at) in stands.iter().enumerate().skip(planted) {
        let depth = wood_density(at);
        let stand = fbm(Vec3::new(at.x * 0.03, 7.0, at.y * 0.03), 3, 853);
        let turn = rng.range(0.0, std::f32::consts::TAU);
        let (grown, pick, scale) = if depth < 0.8 && stand > 0.02 {
            (&old_oak, 0, rng.range(0.7, 1.05))
        } else if depth < 0.9 && stand < -0.1 {
            (&birch, 1, rng.range(0.8, 1.2))
        } else if depth > 0.6 && stand > 0.05 {
            (&fir, 2, rng.range(0.85, 1.35))
        } else {
            (&oak, 3, rng.range(0.9, 1.4))
        };
        counts[pick] += 1;
        grown.place(garden, index as u32, at, turn, scale, Vec2::ZERO);
    }
    // Bushes under the rim of the wood, where the light reaches.
    let mut bushes: Vec<Vec2> = Vec::new();
    for _ in 0..30_000 {
        let at = Vec2::new(rng.range(-90.0, 90.0), rng.range(-90.0, 90.0));
        let edge = wood_density(at);
        let chance = (1.0 - ((edge - 0.3) / 0.3).abs()).max(0.0);
        if rng.unit() > chance * 0.5
            || garden.site.path_distance(at) < PATH_WIDTH * 0.5 + 1.2
            || stands.iter().any(|s| s.distance_squared(at) < 2.2 * 2.2)
            || bushes.iter().any(|b| b.distance_squared(at) < 1.7 * 1.7)
        {
            continue;
        }
        bushes.push(at);
        let turn = rng.range(0.0, std::f32::consts::TAU);
        let copy = rng.index(2) as u32;
        bush.place(garden, copy, at, turn, rng.range(0.8, 1.5), Vec2::ZERO);
    }
    println!(
        "  {} trees in the wood ({} great oaks, {} birches, {} firs, {} oaks) and {} bushes",
        stands.len() - planted,
        counts[0],
        counts[1],
        counts[2],
        counts[3],
        bushes.len()
    );
}
