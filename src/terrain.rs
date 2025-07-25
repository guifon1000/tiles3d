// Import statements - bring in code from other modules and crates
use bevy::prelude::*;           // Bevy game engine core functionality
use bevy_rapier3d::prelude::*;  // Physics engine for 3D collision detection
use bevy::pbr::wireframe::Wireframe; // Wireframe rendering for debugging/visualization

// Import the planisphere module for gnomonic projection
use crate::planisphere;
use crate::beacons::PlayerTileBeacon;
// use crate::TerrainAssets;

/// Tile Component - Marks entities as part of the terrain
/// This is attached to terrain entities so agents can detect when they touch the ground
#[derive(Component)]
pub struct Tile;

// Components for landscape/beacons have been moved to their respective modules

/// SubpixelPosition Component - Tracks an object's "home" position in the (i,j,k) subpixel coordinate system
/// For agents, this represents their respawn location when terrain is recreated
/// For static objects, this is their fixed position for visibility calculations
#[derive(Component, Clone, Debug)]
pub struct SubpixelPosition {
    pub i: usize,  // Horizontal pixel index
    pub j: usize,  // Vertical pixel index 
    pub k: usize,  // Subpixel index within the pixel
}

impl SubpixelPosition {
    pub fn new(i: usize, j: usize, k: usize) -> Self {
        Self { i, j, k }
    }
}

// Object system components have been moved to objects.rs module

/// Resource to track which subpixels are currently rendered in the terrain
/// Objects will only be visible if their subpixel is in this set
#[derive(Resource, Default)]
pub struct RenderedSubpixels {
    pub subpixels: std::collections::HashSet<(usize, usize, usize)>,
    pub center_i: usize,
    pub center_j: usize, 
    pub center_k: usize,
    pub max_distance: usize,
}

/// Resource to map triangle indices to their corresponding subpixel coordinates
/// Each index i in the vector corresponds to triangle i, and contains the (i,j,k) subpixel coordinates
#[derive(Resource, Default)]
pub struct TriangleSubpixelMapping {
    pub triangle_to_subpixel: Vec<(usize, usize, usize)>,
    pub mesh_generation_time: f64,
    pub terrain_center_lon: f64,
    pub terrain_center_lat: f64,
}

/// Performance tracking resource
#[derive(Resource, Default)]
pub struct PerformanceStats {
    pub _visible_landscape_elements: usize, // Prefixed with _ to indicate intentionally unused
    pub _total_landscape_elements: usize,
    pub _culled_by_distance: usize,
    pub _culled_by_terrain: usize,
}

impl RenderedSubpixels {
    pub fn new() -> Self {
        Self {
            subpixels: std::collections::HashSet::new(),
            center_i: 0,
            center_j: 0,
            center_k: 0,
            max_distance: 0,
        }
    }
    
    pub fn is_visible(&self, i: usize, j: usize, k: usize) -> bool {
        self.subpixels.contains(&(i, j, k))
    }
    
    pub fn update_rendered_subpixels(&mut self, subpixels: &[(usize, usize, usize, [(f64, f64); 4])]) {
        self.subpixels.clear();
        for (i, j, k, _corners) in subpixels {
            self.subpixels.insert((*i, *j, *k));
        }
    }
}

/// Generate a deterministic random value (0.0 to 1.0) based on (i,j,k) coordinates
/// This ensures consistent landscape element placement across terrain regenerations
fn deterministic_random(i: usize, j: usize, k: usize) -> f64 {
    // Improved hash function with better mixing to avoid patterns
    // Based on xxHash and other high-quality hash functions
    
    // Convert coordinates to u64 for better mixing
    let mut hash = (i as u64).wrapping_mul(0x9E3779B185EBCA87); // Large prime
    hash ^= (j as u64).wrapping_mul(0xC2B2AE3D27D4EB4F);      // Another large prime
    hash ^= (k as u64).wrapping_mul(0x165667B19E3779F9);      // Another large prime
    
    // Additional mixing steps to break patterns
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x3C79AC492BA7B653);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0x1C69B3F74AC4AE35);
    hash ^= hash >> 27;
    
    // Convert to 0.0-1.0 range
    (hash as f64) / (u64::MAX as f64)
}

/// Determine landscape element type based on RGBA channel values and random probability
///
/// # Parameters
/// * `red` - Red channel value (0.0 to 1.0)
/// * `green` - Green channel value (0.0 to 1.0)  
/// * `blue` - Blue channel value (0.0 to 1.0)
/// * `alpha` - Alpha channel value (0.0 to 1.0)
/// * `i`, `j`, `k` - Subpixel coordinates for deterministic randomness
///
/// # Returns
/// Option containing (element_type, y_offset) or None if no landscape element

