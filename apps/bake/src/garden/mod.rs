//! A garden: lawns round a pond, willows on its banks and oaks behind, borders in flower, and a
//! path over a footbridge to a seat in the shade.
//!
//! [`site`] is the plan everything else asks: where the water is, the paths and the beds.
//! [`ground`] makes the land from it.

mod grass;
mod ground;
mod herbs;
mod planting;
mod rocks;
pub mod site;
mod surfaces;
mod timber;
mod trees;

use glam::{Mat4, Vec2};
use site::{ground_height, pond_bounds, pond_distance, Site, WATER_LEVEL};
use solstice_asset::scene::Viewpoint;
use solstice_asset::{Heightfield, Instance, Material, SceneAsset, StillWater};
use solstice_builder::{build_virtual_mesh, BuildOptions};
use solstice_procgen::surface::{Surface, TILE_METRES};
use solstice_procgen::surfaces as library;

/// A garden pond's water: dark with peat and leaf, so the bed shows only in the first hand's
/// depth at the margin and fades from there, and the rest is a mirror for the trees and sky.
const POND_WATER: &str = "\
absorption 2.1 1.5 2.0
scattering 0.012 0.017 0.008
turbidity 0.5
";

/// The scene so far, and the texture layers made for it, by name, so each is made once.
pub struct Garden {
    pub scene: SceneAsset,
    layers: Vec<String>,
    pub site: Site,
}

impl Garden {
    /// The texture layer of `name`, made by `make` the first time it is asked for.
    pub fn layer(&mut self, name: &str, make: impl FnOnce() -> Surface) -> i32 {
        if let Some(index) = self.layers.iter().position(|n| n == name) {
            return index as i32;
        }
        println!("  texture: {name}");
        self.layers.push(name.to_string());
        self.scene
            .textures
            .push(make().bake(self.scene.texture_size));
        self.scene.textures.len() as i32 - 1
    }

    /// Adds a mesh, and returns its index.
    pub fn mesh(&mut self, mesh: &solstice_builder::Mesh) -> u32 {
        self.scene
            .meshes
            .push(build_virtual_mesh(mesh, &BuildOptions::default()));
        self.scene.meshes.len() as u32 - 1
    }

    /// Adds a material, and returns its index.
    pub fn material(&mut self, material: Material) -> u32 {
        self.scene.materials.push(material);
        self.scene.materials.len() as u32 - 1
    }

    pub fn place(&mut self, mesh: u32, material: u32, transform: Mat4) {
        self.scene.instances.push(Instance {
            mesh,
            material,
            transform,
        });
    }
}

/// The ground for walking on: a grid of heights about a third of a metre apart.
fn ground_heights() -> Heightfield {
    let cells = 1024u32;
    let spacing = site::SIZE / cells as f32;
    let origin = -site::SIZE * 0.5;
    let mut heights = Vec::with_capacity(((cells + 1) * (cells + 1)) as usize);
    for row in 0..=cells {
        let z = origin + row as f32 * spacing;
        for column in 0..=cells {
            heights.push(ground_height(origin + column as f32 * spacing, z));
        }
    }
    Heightfield {
        columns: cells + 1,
        rows: cells + 1,
        origin: [origin, origin],
        spacing: [spacing, spacing],
        heights,
    }
}

/// Lays the land: turf, the paths' gravel and bare soil, blended by the plan.
fn add_ground(garden: &mut Garden) {
    println!("laying the ground...");
    let mesh = ground::mesh(&garden.site);
    println!("  {} triangles", mesh.triangle_count());
    // Turf first, then gravel and soil, in consecutive layers and materials, as the blend reads
    // them. Each darkened as it is seen outdoors beside real leaves.
    let first = garden.layer("turf", || library::grass(0));
    garden.layer("gravel", || surfaces::pea_gravel(0));
    garden.layer("soil", || library::dirt(0));
    let mut ground = 0;
    let tiles = [TILE_METRES, surfaces::GRAVEL_TILE_METRES, TILE_METRES];
    for (k, tone) in [0.66, 0.66, 0.36].into_iter().enumerate() {
        let material = garden.material(Material {
            base_color: [tone, tone, tone, 1.0],
            roughness: 1.0,
            uv_scale: 1.0 / tiles[k],
            // Turf lies under real blades, which should make its relief, not a picture of them.
            normal_strength: if k == 0 { 0.35 } else { 1.0 },
            texture_layer: first + k as i32,
            // Gravel is raked even: only turf and soil are patchy.
            macro_variation: if k == 1 { 0.25 } else { 1.0 },
            flags: if k == 0 { Material::BLENDS } else { 0 },
            ..Default::default()
        });
        if k == 0 {
            ground = material;
        }
    }
    let mesh = garden.mesh(&mesh);
    garden.place(mesh, ground, Mat4::IDENTITY);
}

