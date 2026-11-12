//! The garden's ground: one mesh, fine where it matters and coarse where it doesn't.
//!
//! A pond's edge, a path's and a bed's are each a line a few centimetres wide, in a garden whose
//! hills run out a hundred and eighty metres. A grid fine enough for the one would spend
//! millions of triangles on the other, so the ground is a quadtree: squares split until each is
//! as small as its place asks, down to nine centimetres along those edges and up to several
//! metres on the far hills. Neighbours are kept within one level of each other, and each square
//! is drawn as a fan about its middle that takes in the extra corner where a finer neighbour
//! has one, so the mesh has no cracks (a restricted quadtree).

use super::site::{ground_height, pond_distance, Site, PATH_WIDTH, SIZE};
use glam::{Vec2, Vec3};
use rustc_hash::{FxHashMap, FxHashSet};
use solstice_builder::{Mesh, Vertex};

/// The finest squares are the terrain's edge halved this many times: 8.8 cm.
const FINEST: u32 = 12;

/// A square of the quadtree: its level, and its column and row among that level's squares.
type Square = (u32, u32, u32);

/// How large the ground's squares may be at `p`, in metres.
fn wanted_size(site: &Site, p: Vec2) -> f32 {
    let shore = pond_distance(p).abs();
    let path = (site.path_distance(p) - PATH_WIDTH * 0.5).abs();
    let bed = site.bed_depth(p).0.abs();
    let edge = shore.min(path).min(bed);
    // Lawns need little: grass covers them. The far hills need less.
    let far = p.length();
    let open = 0.3 + 0.04 * (far - 45.0).max(0.0);
    (0.085 + 0.3 * edge).min(open)
}

/// The length of a square's side at `level`.
fn side(level: u32) -> f32 {
    SIZE / (1u32 << level) as f32
}

/// Whether a square is too large for where it lies.
fn too_large(site: &Site, (level, column, row): Square) -> bool {
    if level >= FINEST {
        return false;
    }
    let size = side(level);
    let corner = Vec2::new(column as f32, row as f32) * size - Vec2::splat(SIZE * 0.5);
    let mut wanted = f32::MAX;
    for i in 0..3 {
        for j in 0..3 {
            let p = corner + Vec2::new(i as f32, j as f32) * (size * 0.5);
            wanted = wanted.min(wanted_size(site, p));
        }
    }
    // What is wanted changes by under a third of a metre a metre, so a square's samples can
    // miss by about a tenth of its side.
    size > wanted + size * 0.1
}

/// The leaf that covers the square at `(column, row)` of `level`, if one that large or larger
/// does: none if that place is split finer.
fn leaf_over(leaves: &FxHashSet<Square>, (level, column, row): Square) -> Option<Square> {
    (0..=level).rev().find_map(|l| {
        let key = (l, column >> (level - l), row >> (level - l));
        leaves.contains(&key).then_some(key)
    })
}

/// The squares of the ground, each as small as its place asks, and none more than one level
/// from its neighbours.
fn squares(site: &Site) -> FxHashSet<Square> {
    let mut leaves = FxHashSet::default();
    let mut stack = vec![(0, 0, 0)];
    while let Some(square) = stack.pop() {
        if too_large(site, square) {
            let (level, column, row) = square;
            for (dx, dz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                stack.push((level + 1, column * 2 + dx, row * 2 + dz));
            }
        } else {
            leaves.insert(square);
        }
    }
    // Split any square with a neighbour two or more levels finer, until none is left.
    let mut check: Vec<Square> = leaves.iter().copied().collect();
    while let Some(square) = check.pop() {
        if !leaves.contains(&square) {
            continue;
        }
        let (level, column, row) = square;
        let count = 1u32 << level;
        for (dx, dz) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let (c, r) = (column as i32 + dx, row as i32 + dz);
            if c < 0 || r < 0 || c >= count as i32 || r >= count as i32 {
                continue;
            }
            let Some(neighbour) = leaf_over(&leaves, (level, c as u32, r as u32)) else {
                continue;
            };
            if neighbour.0 + 1 < level {
                leaves.remove(&neighbour);
                let (l, nc, nr) = neighbour;
                for (ex, ez) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let child = (l + 1, nc * 2 + ex, nr * 2 + ez);
                    leaves.insert(child);
                    check.push(child);
                }
                // This square's other sides, and the split square's neighbours, are looked at
                // again through the children.
                check.push(square);
            }
        }
    }
    leaves
}

