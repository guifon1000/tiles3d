use bevy::prelude::*;
use bevy::pbr::wireframe::Wireframe;
use bevy_rapier3d::prelude::*;

use crate::planisphere;
use super::{TerrainCenter, RenderedSubpixels, TriangleSubpixelMapping, Tile};
use super::mesh::terrain_mesh;
use super::collider::terrain_collider;

/// Refactor your compute_mesh to return both the mesh and the updates
pub fn compute_mesh_async(
    planisphere: &planisphere::Planisphere,
    subpixel: (usize, usize, usize),
    max_subpixel_distance: usize,
) -> (Mesh, RenderedSubpixels, TriangleSubpixelMapping) {
    let subpixels = planisphere.get_subpixels_by_distance_method(
        subpixel.0,
        subpixel.1,
        subpixel.2,
        max_subpixel_distance,
        crate::planisphere::DistanceMethod::Chebyshev
    );
    let mut rendered_subpixels = RenderedSubpixels::new();
    rendered_subpixels.subpixels = subpixels.clone();
    let lonlat = planisphere.subpixel_to_geo(subpixel.0, subpixel.1, subpixel.2);
    let (vertices, indices, uvs, mapping) = terrain_mesh(planisphere, subpixels, lonlat);
    let triangle_map = TriangleSubpixelMapping { triangle_to_subpixel: mapping };
    let (trimesh_collider, _triangles) = terrain_collider(&vertices, &indices);

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default()
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh.compute_smooth_normals();

    let _ = trimesh_collider; // collider is computed inside terrain_collider but not returned here
    (mesh, rendered_subpixels, triangle_map)
}

/// Create a very simple terrain using Bevy's built-in plane
/// This is the simplest way to create a flat surface
/// Uses Bevy's Plane3d primitive instead of manually creating vertices
pub fn create_terrain_simple(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // Create plane using Bevy's built-in primitive
    let plane_mesh = meshes.add(Plane3d::default().mesh().size(50.0, 50.0));

    // Create material
    let plane_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.7, 0.3), // Green
        metallic: 0.0,
        perceptual_roughness: 0.8,
        cull_mode: None,
        ..default()
    });

    // Spawn the plane
    commands.spawn((
        Mesh3d(plane_mesh),
        MeshMaterial3d(plane_material),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        Collider::cuboid(25.0, 0.1, 25.0), // Simple box collider
        RigidBody::Fixed,
        Tile,
        Wireframe,
    ));
}

