// Import statements - bring in code from other modules and crates
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;

use crate::planisphere;
use crate::game_object::EntitySubpixelPosition;
use crate::game_object::{MouseTrackerObject, ObjectShape, ObjectDefinition, CollisionBehavior, ExistenceConditions,
                            spawn_template_scene, ObjectTemplates, despawn_unified_objects_from_name};
use crate::player::Player;

// Submodule declarations
pub mod generation;
pub mod mesh;
pub mod texture;
pub mod collider;

// Re-exports so all public API remains accessible via `use crate::terrain::...`
pub use generation::{create_terrain_gnomonic_rectangular, create_terrain_simple, compute_mesh_async};
pub use mesh::terrain_mesh;
pub use texture::{select_texture_from_rgba, determine_landscape_element_from_rgba};
pub use collider::terrain_collider;

// Keep the deterministic_random private re-export for use within this module only
use texture::deterministic_random;

/// Tile Component - Marks entities as part of the terrain
/// This is attached to terrain entities so agents can detect when they touch the ground
#[derive(Component)]
pub struct Tile;

/// Resource to track which subpixels are currently rendered in the terrain
/// Objects will only be visible if their subpixel is in this set
#[derive(Resource, Default, Clone)]
pub struct RenderedSubpixels {
    pub subpixels: Vec<(usize, usize, usize, [(f64, f64); 4])>,
}

/// Resource to map triangle indices to their corresponding subpixel coordinates
/// Each index i in the vector corresponds to triangle i, and contains the (i,j,k) subpixel coordinates
#[derive(Resource, Default, Clone)]
pub struct TriangleSubpixelMapping {
    pub triangle_to_subpixel: Vec<(usize, usize, usize)>,
}

impl TriangleSubpixelMapping {
    pub fn new() -> Self {
        Self {
            triangle_to_subpixel: Vec::new(),
        }
    }
}

impl RenderedSubpixels {
    pub fn new() -> Self {
        Self {
            subpixels: Vec::new(),
        }
    }

    pub fn update_rendered_subpixels(&mut self, subpixels: &[(usize, usize, usize, [(f64, f64); 4])]) {
        self.subpixels.clear();
        for (i, j, k, _corners) in subpixels {
            self.subpixels.push((*i, *j, *k, *_corners));
        }
    }
}

pub fn ijk_to_world(
    i: i32,
    j: i32,
    k: i32,
    planisphere: &crate::planisphere::Planisphere,
    terrain_center: &TerrainCenter
) -> Vec3 {
    // Use the proper subpixel_to_geo method instead of manually averaging corners
    // This handles edge cases like longitude discontinuities correctly
    let (center_lon, center_lat) = planisphere.subpixel_to_geo(i as usize, j as usize, k as usize);

    // Convert the geographic center to world coordinates using the same method as terrain generation
    let (world_x, world_y) = planisphere.geo_to_gnomonic(
        center_lon,
        center_lat,
        terrain_center.longitude,
        terrain_center.latitude
    );

    // Return as Vec3 (Y=0 for ground level, could be modified for elevation)
    Vec3::new(world_x as f32 + 0.5 * planisphere.mean_tile_size as f32, 0.0, world_y as f32 + 0.5 * planisphere.mean_tile_size as f32)
}

