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

/// Determine what type of landscape element should be placed based on RGBA values
/// This function analyzes the color data from the geographic map to decide
/// what decorative elements (trees, rocks, etc.) should spawn at this location
pub fn determine_landscape_element_from_rgba(_red: f64, _green: f64, _blue: f64, alpha: f64, i: usize, j: usize, k: usize) -> Option<(String, f32)> {
    // Simple logic: use alpha channel and subpixel position to determine element type
    // You can expand this to create complex biome-based element placement
    
    let position_hash = ((i * 7919) ^ (j * 6131) ^ (k * 4801)) % 1000;
    
    if alpha > 0.7 && position_hash < 5 {  // Reduced from 50 to 5 (10x fewer trees)
        Some(("tree".to_string(), 2.0)) // Tree with scale 2.0
    } else if alpha > 0.5 && position_hash < 10 { // Reduced from 80 to 10 (8x fewer rocks)
        Some(("rock".to_string(), 1.0)) // Rock with scale 1.0
    } else if alpha > 0.3 && position_hash < 20 {
        Some(("stone".to_string(), 0.5)) // Stone with scale 0.5
    } else {
        None // No landscape element at this position
    }
}

/// Create landscape elements (trees, rocks, decorative objects) across the terrain
/// These elements are placed based on the actual terrain triangles using the triangle mapping
pub fn create_landscape_elements(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: &Planisphere,
    center_lon: f64,
    center_lat: f64,
    _terrain_config: &crate::TerrainConfig, // Not used anymore since we use triangle mapping
    triangle_mapping: &crate::terrain::TriangleSubpixelMapping,
    mut asset_tracker: Option<&mut ResMut<crate::TerrainAssetTracker>>, // Optional asset tracker for cleanup
) {
    println!("Creating landscape elements using terrain triangle mapping with {} triangles", triangle_mapping.triangle_to_subpixel.len());
    
    // Create reusable mesh handles to prevent asset accumulation
    let tree_mesh = meshes.add(Cuboid::new(0.3, 3.0, 0.3)); // Standard tree size
    let small_tree_mesh = meshes.add(Cuboid::new(0.3, 1.5, 0.3)); // Small tree
    let rock_mesh = meshes.add(Sphere::new(0.5)); // Standard rock
    let stone_mesh = meshes.add(Cuboid::new(0.4, 0.4, 0.4)); // Standard stone
    
    // Create reusable material handles
    let tree_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.6, 0.0), // Green for trees
        metallic: 0.1,
        perceptual_roughness: 0.8,
        ..default()
    });
    let rock_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.5), // Gray for rocks  
        metallic: 0.1,
        perceptual_roughness: 0.8,
        ..default()
    });
    let stone_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.7, 0.6, 0.4), // Brown for stones
        metallic: 0.1,
        perceptual_roughness: 0.8,
        ..default()
    });
    
    // Track landscape assets for cleanup
    if let Some(asset_tracker) = asset_tracker.as_deref_mut() {
        asset_tracker.landscape_meshes.extend_from_slice(&[
            tree_mesh.clone(), small_tree_mesh.clone(), rock_mesh.clone(), stone_mesh.clone()
        ]);
        asset_tracker.landscape_materials.extend_from_slice(&[
            tree_material.clone(), rock_material.clone(), stone_material.clone()
        ]);
        println!("Tracked landscape assets ({} meshes, {} materials total)", 
                 asset_tracker.landscape_meshes.len(), asset_tracker.landscape_materials.len());
    }
    
    // Create landscape elements only on subpixel coordinates that have actual terrain triangles
    // Use a set to track unique (i,j,k) coordinates from the triangle mapping
    let mut processed_subpixels = std::collections::HashSet::new();
    let mut elements_created = 0;
    
    for &(i, j, k) in &triangle_mapping.triangle_to_subpixel {
        // Skip if we already processed this subpixel coordinate
        if processed_subpixels.contains(&(i, j, k)) {
            continue;
        }
        processed_subpixels.insert((i, j, k));
        
        // Additional throttling - only process every 10th subpixel for performance
        if (i + j + k) % 10 != 0 {
            continue;
        }
        
        // Skip if too far from planisphere bounds
        if i >= planisphere.width_pixels || j >= planisphere.height_pixels {
            continue;
        }
        
        // Get RGBA data for this position
        let (red, green, blue, alpha) = planisphere.get_rgba_at_subpixel(i, j, k);
        
        // Determine if we should place a landscape element here
        if let Some((element_type, scale)) = determine_landscape_element_from_rgba(red, green, blue, alpha, i, j, k) {
            // Convert subpixel coordinates to world position
            // First convert to geographic coordinates, then to world coordinates
            let (lon, lat) = planisphere.subpixel_to_geo(i, j, k);
            let (world_x, world_z) = planisphere.geo_to_gnomonic(lon, lat, center_lon, center_lat);
            
            // Choose mesh and material based on element type and scale (reusing assets)
            let (mesh_handle, material_handle, element_color) = match element_type.as_str() {
                "tree" => {
                    let mesh = if scale > 1.5 { tree_mesh.clone() } else { small_tree_mesh.clone() };
                    (mesh, tree_material.clone(), Color::srgb(0.0, 0.6, 0.0))
                },
                "rock" => (rock_mesh.clone(), rock_material.clone(), Color::srgb(0.5, 0.5, 0.5)),
                "stone" => (stone_mesh.clone(), stone_material.clone(), Color::srgb(0.7, 0.6, 0.4)),
                _ => (stone_mesh.clone(), stone_material.clone(), Color::srgb(0.8, 0.4, 0.2)), // Default
            };
            
            // Calculate ground height (use a simple elevation for now)
            let ground_height = 0.0; // TODO: Get actual terrain elevation
            let y_offset = match element_type.as_str() {
                "tree" => 1.5 * scale, // Trees are taller, offset upward
                "rock" => 0.5 * scale,  // Rocks sit mostly on ground
                "stone" => 0.2 * scale, // Stones sit on ground
                _ => 0.5 * scale,
            };
            
            // Spawn the landscape element
            commands.spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(material_handle),
                Transform::from_translation(Vec3::new(
                    world_x as f32,
                    ground_height + y_offset,
                    world_z as f32
                )),
                RigidBody::Fixed, // Landscape elements are static
                Collider::from(match element_type.as_str() {
                    "tree" => Collider::cuboid(0.15, 1.5 * scale, 0.15),
                    "rock" => Collider::ball(0.5 * scale),
                    "stone" => Collider::cuboid(0.2 * scale, 0.2 * scale, 0.2 * scale),
                    _ => Collider::cuboid(0.5 * scale, 0.5 * scale, 0.5 * scale),
                }),
                LandscapeElement {
                    _element_type: element_type.clone(),
                    _color: element_color,
                },
                DistanceLOD {
                    _high_detail_distance: 20.0,
                    medium_detail_distance: 50.0,
                    low_detail_distance: 100.0,
                    cull_distance: 150.0,
                },
            ));
            
            elements_created += 1;
        }
    }
    
    println!("Created {} landscape elements", elements_created);
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