// ===== UNIFIED OBJECT SPAWNING FUNCTIONS =====

/// Convert tile indices to world coordinates using planisphere projection

// Object helper functions have been moved to objects.rs module

// Object spawning functions have been moved to objects.rs, landscape.rs, and beacons.rs modules

// Enhanced beacon spawning function has been moved to beacons.rs module

// All object spawning functions have been moved to dedicated modules:
// - Enhanced landscape elements -> landscape.rs
// - Beacon functions -> beacons.rs  
// - Generic object functions -> objects.rs

pub fn determine_landscape_element_from_rgba(_red: f64, _green: f64, _blue: f64, alpha: f64, i: usize, j: usize, k: usize) -> Option<(String, f32)> {
    // Get deterministic random value for this position
    let random_value = deterministic_random(i, j, k);
    
    // Use alpha channel to determine potential landscape element type
    let element_type = if alpha >= 0.8 && alpha <= 1.0 {
        // High alpha values = potential trees
        Some(("tree".to_string(), 0.6))
    } else if alpha >= 0.6 && alpha < 0.8 {
        // Medium-high alpha values = potential rocks
        Some(("rock".to_string(), 0.3))
    } else if alpha >= 0.3 && alpha < 0.6 {
        // Medium alpha values = potential stones
        Some(("stone".to_string(), 0.15))
    } else {
        // Low alpha values = no landscape element
        None
    };
    
    // If we have a potential element, use random probability to decide if it actually appears
    if let Some((elem_type, y_offset)) = element_type {
        // Different spawn probabilities for different elements (very sparse distribution)
        let spawn_probability = match elem_type.as_str() {
            "tree" => 0.003,  // 0.3% chance for trees
            "rock" => 0.006,  // 0.6% chance for rocks  
            "stone" => 0.010, // 1.0% chance for stones
            _ => 0.003,
        };
        
        // Debug output for first few elements (disabled for performance)
        // if i < 260 && j < 135 && k < 5 {
        //     println!("DEBUG: alpha={:.3}, type={}, random={:.3}, prob={:.3}, spawn={}", 
        //              alpha, elem_type, random_value, spawn_probability, random_value < spawn_probability);
        // }
        
        if random_value < spawn_probability {
            Some((elem_type, y_offset))
        } else {
            None
        }
    } else {
        None
    }
}