/// Writes the plan as a picture seen from above, for checking it by eye: `metres` across,
/// centred on the pond, north up.
fn write_plan(site: &Site, path: &std::path::Path, metres: f32) -> std::io::Result<()> {
    const EDGE: usize = 1024;
    let mut pixels = Vec::with_capacity(EDGE * EDGE * 3);
    for row in 0..EDGE {
        for column in 0..EDGE {
            let p = Vec2::new(
                (column as f32 + 0.5) / EDGE as f32 - 0.5,
                (row as f32 + 0.5) / EDGE as f32 - 0.5,
            ) * metres;
            let height = ground_height(p.x, p.y);
            let shares = site.ground_shares(p);
            let mut color = glam::Vec3::new(0.25, 0.5, 0.2) * (0.8 + 0.12 * height);
            color = color.lerp(glam::Vec3::new(0.75, 0.72, 0.65), shares.x);
            color = color.lerp(glam::Vec3::new(0.35, 0.24, 0.15), shares.y);
            if pond_distance(p) < 0.0 {
                let depth = WATER_LEVEL - height;
                color = color.lerp(
                    glam::Vec3::new(0.1, 0.25, 0.45),
                    0.5 + 0.35 * depth.min(1.4),
                );
            }
            for tree in &site.specimens {
                let d = p.distance(tree.at);
                if d < 0.5 {
                    color = glam::Vec3::new(0.9, 0.2, 0.1);
                } else if d < 0.5 + 5.0 * tree.scale && d > 5.0 * tree.scale {
                    color = glam::Vec3::new(0.05, 0.25, 0.05);
                }
            }
            let (south, north) = site.bridge;
            if site::to_segment(p, south, north) < 0.7 {
                color = glam::Vec3::new(0.55, 0.38, 0.2);
            }
            // A cross every ten metres.
            let grid = (p / 10.0 - (p / 10.0).round()).abs() * 10.0;
            if grid.x < 0.12 && grid.y < 0.6 || grid.y < 0.12 && grid.x < 0.6 {
                color *= 0.6;
            }
            for channel in color.to_array() {
                pixels.push((channel.clamp(0.0, 1.0) * 255.0) as u8);
            }
        }
    }
    let mut file = format!("P6\n{EDGE} {EDGE}\n255\n").into_bytes();
    file.extend_from_slice(&pixels);
    std::fs::write(path, file)
}

/// Writes the ground to walk on, for a page that walks a visitor round: heights on a grid over
/// the middle of the garden, with the bridge's deck where it crosses the water, then the trunks
/// that stand in the way, each as x, z and radius. Little-endian throughout: `GWLK`, the
/// grid's samples a side, half its width in metres, the heights row by row from the north-west,
/// the number of trunks, and the trunks.
fn write_walk(site: &Site, path: &std::path::Path) -> std::io::Result<()> {
    const SAMPLES: u32 = 641;
    const HALF: f32 = 80.0;
    let mut bytes = b"GWLK".to_vec();
    bytes.extend_from_slice(&SAMPLES.to_le_bytes());
    bytes.extend_from_slice(&HALF.to_le_bytes());
    for row in 0..SAMPLES {
        let z = -HALF + 2.0 * HALF * row as f32 / (SAMPLES - 1) as f32;
        for column in 0..SAMPLES {
            let x = -HALF + 2.0 * HALF * column as f32 / (SAMPLES - 1) as f32;
            let height =
                timber::deck_height(site, Vec2::new(x, z)).unwrap_or_else(|| ground_height(x, z));
            bytes.extend_from_slice(&height.to_le_bytes());
        }
    }
    bytes.extend_from_slice(&(site.specimens.len() as u32).to_le_bytes());
    for tree in &site.specimens {
        let radius = match tree.species {
            "old_oak" => 0.85,
            "willow" => 0.7,
            _ => 0.3,
        } * tree.scale;
        for value in [tree.at.x, tree.at.y, radius] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    // Written before the scene is, so on a fresh checkout its folder may not be there yet.
    if let Some(folder) = path.parent().filter(|f| !f.as_os_str().is_empty()) {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(path, bytes)
}

pub fn build(
    texture_size: u32,
    plan: Option<&std::path::Path>,
    walk: Option<&std::path::Path>,
) -> SceneAsset {
    let site = Site::new();
    if let Some(path) = walk {
        match write_walk(&site, path) {
            Ok(()) => println!("wrote the ground to walk on to {}", path.display()),
            Err(error) => println!("warning: could not write {}: {error}", path.display()),
        }
    }
    if let Some(path) = plan {
        match write_plan(&site, path, 110.0) {
            Ok(()) => println!("wrote the plan to {}", path.display()),
            Err(error) => println!("warning: could not write the plan: {error}"),
        }
    }
    let mut garden = Garden {
        scene: SceneAsset {
            texture_size,
            ..Default::default()
        },
        layers: Vec::new(),
        site,
    };
    add_ground(&mut garden);
    trees::add(&mut garden);
    grass::add(&mut garden);
    planting::add(&mut garden);
    rocks::add(&mut garden);
    timber::add(&mut garden);

    let [x0, z0, x1, z1] = pond_bounds();
    garden.scene.still_water = Some(StillWater {
        ponds: vec![[x0, z0, x1, z1, WATER_LEVEL]],
        water: POND_WATER.to_string(),
    });
    garden.scene.ground = Some(ground_heights());
    // On the path in from the south, where the pond first opens out below.
    let eye = Vec2::new(1.6, 23.0);
    garden.scene.viewpoint = Some(Viewpoint {
        position: [eye.x, ground_height(eye.x, eye.y) + 1.65, eye.y],
        yaw: 0.02,
        pitch: -0.1,
    });
    garden.scene
}