/// The ground as a mesh: each point's shares of gravel and soil in its texture coordinates, as
/// a blended ground material reads them.
pub fn mesh(site: &Site) -> Mesh {
    let leaves = squares(site);
    // Corners are named in half steps of the finest squares, so a middle has whole numbers too.
    let unit = SIZE / (1u32 << (FINEST + 1)) as f32;
    let mut mesh = Mesh::default();
    let mut indices: FxHashMap<(u32, u32), u32> = FxHashMap::default();
    let mut vertex = |mesh: &mut Mesh, at: (u32, u32)| {
        *indices.entry(at).or_insert_with(|| {
            let (x, z) = (
                at.0 as f32 * unit - SIZE * 0.5,
                at.1 as f32 * unit - SIZE * 0.5,
            );
            // The slope over a few centimetres, finer than any square.
            let step = 0.04;
            let normal = Vec3::new(
                ground_height(x - step, z) - ground_height(x + step, z),
                2.0 * step,
                ground_height(x, z - step) - ground_height(x, z + step),
            )
            .normalize();
            mesh.vertices.push(Vertex {
                position: Vec3::new(x, ground_height(x, z), z),
                normal,
                uv: site.ground_shares(Vec2::new(x, z)),
            });
            mesh.vertices.len() as u32 - 1
        })
    };

    let mut ordered: Vec<Square> = leaves.iter().copied().collect();
    ordered.sort_unstable();
    for (level, column, row) in ordered {
        let span = 1u32 << (FINEST + 1 - level);
        let (x0, z0) = (column * span, row * span);
        let half = span / 2;
        let count = 1u32 << level;
        // Whether the neighbour across a side is split finer than this square, which puts a
        // corner of its squares at the middle of the side.
        let finer = |dx: i32, dz: i32| {
            let (c, r) = (column as i32 + dx, row as i32 + dz);
            c >= 0
                && r >= 0
                && c < count as i32
                && r < count as i32
                && leaf_over(&leaves, (level, c as u32, r as u32)).is_none()
        };
        // Round the square, clockwise seen from above with z towards the viewer, which is
        // counter-clockwise in a right-handed world looking down -Y: so the fan faces up.
        let mut rim = vec![(x0, z0)];
        if finer(0, -1) {
            rim.push((x0 + half, z0));
        }
        rim.push((x0 + span, z0));
        if finer(1, 0) {
            rim.push((x0 + span, z0 + half));
        }
        rim.push((x0 + span, z0 + span));
        if finer(0, 1) {
            rim.push((x0 + half, z0 + span));
        }
        rim.push((x0, z0 + span));
        if finer(-1, 0) {
            rim.push((x0, z0 + half));
        }
        if rim.len() == 4 {
            // No finer neighbour: two triangles, split along the diagonal that bends least.
            let corners: Vec<u32> = rim.iter().map(|&at| vertex(&mut mesh, at)).collect();
            let y = |i: usize| mesh.vertices[corners[i] as usize].position.y;
            if (y(0) - y(2)).abs() <= (y(1) - y(3)).abs() {
                mesh.indices.extend_from_slice(&[
                    corners[0], corners[2], corners[1], corners[0], corners[3], corners[2],
                ]);
            } else {
                mesh.indices.extend_from_slice(&[
                    corners[0], corners[3], corners[1], corners[1], corners[3], corners[2],
                ]);
            }
            continue;
        }
        let middle = vertex(&mut mesh, (x0 + half, z0 + half));
        for k in 0..rim.len() {
            let a = vertex(&mut mesh, rim[k]);
            let b = vertex(&mut mesh, rim[(k + 1) % rim.len()]);
            mesh.indices.extend_from_slice(&[middle, b, a]);
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ground_faces_up_and_has_no_cracks() {
        let site = Site::new();
        let mesh = mesh(&site);
        assert!(mesh.triangle_count() > 200_000, "{}", mesh.triangle_count());
        // Every triangle faces up.
        for triangle in mesh.indices.chunks_exact(3) {
            let [a, b, c] = [0, 1, 2].map(|i| mesh.vertices[triangle[i] as usize].position);
            assert!((b - a).cross(c - a).y > 0.0, "a triangle faces down at {a}");
        }
        // Every edge inside the ground is shared by two triangles, each running it the other
        // way: a crack would leave an edge with one.
        let mut edges: FxHashMap<(u32, u32), i32> = FxHashMap::default();
        for triangle in mesh.indices.chunks_exact(3) {
            for k in 0..3 {
                let (a, b) = (triangle[k], triangle[(k + 1) % 3]);
                *edges.entry((a.min(b), a.max(b))).or_default() += if a < b { 1 } else { -1 };
            }
        }
        let half = SIZE * 0.5 - 1.0e-3;
        for (&(a, b), &balance) in &edges {
            let (pa, pb) = (
                mesh.vertices[a as usize].position,
                mesh.vertices[b as usize].position,
            );
            let on_rim = |p: Vec3| p.x.abs() > half || p.z.abs() > half;
            if !(on_rim(pa) && on_rim(pb)) {
                assert_eq!(balance, 0, "an open edge from {pa} to {pb}");
            }
        }
    }

    #[test]
    fn the_ground_is_fine_at_the_waters_edge_and_coarse_on_the_hills() {
        let site = Site::new();
        let leaves = squares(&site);
        let level_at = |x: f32, z: f32| {
            let n = 1u32 << FINEST;
            let c = (((x + SIZE * 0.5) / SIZE) * n as f32) as u32;
            let r = (((z + SIZE * 0.5) / SIZE) * n as f32) as u32;
            leaf_over(&leaves, (FINEST, c, r))
                .expect("a leaf covers every spot")
                .0
        };
        // On the shore where the bridge lands.
        let (south, _) = site.bridge;
        assert!(level_at(south.x + 3.0, south.y - 0.55) >= FINEST - 1);
        assert!(level_at(150.0, 150.0) <= 7);
    }
}