/// Select texture atlas tile index based on RGBA color values from geographic map data
/// 
/// This is the core texture selection function that determines which texture from the
/// 16x16 texture atlas (256 total textures) should be applied to each terrain subpixel.
/// The selection is based on RGBA color data extracted from sphere_texture.png.
/// 
/// # How It Works:
/// 1. Each pixel in sphere_texture.png represents a geographic location
/// 2. The RGBA values of that pixel encode terrain type information
/// 3. This function converts those RGBA values into a texture atlas index (0-255)
/// 4. The index determines which 16x16 texture tile from texture_atlas.png is used
/// 
/// # Current Implementation:
/// - Uses only the RED channel for texture selection (ignoring green, blue, alpha)
/// - Maps red values 0.0-1.0 to texture indices 0-9 (only 10 of 256 available textures)
/// - Uses simple threshold-based selection with 0.1 increments
/// 
/// # Available Textures (in texture_atlas.png):
/// The atlas contains these terrain textures:
/// - 0-9: deepwater, dirt, drygrass, eastgrass, grass, greenstone, ice, lava, lavastone, moss
/// - 10+: mossystone, northgrass, pavedstone, rawstone, sand, snow, southgrass, water, westgrass
/// 
/// # Parameters
/// * `red` - Red channel value (0.0 to 1.0) from corresponding map pixel
/// * `_green` - Green channel value (0.0 to 1.0) - currently unused but available
/// * `_blue` - Blue channel value (0.0 to 1.0) - currently unused but available  
/// * `_alpha` - Alpha channel value (0.0 to 1.0) - currently unused but available
///
/// # Returns
/// Texture atlas tile index (0 to 255 for a 16x16 atlas, currently returns 0-9)
/// 
/// # Example Usage in Terrain Generation:
/// ```rust
/// let (red, green, blue, alpha) = planisphere.get_rgba_at_subpixel(i, j, k);
/// let tile_index = select_texture_from_rgba(red, green, blue, alpha);
/// // tile_index is used to calculate UV coordinates in the texture atlas
/// ```
/// 
/// # Future Improvements:
/// - Could use all 4 RGBA channels for more complex terrain classification
/// - Could utilize more of the 256 available texture slots
/// - Could implement biome-based selection using multiple channels
/// - Could add noise/randomization for texture variety
pub fn select_texture_from_rgba(red: f64, _green: f64, _blue: f64, _alpha: f64) -> usize {
    // Current implementation: Simple red-channel-based texture selection
    // Maps red values to texture indices using threshold ranges
    
    let texture_index = if red < 0.1 {
        0  // Very dark red -> texture 0 (e.g., deep water)
    } else if red < 0.2 {
        1  // Dark red -> texture 1 (e.g., dirt) 
    } else if red < 0.3 {
        2  // Low red -> texture 2 (e.g., dry grass)
    } else if red < 0.4 {
        3  // Medium-low red -> texture 3 (e.g., regular grass)
    } else if red < 0.5 {
        4  // Medium red -> texture 4 (e.g., green stone)   
    } else if red < 0.6 {
        5  // Medium-high red -> texture 5 (e.g., moss)
    } else if red < 0.7 {
        6  // High red -> texture 6 (e.g., sand)
    } else if red < 0.8 {
        7  // Higher red -> texture 7 (e.g., stone)
    } else if red < 0.9 {
        8  // Very high red -> texture 8 (e.g., snow)
    } else {
        9  // Maximum red -> texture 9 (e.g., lava)
    };
    
    // Debug output to track texture selection (remove in production)
    // eprintln!("DEBUG: red={:.3} -> texture_index={}", red, texture_index);
    
    texture_index
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
    center_lon: f64,
    center_lat: f64,
    max_subpixel_render_distance: usize,
    planisphere: &planisphere::Planisphere,
    rendered_subpixels: Option<&mut ResMut<RenderedSubpixels>>,
    triangle_mapping: Option<&mut ResMut<TriangleSubpixelMapping>>,
    asset_tracker: Option<&mut ResMut<crate::TerrainAssetTracker>>,
) {
    create_terrain_gnomonic_with_distance_method(
        commands, meshes, materials, asset_server,
        center_lon, center_lat, max_subpixel_render_distance,
        planisphere, rendered_subpixels, triangle_mapping,
        crate::planisphere::DistanceMethod::Chebyshev,
        asset_tracker,
    )
}


