//! The garden's grass: mown lawns round the pond and along the paths, a flowering meadow beyond
//! them, and rough tall grass at the water's margin and under the rim of the wood.
//!
//! Each kind is a breed of the engine's grass, written here as a breed file is, and laid as
//! square patches. A lawn's edge against a path is a line a hand's width wide, so the patches
//! are laid as the ground's squares are: large where a whole one fits, and halved, and halved
//! again, where an edge crosses, down to a quarter of a metre. No patch is cut to shape; the
//! small ones follow every curve closely enough that the blades leaning out of them make the
//! edge.

use super::site::{ground_height, pond_distance, wood_density, Site};
use super::Garden;
use glam::{Mat4, Vec2, Vec3, Vec4};
use solstice_asset::Material;
use solstice_procgen::foliage::grass::Meadow;
use solstice_procgen::noise::fbm;

/// A garden lawn, a week after mowing: fine blades standing close, with daisies and white
/// clover low enough to have missed the blades.
const LAWN: &str = "\
size 1
tufts per_m2=320 blades=6..12 height=0.045..0.08 width=0.0035..0.0055 lean=0.05..0.45
undergrowth per_m2=2000 height=0.025..0.055
variants 0.5,0.35,0.15,0
dry 0.02
colors 0.05,0.115,0.02 0.075,0.14,0.024 0.042,0.1,0.03 0.2,0.17,0.08
weed kind=clover per_m2=10 leaves=5..10 length=0.008..0.013 width=0.9 stalk=0.025..0.05 flowers=0.3 flower_height=0.05..0.08 flower_size=0.007 leaf=0.035,0.08,0.03 flower=0.6,0.58,0.52
weed kind=rosette per_m2=5 leaves=6..10 length=0.02..0.04 width=0.5 flowers=0.75 flower_height=0.04..0.09 flower_size=0.011 leaf=0.04,0.085,0.02 flower=0.78,0.78,0.72 centre=0.62,0.44,0.02
";

/// The meadow beyond the lawns, in June: knee-high grass in flower, with buttercups, oxeye
/// daisies and red clover through it.
const MEADOW: &str = "\
size 2
tufts per_m2=46 blades=8..20 height=0.25..0.6 width=0.005..0.009 lean=0.12..0.82
undergrowth per_m2=420 height=0.05..0.18
variants 0.45,0.3,0.25,0
dry 0.1
colors 0.05,0.105,0.018 0.085,0.13,0.022 0.042,0.09,0.034 0.25,0.2,0.09
weed kind=rosette per_m2=5 leaves=4..7 length=0.05..0.1 width=0.6 flowers=0.9 flower_height=0.28..0.5 flower_size=0.011 leaf=0.04,0.09,0.02 flower=0.75,0.58,0.01 centre=0.6,0.42,0.01
weed kind=rosette per_m2=1.6 leaves=6..10 length=0.06..0.12 width=0.3 flowers=0.9 flower_height=0.35..0.6 flower_size=0.022 leaf=0.04,0.085,0.02 flower=0.8,0.8,0.76 centre=0.62,0.42,0.01
weed kind=clover per_m2=5 leaves=5..10 length=0.012..0.02 width=0.9 stalk=0.06..0.14 flowers=0.6 flower_height=0.12..0.25 flower_size=0.012 leaf=0.035,0.08,0.03 flower=0.5,0.12,0.22
";

/// Rough grass where no mower goes: broad blades to a metre, in clumps that arch over.
const ROUGH: &str = "\
size 2
tufts per_m2=20 blades=12..26 height=0.55..1.05 width=0.008..0.014 lean=0.2..0.85
undergrowth per_m2=240 height=0.1..0.3
fold 0.22
variants 0.35,0.25,0.4,0
dry 0.12
colors 0.045,0.1,0.02 0.07,0.12,0.025 0.035,0.085,0.04 0.26,0.2,0.09
";

/// The kinds of grass, in the order of their breeds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sward {
    Lawn,
    Meadow,
    Rough,
}

/// Which grass grows at `p`, if any does.
fn sward_at(site: &Site, p: Vec2) -> Option<Sward> {
    if p.abs().max_element() > 110.0 || !site.may_grow_grass(p, 0.0) {
        return None;
    }
    // Deep in the wood too little light reaches the ground.
    let wood = wood_density(p);
    if wood > 0.85 {
        return None;
    }
    // Shingle and bare earth stay bare.
    let shares = site.ground_shares(p);
    if shares.x > 0.3 || shares.y > 0.5 {
        return None;
    }
    let pond = pond_distance(p);
    let drift = fbm(Vec3::new(p.x * 0.13, 21.0, p.y * 0.13), 3, 861);
    // Left long along stretches of the bank, and under the rim of the wood.
    if (pond < 1.5 + drift * 2.0 && drift > 0.0) || wood > 0.22 {
        return Some(Sward::Rough);
    }
    // Mown: the lawns about the pond, whose edge wanders, and a strip beside every path.
    let wander = fbm(Vec3::new(p.x * 0.05, 23.0, p.y * 0.05), 3, 863) * 5.0;
    if p.length() + wander < 31.0 || site.path_distance(p) < 2.7 {
        Some(Sward::Lawn)
    } else {
        Some(Sward::Meadow)
    }
}