/// Create terrain using rectangular (Chebyshev) distance pattern
pub fn create_terrain_gnomonic_rectangular(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    planisphere: &planisphere::Planisphere,
    terrain_center: &mut TerrainCenter,
    mut asset_tracker: Option<&mut ResMut<crate::TerrainAssetTracker>>,
    time: &Res<Time>,
) {
    let t0 = std::time::Instant::now();
    let method = terrain_center.distance_method;
    let subpixels = planisphere.get_subpixels_by_distance_method(
        terrain_center.subpixel.0,
        terrain_center.subpixel.1,
        terrain_center.subpixel.2,
        terrain_center.max_subpixel_distance,
        method);

    println!("Generated {} subpixels within distance {} using method {:?}", subpixels.len(), terrain_center.max_subpixel_distance, method);
    println!("center at {} {} {}", terrain_center.subpixel.0, terrain_center.subpixel.1, terrain_center.subpixel.2);
    let t1 = std::time::Instant::now();
    println!("Subpixel generation took {:.3} ms", (t1 - t0).as_secs_f64() * 1000.0);

    if subpixels.is_empty() {
        println!("ERROR: No subpixels generated! Falling back to simple terrain.");
        create_terrain_simple(commands, meshes, materials);
        return;
    } else {
        terrain_center.rendered_subpixels.update_rendered_subpixels(&subpixels);
    }

    let _t0 = std::time::Instant::now();
    // Update the rendered subpixels in terrain_center
    let lonlat = (terrain_center.longitude, terrain_center.latitude);
    let (vertices, indices, uvs, mapping) = terrain_mesh(planisphere, subpixels, lonlat);

    terrain_center.triangle_mapping.triangle_to_subpixel = mapping;

    let (trimesh_collider, triangles) = terrain_collider(&vertices, &indices);

    println!("Physics collider created with {} triangles (should match mapping size)", triangles.len());

    let t0 = std::time::Instant::now();
    let mut terrain_mesh_obj = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default()
    );
    let vertex_count = vertices.len();
    let triangle_count = indices.len() / 3;
    terrain_mesh_obj.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    terrain_mesh_obj.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    terrain_mesh_obj.insert_indices(bevy::render::mesh::Indices::U32(indices));
    terrain_mesh_obj.compute_smooth_normals();

    let terrain_mesh_handle = meshes.add(terrain_mesh_obj);
    let t1 = std::time::Instant::now();
    println!("Mesh creation took {:.3} ms for {} vertices and {} triangles", (t1 - t0).as_secs_f64() * 1000.0, vertex_count, triangle_count);

    // === TEXTURE ATLAS LOADING ===
    // Load the 256x256 pixel texture atlas containing all terrain textures
    // This atlas is a 16x16 grid where each 16x16 pixel tile represents a different terrain type
    // Generated by assets/textures/atlas_creator.py from individual texture files
    let tile_texture: Handle<Image> = asset_server.load("textures/texture_atlas.png");

    // Store atlas texture handle in asset tracker (reusable across terrain recreations)
    if let Some(asset_tracker) = asset_tracker.as_deref_mut() {
        if asset_tracker.texture_atlas.is_none() {
            asset_tracker.texture_atlas = Some(tile_texture.clone());
            println!("Stored texture atlas handle in asset tracker");
        }
    }

    // === MATERIAL SETUP FOR TERRAIN TEXTURES ===
    // Configure the standard material for terrain rendering
    let terrain_material_handle = materials.add(StandardMaterial {
        // Enable texture atlas for terrain textures
        base_color_texture: Some(tile_texture),

        // BRIGHTNESS: Normal base color for realistic terrain appearance
        // Values > 1.0 make textures brighter, 1.0 is natural brightness
        base_color: Color::srgb(1.0, 1.0, 1.0), // White base color to show textures properly

        // METALLIC SHINE: Keep terrain non-metallic for natural appearance
        // 0.0 = completely non-metallic (like dirt/grass), 1.0 = pure metal
        metallic: 0.1, // Minimal metallic shine for natural terrain

        // SURFACE ROUGHNESS: Natural terrain surface properties
        // 0.0 = mirror-like, 1.0 = completely rough/matte
        perceptual_roughness: 0.8, // Rough surface for natural terrain look

        // CULLING: Disable back-face culling to render both sides of terrain faces
        // Useful for debugging and ensuring terrain is visible from all angles
        cull_mode: None,

        // TRANSPARENCY: Enable alpha blending for transparent texture areas
        // Allows texture atlas tiles to have transparent borders
        alpha_mode: AlphaMode::Blend,

        // EMISSIVE GLOW: Disabled for natural terrain appearance
        emissive: LinearRgba::BLACK, // No emissive glow for realistic terrain

        // Use default values for other material properties
        ..default()
    });

    // Track terrain assets for cleanup
    if let Some(asset_tracker) = asset_tracker.as_deref_mut() {
        asset_tracker.terrain_meshes.push(terrain_mesh_handle.clone());
        asset_tracker.terrain_materials.push(terrain_material_handle.clone());
        println!("Tracked terrain mesh and material handles ({} meshes, {} materials total)",
                 asset_tracker.terrain_meshes.len(), asset_tracker.terrain_materials.len());
    }

    // Spawn single terrain entity
    let terrain_entity = commands.spawn((
        Mesh3d(terrain_mesh_handle),
        MeshMaterial3d(terrain_material_handle),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        RigidBody::Fixed,
        trimesh_collider,
        Tile,
        // Wireframe, // Disabled wireframe for normal terrain rendering
    )).id();

    println!("Spawned terrain entity: {:?}", terrain_entity);

    let t0 = std::time::Instant::now();
    // Update triangle mapping in terrain_center
    println!("Updated triangle mapping with {} triangles for terrain center ({:.6}, {:.6})",
        terrain_center.triangle_mapping.triangle_to_subpixel.len(), terrain_center.longitude, terrain_center.latitude);
    let t1 = std::time::Instant::now();
    println!("Triangle mapping update took {:.3} ms", (t1 - t0).as_secs_f64() * 1000.0);

    println!("=== TERRAIN MESH DEBUG ===");
    println!("Generated single terrain mesh with {} vertices", vertex_count);
    println!("Mesh has {} triangles", triangle_count);
    println!("Terrain entity ID: {:?}", terrain_entity);
    println!("==========================");

    let _ = time; // suppress unused warning - kept for API compatibility
}