pub fn create_terrain_gnomonic_with_distance_method(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    center_lon: f64,                     // Center longitude for projection
    center_lat: f64,                     // Center latitude for projection
    max_subpixel_render_distance: usize,       // Maximum subpixel distance from center
    planisphere: &planisphere::Planisphere, // Reference to planisphere
    rendered_subpixels: Option<&mut ResMut<RenderedSubpixels>>, // Optional resource to update
    triangle_mapping: Option<&mut ResMut<TriangleSubpixelMapping>>, // Optional triangle mapping to update
    distance_method: crate::planisphere::DistanceMethod, // Distance calculation method
    mut asset_tracker: Option<&mut ResMut<crate::TerrainAssetTracker>>, // Optional asset tracker for cleanup
) {
    // Use the provided center coordinates for terrain generation
    let (actual_center_lon, actual_center_lat) = (center_lon, center_lat);
    println!("Creating terrain with center: Lat: {:.6}°, Lon: {:.6}°", actual_center_lat, actual_center_lon);
    
    // Find the corresponding subpixel coordinates for the terrain center
    let (center_i, center_j, center_k) = planisphere.geo_to_subpixel(center_lon, center_lat);
    
    // Get subpixels using the specified distance method
    let subpixels = planisphere.get_subpixels_by_distance_method(center_i, center_j, center_k, max_subpixel_render_distance, distance_method);
    println!("Generated {} subpixels within distance {} using method {:?}", subpixels.len(), max_subpixel_render_distance, distance_method);
    
    if subpixels.is_empty() {
        println!("ERROR: No subpixels generated! Falling back to simple terrain.");
        create_terrain_simple(commands, meshes, materials);
        return;
    }
    
    // Update the rendered subpixels resource if provided
    if let Some(rendered_subpixels) = rendered_subpixels {
        rendered_subpixels.center_i = center_i;
        rendered_subpixels.center_j = center_j;
        rendered_subpixels.center_k = center_k;
        rendered_subpixels.max_distance = max_subpixel_render_distance;
        rendered_subpixels.update_rendered_subpixels(&subpixels);
        println!("Updated RenderedSubpixels resource with {} subpixels", subpixels.len());
    }
    
    // Create single mesh with all subpixels
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut vertex_index = 0u32;
    let mut uvs = Vec::new();
    
    // Initialize triangle-to-subpixel mapping if provided
    let mut triangle_to_subpixel = Vec::new();
    
    for (_i, _j, _k, corners) in &subpixels {
        let (i, j, k) = (*_i, *_j, *_k);
        let current_pixel_norm_lat = j as f64 / planisphere.height_pixels as f64;
        let current_latitude = current_pixel_norm_lat * 180.0 - 90.0;
        let current_lon_subdivisions = (planisphere.subpixel_divisions as f64 * current_latitude.to_radians().cos()).max(1.0) as usize;
        // Create vertices for this subpixel
        for (lon, lat) in corners.iter() {
            let (x, y) = planisphere.geo_to_gnomonic(*lon, *lat, actual_center_lon, actual_center_lat);
            vertices.push([x as f32, 0.0, y as f32]);
        }
        let atlas_size = 16; // 16x16 grid

        // Texture selection mode - set to true for RGBA-based, false for border-based
        let use_rgba_texture_selection = true;
        
        let tile_index = if use_rgba_texture_selection {
            // RGBA-based texture selection
            let (red, green, blue, alpha) = planisphere.get_rgba_at_subpixel(i, j, k);
            
            // Debug output for first few subpixels
            if vertex_index < 20 {
                println!("Subpixel ({},{},{}) RGBA: ({:.3},{:.3},{:.3},{:.3})", 
                         i, j, k, red, green, blue, alpha);
            }
            
            select_texture_from_rgba(red, green, blue, alpha)
        } else {
            // Original border-based texture selection
            let mut tile_index = 5; // default texture
            
            //north border
            if k%planisphere.subpixel_divisions == 0 {
                tile_index = 15; //north
            }

            //south border
            if k%planisphere.subpixel_divisions == planisphere.subpixel_divisions-1 {
                tile_index = 12;
            }
            
            //west border
            if k/planisphere.subpixel_divisions==0  {
                tile_index = 13;
            }        

            //east border
            if k/planisphere.subpixel_divisions==current_lon_subdivisions-1 {
                tile_index = 7;
            }
            
            tile_index
        };


        let tile_u = (tile_index % atlas_size) as f32 / atlas_size as f32;
        let tile_v = (tile_index / atlas_size) as f32 / atlas_size as f32;
        let tile_size = 1.0 / atlas_size as f32;
        

        // UVs for this quad
        uvs.push([tile_u, tile_v]); // bottom-left
        uvs.push([tile_u + tile_size, tile_v]); // bottom-right
        uvs.push([tile_u + tile_size, tile_v + tile_size]); // top-right
        uvs.push([tile_u, tile_v + tile_size]); // top-left
        
        // Create triangles (two triangles per quad)
        indices.extend_from_slice(&[
            vertex_index, vertex_index + 1, vertex_index + 2,
            vertex_index, vertex_index + 2, vertex_index + 3
        ]);
        
        // Map both triangles to this subpixel (i, j, k)
        triangle_to_subpixel.push((i, j, k)); // Triangle 1
        triangle_to_subpixel.push((i, j, k)); // Triangle 2
        
        vertex_index += 4;
    }
    
    // Create collision data for physics BEFORE moving vertices
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
    
    println!("Physics collider created with {} triangles (should match mapping size)", triangles.len());
    println!("Triangle index range: 0 to {}", triangles.len() - 1);
    println!("First few triangles: {:?}", &triangles[0..5.min(triangles.len())]);
    println!("Last few triangles: {:?}", &triangles[triangles.len().saturating_sub(5)..]);

    // Create the combined mesh
    let mut terrain_mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default()
    );
    let vertex_count = vertices.len();
    let triangle_count = indices.len() / 3;
    terrain_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    terrain_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    terrain_mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    terrain_mesh.compute_smooth_normals();
    
    let terrain_mesh_handle = meshes.add(terrain_mesh);
    
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
    
    // Update triangle mapping resource if provided
    if let Some(triangle_mapping) = triangle_mapping {
        triangle_mapping.triangle_to_subpixel = triangle_to_subpixel;
        triangle_mapping.mesh_generation_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        triangle_mapping.terrain_center_lon = center_lon;
        triangle_mapping.terrain_center_lat = center_lat;
        println!("Updated triangle mapping with {} triangles for terrain center ({:.6}, {:.6})", 
            triangle_mapping.triangle_to_subpixel.len(), center_lon, center_lat);
    }
    
    println!("=== TERRAIN MESH DEBUG ===");
    println!("Generated single terrain mesh with {} vertices", vertex_count);
    println!("Mesh has {} triangles", triangle_count);
    println!("Terrain entity ID: {:?}", terrain_entity);
    println!("==========================");
}