/// A patch whose local square is `0..size`, laid with its corner at `(x, z)`: sheared to the
/// ground's slope, so every blade still grows straight up and every root touches the ground,
/// and turned by `turn` quarter turns about its middle.
fn patch_transform(x: f32, z: f32, size: f32, turn: u32) -> Mat4 {
    let half = size * 0.5;
    let (cx, cz) = (x + half, z + half);
    // The ground's best plane over the patch, from its four corners and its middle.
    let corner = |dx: f32, dz: f32| ground_height(cx + dx * half, cz + dz * half);
    let (a, b, c, d) = (
        corner(-1.0, -1.0),
        corner(1.0, -1.0),
        corner(-1.0, 1.0),
        corner(1.0, 1.0),
    );
    let slope_x = (b + d - a - c) / (2.0 * size);
    let slope_z = (c + d - a - b) / (2.0 * size);
    let height = (a + b + c + d + 4.0 * ground_height(cx, cz)) / 8.0;
    let shear = Mat4::from_cols(
        Vec4::new(1.0, slope_x, 0.0, 0.0),
        Vec4::Y,
        Vec4::new(0.0, slope_z, 1.0, 0.0),
        Vec4::W,
    );
    Mat4::from_translation(Vec3::new(cx, height, cz))
        * shear
        * Mat4::from_rotation_y(turn as f32 * std::f32::consts::FRAC_PI_2)
        * Mat4::from_translation(Vec3::new(-half, 0.0, -half))
}

/// A small, fast hash for placement.
fn hash(x: i32, z: i32, channel: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (z as u32).wrapping_mul(0xd816_3841)
        ^ channel.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^ (h >> 12)
}

/// One breed as laid: its patches' meshes at each size, largest first, and its material.
struct Laid {
    sward: Sward,
    /// For each size, halving each time: the size, and its patches' meshes.
    sizes: Vec<(f32, Vec<u32>)>,
    material: u32,
}

/// Grows a breed's patches at `levels` sizes, the largest `seeds` different ways.
fn grow(
    garden: &mut Garden,
    sward: Sward,
    name: &str,
    text: &str,
    levels: u32,
    seeds: u32,
) -> Laid {
    let breed = Meadow::parse(text).unwrap_or_else(|e| panic!("the garden's {name}: {e}"));
    let atlas = garden.layer(name, || breed.atlas(0));
    let material = garden.material(Material {
        base_color: [1.0; 4],
        roughness: 1.0,
        normal_strength: 1.0,
        texture_layer: atlas,
        flags: Material::TWO_SIDED | Material::FOLIAGE | Material::SWAYS,
        ..Default::default()
    });
    let mut sizes = Vec::new();
    for level in 0..levels {
        let size = breed.size / (1u32 << level) as f32;
        let meshes: Vec<u32> = (0..seeds)
            .map(|seed| {
                let mesh = Meadow {
                    size,
                    seed: seed + level * 16,
                    ..breed.clone()
                }
                .mesh();
                if seed == 0 {
                    println!("  {name}, {size} m: {} triangles", mesh.triangle_count());
                }
                garden.mesh(&mesh)
            })
            .collect();
        sizes.push((size, meshes));
    }
    Laid {
        sward,
        sizes,
        material,
    }
}

/// Lays the patches of one breed over the square from `(x, z)`, `size` across, which is
/// `level` halvings down: whole where the breed grows all over it, in quarters where it grows
/// over part, and at the smallest size wherever it grows at the middle.
fn lay(garden: &mut Garden, laid: &Laid, x: f32, z: f32, level: usize, counts: &mut [u32]) {
    let (size, meshes) = &laid.sizes[level];
    let size = *size;
    let mut grows = 0;
    for i in 0..3 {
        for j in 0..3 {
            let p = Vec2::new(x + size * 0.5 * i as f32, z + size * 0.5 * j as f32);
            if sward_at(&garden.site, p) == Some(laid.sward) {
                grows += 1;
            }
        }
    }
    let last = level + 1 == laid.sizes.len();
    let middle = Vec2::new(x + size * 0.5, z + size * 0.5);
    if grows == 9 || (last && sward_at(&garden.site, middle) == Some(laid.sward)) {
        // Named by where it lies, so the same garden grows every time.
        let (i, j) = ((x * 8.0).round() as i32, (z * 8.0).round() as i32);
        let mesh = meshes[hash(i, j, 0) as usize % meshes.len()];
        let turn = hash(i, j, 1) % 4;
        garden.place(mesh, laid.material, patch_transform(x, z, size, turn));
        counts[level] += 1;
    } else if grows > 0 && !last {
        let half = size * 0.5;
        for (dx, dz) in [(0.0, 0.0), (half, 0.0), (0.0, half), (half, half)] {
            lay(garden, laid, x + dx, z + dz, level + 1, counts);
        }
    }
}

/// Grows the garden's grass.
pub fn add(garden: &mut Garden) {
    println!("growing the grass...");
    let breeds = [
        grow(garden, Sward::Lawn, "lawn", LAWN, 3, 4),
        grow(garden, Sward::Meadow, "meadow", MEADOW, 3, 4),
        grow(garden, Sward::Rough, "rough grass", ROUGH, 3, 3),
    ];
    for laid in &breeds {
        let mut counts = vec![0u32; laid.sizes.len()];
        let step = laid.sizes[0].0;
        let cells = (112.0 / step).ceil() as i32;
        for j in -cells..cells {
            for i in -cells..cells {
                lay(
                    garden,
                    laid,
                    i as f32 * step,
                    j as f32 * step,
                    0,
                    &mut counts,
                );
            }
        }
        let sizes: Vec<String> = laid
            .sizes
            .iter()
            .zip(&counts)
            .map(|((size, _), count)| format!("{count} of {size} m"))
            .collect();
        println!("  {:?}: {}", laid.sward, sizes.join(", "));
    }
}
