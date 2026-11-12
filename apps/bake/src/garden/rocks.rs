//! Stones: boulders set into the pond's banks in groups, as a gardener places them, a pair at
//! each end of the bridge, and a few in the borders.

use super::site::{ground_height, pond_bounds, pond_distance, to_segment, PATH_WIDTH};
use super::Garden;
use glam::{Mat4, Vec2, Vec3};
use solstice_asset::Material;
use solstice_bake::meshes;
use solstice_procgen::foliage::Rng;
use solstice_procgen::noise::fbm;
use solstice_procgen::surface::TILE_METRES;
use solstice_procgen::surfaces as library;

/// Different boulders, each turned and stretched its own way wherever it is set.
const BOULDERS: u32 = 5;

/// Sets the garden's stones.
pub fn add(garden: &mut Garden) {
    println!("setting the stones...");
    let layer = garden.layer("rock", || library::rock(0));
    let stone = garden.material(Material {
        base_color: [0.78, 0.76, 0.72, 1.0],
        roughness: 1.0,
        uv_scale: 1.0 / TILE_METRES,
        normal_strength: 1.0,
        texture_layer: layer,
        macro_variation: 0.6,
        ..Default::default()
    });
    let boulders: Vec<u32> = (0..BOULDERS)
        .map(|seed| garden.mesh(&meshes::rock(5, 40 + seed)))
        .collect();

    let mut rng = Rng::new(61, 13);
    let mut set: Vec<(Vec2, f32)> = Vec::new();
    let place =
        |garden: &mut Garden, rng: &mut Rng, set: &mut Vec<(Vec2, f32)>, at: Vec2, size: f32| {
            // Garden boulders lie on their broad sides, a third or more of them in the ground.
            let squash = Vec3::new(
                rng.range(0.85, 1.25),
                rng.range(0.5, 0.75),
                rng.range(0.85, 1.25),
            ) * size;
            let buried = rng.range(0.3, 0.5);
            let centre = ground_height(at.x, at.y) + squash.y * (1.0 - 2.0 * buried);
            let transform = Mat4::from_translation(Vec3::new(at.x, centre, at.y))
                * Mat4::from_rotation_y(rng.range(0.0, std::f32::consts::TAU))
                * Mat4::from_rotation_x(rng.signed() * 0.25)
                * Mat4::from_scale(squash);
            let mesh = boulders[rng.index(boulders.len())];
            garden.place(mesh, stone, transform);
            set.push((at, size));
        };

    // A pair at each end of the bridge, either side of where it lands.
    let (south, north) = garden.site.bridge;
    for end in [south, north] {
        for side in [-1.0, 1.0] {
            let at = end + Vec2::new(side * rng.range(1.0, 1.25), rng.signed() * 0.3);
            let size = rng.range(0.42, 0.6);
            place(garden, &mut rng, &mut set, at, size);
            let beside = at + Vec2::new(side * rng.range(0.45, 0.7), rng.signed() * 0.5);
            let small = size * rng.range(0.45, 0.7);
            place(garden, &mut rng, &mut set, beside, small);
        }
    }

    // Groups along the water's edge, where the plan's drift of stone lies.
    let [x0, z0, x1, z1] = pond_bounds();
    let mut groups: Vec<Vec2> = Vec::new();
    for _ in 0..4000 {
        if groups.len() >= 13 {
            break;
        }
        let at = Vec2::new(rng.range(x0, x1), rng.range(z0, z1));
        let stony = fbm(Vec3::new(at.x * 0.14, 41.0, at.y * 0.14), 3, 891);
        if pond_distance(at).abs() < 0.3
            && stony > 0.05
            && to_segment(at, south, north) > 3.0
            && garden.site.path_distance(at) > PATH_WIDTH * 0.5 + 0.8
            && groups.iter().all(|g| g.distance(at) > 5.5)
        {
            groups.push(at);
        }
    }
    for group in groups {
        let count = 2 + rng.index(4);
        let mut size = rng.range(0.5, 0.85);
        for k in 0..count {
            // The largest first, the rest gathered about it, smaller each time.
            let at = if k == 0 {
                group
            } else {
                group
                    + Vec2::from_angle(rng.range(0.0, std::f32::consts::TAU)) * rng.range(0.45, 1.1)
            };
            if pond_distance(at) < -0.9
                || set.iter().any(|&(s, r)| s.distance(at) < (r + size) * 0.55)
            {
                continue;
            }
            place(garden, &mut rng, &mut set, at, size);
            size *= rng.range(0.55, 0.8);
        }
    }
    println!("  {} stones", set.len());
}