/// System to manage object visibility based on rendered terrain
/// This system runs every frame and shows/hides objects based on whether their subpixel is rendered
/// Regular objects use full terrain visibility, agents use 2/3 radius visibility
pub fn manage_object_visibility(
    rendered_subpixels: Res<RenderedSubpixels>,
    mut regular_objects: Query<(&mut Visibility, &SubpixelPosition), (Without<Tile>, Without<PlayerTileBeacon>, Without<crate::agent::Agent>)>,
    mut agents: Query<(&mut Visibility, &SubpixelPosition), With<crate::agent::Agent>>,
) {
    if rendered_subpixels.is_changed() {
        // Handle regular objects (full terrain visibility)
        for (mut visibility, subpixel_pos) in regular_objects.iter_mut() {
            let is_visible = rendered_subpixels.is_visible(subpixel_pos.i, subpixel_pos.j, subpixel_pos.k);
            *visibility = if is_visible { Visibility::Visible } else { Visibility::Hidden };
        }
        
        // Handle agents (2/3 radius visibility based on terrain mesh size)
        let agent_max_distance = (rendered_subpixels.max_distance as f64 * 2.0 / 3.0) as usize;
        let mut visible_agents = 0;
        let mut total_agents = 0;
        
        for (mut visibility, subpixel_pos) in agents.iter_mut() {
            total_agents += 1;
            
            // Calculate distance from terrain center to agent position using the same method as terrain generation
            let center_i = rendered_subpixels.center_i;
            let center_j = rendered_subpixels.center_j;
            let center_k = rendered_subpixels.center_k;
            
            let dist_i = if subpixel_pos.i > center_i { subpixel_pos.i - center_i } else { center_i - subpixel_pos.i };
            let dist_j = if subpixel_pos.j > center_j { subpixel_pos.j - center_j } else { center_j - subpixel_pos.j };
            
            // Calculate base distance in terms of full pixels
            let pixel_distance = dist_i + dist_j;
            
            // Calculate subpixel distance using same method as terrain generation
            let mut subpixel_distance = pixel_distance * 8; // subpixel_divisions from main.rs
            
            // If in the same pixel, calculate subpixel-level distance
            if subpixel_pos.i == center_i && subpixel_pos.j == center_j {
                // Direct subpixel distance calculation
                let sub_distance = if subpixel_pos.k > center_k { 
                    subpixel_pos.k - center_k 
                } else { 
                    center_k - subpixel_pos.k 
                };
                subpixel_distance = sub_distance;
            }
            
            // Agent is visible if within 2/3 of the terrain mesh radius
            let is_visible = subpixel_distance <= agent_max_distance;
            
            if is_visible {
                visible_agents += 1;
            }
            
            *visibility = if is_visible { Visibility::Visible } else { Visibility::Hidden };
        }
        
        println!("Updated visibility: {} regular objects, {}/{} agents visible (2/3 radius: {} pixels)", 
                 rendered_subpixels.subpixels.len(), visible_agents, total_agents, agent_max_distance);
        
        // Debug: If agents are missing, report it
        if total_agents == 0 {
            println!("WARNING: No agents found in visibility system!");
        } else if visible_agents == 0 && total_agents > 0 {
            println!("WARNING: All {} agents are hidden (outside visibility radius)", total_agents);
        }
    }
}

/// Resource to track terrain center changes for object repositioning
#[derive(Resource, Default)]
pub struct TerrainCenter {
    pub longitude: f64,
    pub latitude: f64,
    pub subpixel: (usize, usize, usize),
    pub world_position: Vec3,
    pub max_subpixel_distance: usize,
    pub last_recreation_time: f32,
    pub terrain_recreated: bool,
}

impl TerrainCenter {
    pub fn new() -> Self {
        Self {
            longitude: 0.0,
            latitude: 0.0,
            subpixel: (0, 0, 0),
            world_position: Vec3::ZERO,
            max_subpixel_distance: 62,
            last_recreation_time: -10.0,
            terrain_recreated: false,
        }
    }
    
    pub fn reset_flag(&mut self) {
        self.terrain_recreated = false;
    }
}

/// Helper function to find the nearest free subpixel position using spiral search
/// This ensures agents don't respawn on top of each other during terrain recreation
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




