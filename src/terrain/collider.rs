use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

pub fn terrain_collider(
    vertices: &Vec<[f32; 3]>,
    indices: &Vec<u32>,
) -> (Collider, Vec<[u32; 3]>) {
    let t0 = std::time::Instant::now();
    let vertices_for_collider: Vec<Vec3> = vertices.iter()
        .map(|v| Vec3::new(v[0], v[1], v[2]))
        .collect();

    let mut triangles = Vec::new();
    for chunk in indices.chunks(3) {
        if chunk.len() == 3 {
            triangles.push([chunk[0], chunk[1], chunk[2]]);
        }
    }

    let trimesh_collider = match Collider::trimesh(vertices_for_collider, triangles.clone()) {
        Ok(collider) => collider,
        Err(e) => {
            eprintln!("Failed to create terrain trimesh collider: {:?}, using box fallback", e);
            Collider::cuboid(25.0, 0.1, 25.0)  // Simple fallback collider
        }
    };
    let t1 = std::time::Instant::now();
    println!("Collider generation took {:.3} ms", (t1 - t0).as_secs_f64() * 1000.0);
    (trimesh_collider, triangles)
}
