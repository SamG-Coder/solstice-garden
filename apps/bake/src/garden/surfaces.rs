//! Surfaces made for the garden, beside those of the procedural library: each built from its
//! real shape first, in metres, and coloured by looking at that shape.

use glam::Vec3;
use solstice_procgen::field::{fbm, smoothstep, Field, Stones};
use solstice_procgen::surface::{Shade, Surface};

/// Metres one tile of the path's gravel covers. Its stones are about a centimetre, so its tile
/// is half the usual: two millimetres a texel at 1024.
pub const GRAVEL_TILE_METRES: f32 = 2.0;

/// A raked garden path: pea gravel, rounded stones of a centimetre or so in the warm buffs and
/// greys of river shingle, packed tight, with finer grit between them and a little fallen
/// litter trodden in.
pub fn pea_gravel(seed: u32) -> Surface {
    let tile_metres = GRAVEL_TILE_METRES;
    let stones = |period: i32, radius: f32, salt: u32| Stones {
        period,
        seed: seed + salt,
        radius,
        tile_metres,
        coverage: 1.0,
    };
    // Three sizes, each its own scatter, so the small fill between the large.
    let large = stones(120, 0.013, 900);
    let medium = stones(190, 0.009, 901);
    let small = stones(330, 0.0055, 902);
    // Where feet and the rake have left it uneven.
    let bed = fbm(4, 4, seed + 903);
    let grit = fbm(380, 2, seed + 904);
    let speckle = fbm(160, 3, seed + 905);
    // Broad drifts of colour: damp and dry, and where one barrow-load ends.
    let drift = fbm(3, 3, seed + 906).unit();
    let litter = fbm(9, 4, seed + 907);
    Surface::shaped(
        move |u, v| {
            let mut top = small.sample(u, v);
            for stone in [medium.sample(u, v), large.sample(u, v)] {
                if stone.height > top.height {
                    top = stone;
                }
            }
            let height = bed.at(u, v) * 0.012 + top.height + grit.at(u, v) * 0.0004;
            (height, if top.height > 0.0 { top.id } else { -1.0 })
        },
        move |u, v, id, relief| {
            // How far up out of the gaps this point stands.
            let exposed = smoothstep(-0.004, 0.001, relief.fine + relief.medium * 0.5);
            let earth = Vec3::new(0.07, 0.055, 0.04);
            if id < 0.0 {
                return Shade {
                    color: earth,
                    roughness: 0.97,
                    cavity: 0.2,
                };
            }
            // Buff and honey for the most part; some grey flint, a few dark and a few pale.
            let rock = if id < 0.5 {
                Vec3::new(0.36, 0.3, 0.215).lerp(Vec3::new(0.47, 0.41, 0.31), id / 0.5)
            } else if id < 0.78 {
                Vec3::new(0.31, 0.3, 0.28).lerp(Vec3::new(0.44, 0.42, 0.39), (id - 0.5) / 0.28)
            } else if id < 0.92 {
                Vec3::new(0.3, 0.2, 0.13).lerp(Vec3::new(0.2, 0.15, 0.11), (id - 0.78) / 0.14)
            } else {
                Vec3::new(0.56, 0.54, 0.5).lerp(Vec3::new(0.12, 0.12, 0.125), (id - 0.92) / 0.08)
            };
            let mottle = 0.9 + speckle.at(u + id * 3.0, v - id * 7.0) * 0.22;
            let damp = 0.86 + drift.at(u, v) * 0.24;
            // Dust and earth settle on the stones' shoulders and in the gaps.
            let dust = (1.0 - exposed) * 0.6;
            let mut color = (rock * mottle * damp).lerp(earth * 1.6, dust);
            // Here and there a scrap of dead leaf trodden in.
            let scrap =
                smoothstep(0.52, 0.6, litter.at(u, v)) * smoothstep(0.3, 0.6, speckle.at(v, u));
            color = color.lerp(Vec3::new(0.12, 0.075, 0.035), scrap * 0.7);
            Shade {
                color: color * (0.35 + exposed * 0.65),
                roughness: 0.68 + id * 0.2,
                cavity: 0.25 + exposed * 0.75,
            }
        },
    )
    .with_tile_metres(tile_metres)
}
