use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::planisphere::Planisphere;

/// Component marker for collectible items scattered around the terrain
#[derive(Component, Debug)]
pub struct Item {
    pub item_type: String,  // Type of item (e.g., "coin", "power-up", "resource")
    pub _value: i32,        // Value or quantity of the item (prefixed with _ to indicate intentionally unused)
    pub _color: Color,      // Color of the item for rendering (prefixed with _ to indicate intentionally unused)
}

/// Component for landscape elements like trees, rocks, and decorative objects
#[derive(Component, Debug)]
pub struct LandscapeElement {
    pub _element_type: String, // Type of element (e.g., "stone", "tree", "rock") (prefixed with _ to indicate intentionally unused)
    pub _color: Color,         // Color of the element (prefixed with _ to indicate intentionally unused)
}

/// Level-of-Detail system for landscape elements
#[derive(Component)]
pub struct DistanceLOD {
    pub _high_detail_distance: f32,   // Distance for full detail (prefixed with _ to indicate intentionally unused)
    pub medium_detail_distance: f32,  // Distance for reduced detail
    pub low_detail_distance: f32,     // Distance for minimal detail
    pub cull_distance: f32,           // Distance to hide completely
}

/// Enhanced landscape element with more properties
#[derive(Component, Debug, Clone)]
pub struct EnhancedLandscapeElement {
    pub _element_type: String,        // Type of element (prefixed with _ to indicate intentionally unused)
    pub _color: Color,                // Color (prefixed with _ to indicate intentionally unused)
    pub _scale: Vec3,                 // Scale (prefixed with _ to indicate intentionally unused)
    pub _distance_from_player: f32,   // Distance from player (prefixed with _ to indicate intentionally unused)
}



/// Create collectible items scattered around the terrain
/// Items can be picked up by players and agents for points or resources
pub fn create_items(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: &Planisphere,
    center_lon: f64,
    center_lat: f64,
    _terrain_config: &crate::TerrainConfig, // Not used anymore since we use triangle mapping
    triangle_mapping: &crate::terrain::TriangleSubpixelMapping,
) {
    println!("Creating items using terrain triangle mapping with {} triangles", triangle_mapping.triangle_to_subpixel.len());
    
    // Create reusable mesh handle to prevent asset accumulation
    let item_mesh = meshes.add(Sphere::new(0.3));
    
    // Create reusable material handles for different item types
    let coin_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 1.0, 0.0), // Gold
        emissive: Color::srgb(0.3, 0.3, 0.0).into(), // Gold glow
        metallic: 0.8,
        perceptual_roughness: 0.1,
        ..default()
    });
    let gem_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 1.0, 1.0), // Cyan
        emissive: Color::srgb(0.0, 0.3, 0.3).into(), // Cyan glow
        metallic: 0.8,
        perceptual_roughness: 0.1,
        ..default()
    });
    let powerup_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.0, 1.0), // Magenta
        emissive: Color::srgb(0.3, 0.0, 0.3).into(), // Magenta glow
        metallic: 0.8,
        perceptual_roughness: 0.1,
        ..default()
    });
    let resource_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 1.0, 0.0), // Green
        emissive: Color::srgb(0.0, 0.3, 0.0).into(), // Green glow
        metallic: 0.8,
        perceptual_roughness: 0.1,
        ..default()
    });
    
    // Create items only on subpixel coordinates that have actual terrain triangles
    // Use a set to track unique (i,j,k) coordinates from the triangle mapping
    let mut processed_subpixels = std::collections::HashSet::new();
    let mut items_created = 0;
    
    for &(i, j, k) in &triangle_mapping.triangle_to_subpixel {
        // Skip if we already processed this subpixel coordinate
        if processed_subpixels.contains(&(i, j, k)) {
            continue;
        }
        processed_subpixels.insert((i, j, k));
        
        // Skip if outside planisphere bounds
        if i >= planisphere.width_pixels || j >= planisphere.height_pixels {
            continue;
        }
        
        // Sparse item placement using position-based randomization
        let item_hash = ((i * 8191) ^ (j * 6367) ^ (k * 5273)) % 1000;
        if item_hash > 15 { // Only 1.5% chance of item placement
            continue;
        }
        
        // Convert to world coordinates
        let (lon, lat) = planisphere.subpixel_to_geo(i, j, k);
        let (world_x, world_z) = planisphere.geo_to_gnomonic(lon, lat, center_lon, center_lat);
        let ground_height = 0.0; // TODO: Get actual terrain elevation
        
        // Determine item type and select reusable material based on hash
        let (item_type, item_color, item_value, material_handle) = match item_hash % 4 {
            0 => ("coin", Color::srgb(1.0, 1.0, 0.0), 10, coin_material.clone()),      // Gold coins
            1 => ("gem", Color::srgb(0.0, 1.0, 1.0), 50, gem_material.clone()),       // Cyan gems  
            2 => ("powerup", Color::srgb(1.0, 0.0, 1.0), 100, powerup_material.clone()),  // Magenta powerups
            _ => ("resource", Color::srgb(0.0, 1.0, 0.0), 25, resource_material.clone()),  // Green resources
        };
        
        // Spawn the item using reusable assets
        commands.spawn((
            Mesh3d(item_mesh.clone()),
            MeshMaterial3d(material_handle),
            Transform::from_translation(Vec3::new(
                world_x as f32,
                ground_height + 0.5, // Float slightly above ground
                world_z as f32
            )),
            RigidBody::Fixed,
            Sensor, // Items are sensors for pickup detection
            Collider::ball(0.5), // Slightly larger pickup radius
            Item {
                item_type: item_type.to_string(),
                _value: item_value,
                _color: item_color,
            },
        ));
        
        items_created += 1;
    }
    
    println!("Created {} items", items_created);
}

/// Update level-of-detail for landscape elements based on distance from player
pub fn update_landscape_lod(
    mut landscape_query: Query<(&mut Transform, &mut Visibility, &LandscapeElement, &DistanceLOD)>,
    player_query: Query<&Transform, (With<crate::player::Player>, Without<LandscapeElement>)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return; // No player found
    };
    
    let player_pos = player_transform.translation;
    
    for (mut transform, mut visibility, _element, lod) in landscape_query.iter_mut() {
        let distance = player_pos.distance(transform.translation);
        
        // Update visibility and scale based on distance
        if distance > lod.cull_distance {
            *visibility = Visibility::Hidden;
        } else if distance > lod.low_detail_distance {
            *visibility = Visibility::Visible;
            transform.scale = Vec3::splat(0.5); // Low detail - smaller scale
        } else if distance > lod.medium_detail_distance {
            *visibility = Visibility::Visible;
            transform.scale = Vec3::splat(0.75); // Medium detail
        } else {
            *visibility = Visibility::Visible;
            transform.scale = Vec3::splat(1.0); // High detail - full scale
        }
    }
}

/// Hide landscape elements that are outside the currently rendered terrain area
pub fn cull_landscape_by_terrain(
    mut landscape_query: Query<&mut Visibility, With<LandscapeElement>>,
    rendered_subpixels: Res<crate::terrain::RenderedSubpixels>,
) {
    // This system would need access to subpixel position of landscape elements
    // For now, we'll keep all landscape elements visible
    // In the future, you could track which elements are within rendered terrain bounds
    
    for mut visibility in landscape_query.iter_mut() {
        // Simple implementation - could be enhanced to check against rendered_subpixels
        *visibility = Visibility::Visible;
    }
    
    // Debug info
    if rendered_subpixels.subpixels.len() > 0 {
        println!("Landscape culling: {} rendered subpixels available", rendered_subpixels.subpixels.len());
    }
}