// Usage in your terrain spawning
pub fn entities_in_rendered_subpixels(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    rendered_subpixels: ResMut<RenderedSubpixels>,
    planisphere: Res<planisphere::Planisphere>,
    terrain_center: ResMut<TerrainCenter>,
    object_templates: Res<ObjectTemplates>,
    query: Query<(Entity, &mut Transform, &ObjectDefinition), (Without<Player>, Without<MouseTrackerObject>)>,
) -> Vec<Entity> {
    const SPAWN_THRESHOLD: f64 = 0.999;
    let mut entities = Vec::new();
    despawn_unified_objects_from_name(commands, "Tree", query);
    for subpixel_pos in rendered_subpixels.subpixels.iter() {
        let rdm0 = deterministic_random(subpixel_pos.0, subpixel_pos.1, subpixel_pos.2);
        let (red, green, blue, alpha) = planisphere.get_rgba_at_subpixel(subpixel_pos.0 as i32, subpixel_pos.1 as i32, subpixel_pos.2);
        if rdm0 > SPAWN_THRESHOLD && 1. - alpha > 0.5 {
            let entity = spawn_template_scene(
                commands,
                materials,
                &planisphere,
                &terrain_center,
                &object_templates.tree,
                (subpixel_pos.0 as usize, subpixel_pos.1 as usize, subpixel_pos.2 as usize),
                0.0, // y_offset
                CollisionBehavior::Static, // Static collision for trees
                ()
            );
            entities.push(entity);
        }
    }
    entities
}


/// Resource to track terrain center changes for object repositioning
#[derive(Resource, Default)]
pub struct TerrainCenter {
    pub longitude: f64,
    pub latitude: f64,
    pub subpixel: (usize, usize, usize),
    pub max_subpixel_distance: usize,
    pub last_recreation_time: f32,
    /// Which distance metric to use when selecting subpixels for rendering
    pub distance_method: planisphere::DistanceMethod,
    /// Set to true to force a terrain rebuild on the next frame (e.g. after changing distance_method)
    pub force_recreation: bool,
    pub rendered_subpixels: RenderedSubpixels,
    pub triangle_mapping: TriangleSubpixelMapping,
}

impl TerrainCenter {


    pub fn set_ijk(&mut self, i: usize, j: usize, k: usize, planisphere: &planisphere::Planisphere) {
        self.subpixel = (i, j, k);
        self.longitude = planisphere.subpixel_to_geo(i, j, k).0;
        self.latitude = planisphere.subpixel_to_geo(i, j, k).1;
        let current_time = std::time::Instant::now().elapsed().as_secs_f32();
        self.last_recreation_time = current_time;
    }

}

/// Helper function to find the nearest free subpixel position using spiral search
/// This ensures agents don't respawn on top of each other during terrain recreation
#[allow(dead_code)]
fn find_nearest_free_subpixel(
    planisphere: &planisphere::Planisphere,
    desired_i: usize,
    desired_j: usize,
    desired_k: usize,
    occupied_positions: &std::collections::HashSet<(usize, usize, usize)>,
    terrain_config: &crate::TerrainConfig,
) -> (usize, usize, usize) {
    // If the desired position is free, use it
    if !occupied_positions.contains(&(desired_i, desired_j, desired_k)) {
        return (desired_i, desired_j, desired_k);
    }

    // Spiral search outward from the desired position
    let max_radius = terrain_config.agent_search_radius; // Maximum search radius from config
    let width = planisphere.get_width_pixels();
    let height = planisphere.get_height_pixels();
    let subpixel_divisions = 8; // From main.rs

    for radius in 1..=max_radius {
        // Search in a spiral pattern around the desired position
        for dy in -(radius as i32)..=(radius as i32) {
            for dx in -(radius as i32)..=(radius as i32) {
                // Only check positions on the current radius boundary
                if dx.abs() != radius as i32 && dy.abs() != radius as i32 {
                    continue;
                }

                // Calculate new pixel coordinates
                let new_i = (desired_i as i32 + dx).max(0).min(width as i32 - 1) as usize;
                let new_j = (desired_j as i32 + dy).max(0).min(height as i32 - 1) as usize;

                // Try different subpixel positions within this pixel
                for k in 0..subpixel_divisions {
                    let candidate = (new_i, new_j, k);
                    if !occupied_positions.contains(&candidate) {
                        return candidate;
                    }
                }
            }
        }
    }

    // If no free position found, return the desired position anyway
    // This is a fallback that shouldn't happen in normal gameplay
    println!("WARNING: Could not find free subpixel position near ({},{},{}), using original",
             desired_i, desired_j, desired_k);
    (desired_i, desired_j, desired_k)
}

