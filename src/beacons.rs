use bevy::prelude::*;
use crate::planisphere::Planisphere;

/// Component marker for debug visualization beacons
/// These are visual indicators placed around the terrain for debugging purposes
#[derive(Component, Debug)]
pub struct DebugBeacon {
    pub _beacon_type: String,   // Type of beacon (e.g., "tile_center", "pixel_border", "player_marker") (prefixed with _ to indicate intentionally unused)
}

/// Component marker for the beacon that follows the player's current tile
/// This beacon snaps to the center of whichever tile the player is currently in
#[derive(Component)]
pub struct PlayerTileBeacon;

/// Component marker for the beacon that shows the terrain center
/// This beacon remains at the center point used for terrain generation
#[derive(Component)]
pub struct TerrainCenterBeacon;

/// Enhanced beacon with more properties for complex visualization
#[derive(Component, Debug, Clone)]
pub struct EnhancedBeacon {
    pub _beacon_type: String,
    pub _color: Color,
    pub _subpixel_info: Option<(usize, usize, usize)>,
    pub _scale: Vec3,
    pub _animated: bool,      // Whether to animate the beacon
}

/// Create debug beacons around the terrain for visualization and debugging
/// These beacons help visualize the tile structure and coordinate system
pub fn create_debug_beacons(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: &Planisphere,
    center_lon: f64,
    center_lat: f64,
    terrain_config: &crate::TerrainConfig,
) {
    println!("Creating debug beacons around center ({:.6}°, {:.6}°)", center_lat, center_lon);
    
    // Get terrain center in subpixel coordinates
    let (center_i, center_j, center_k) = planisphere.geo_to_subpixel(center_lon, center_lat);
    
    // Create a small grid of beacons around the center
    let beacon_radius = terrain_config.beacon_radius;
    let mut beacons_created = 0;
    
    for di in -(beacon_radius as i32)..=(beacon_radius as i32) {
        for dj in -(beacon_radius as i32)..=(beacon_radius as i32) {
            // Skip some positions to avoid cluttering
            if (di + dj) % 3 != 0 {
                continue;
            }
            
            let i = (center_i as i32 + di) as usize;
            let j = (center_j as i32 + dj) as usize;
            let k = center_k;
            
            // Skip if outside bounds
            if i >= planisphere.width_pixels || j >= planisphere.height_pixels {
                continue;
            }
            
            // Convert to world coordinates
            let (lon, lat) = planisphere.subpixel_to_geo(i, j, k);
            let (world_x, world_z) = planisphere.geo_to_gnomonic(lon, lat, center_lon, center_lat);
            let ground_height = 0.0; // TODO: Get actual terrain elevation
            
            // Choose beacon color based on position
            let beacon_color = if di == 0 && dj == 0 {
                Color::srgb(1.0, 0.0, 0.0) // Red for center
            } else if di == 0 {
                Color::srgb(0.0, 1.0, 0.0) // Green for vertical axis
            } else if dj == 0 {
                Color::srgb(0.0, 0.0, 1.0) // Blue for horizontal axis
            } else {
                Color::srgb(1.0, 1.0, 0.0) // Yellow for other positions
            };
            
            // Create beacon mesh (tall, thin cylinder)
            let beacon_mesh = meshes.add(Cylinder::new(0.1, 2.0));
            let beacon_material = materials.add(StandardMaterial {
                base_color: beacon_color,
                emissive: Color::srgb(
                    beacon_color.to_srgba().red * 0.5,
                    beacon_color.to_srgba().green * 0.5,
                    beacon_color.to_srgba().blue * 0.5
                ).into(), // Glowing effect
                ..default()
            });
            
            commands.spawn((
                Mesh3d(beacon_mesh),
                MeshMaterial3d(beacon_material),
                Transform::from_translation(Vec3::new(
                    world_x as f32,
                    ground_height + 1.0, // Raise above ground
                    world_z as f32
                )),
                DebugBeacon {
                    _beacon_type: "tile_debug".to_string(),
                },
            ));
            
            beacons_created += 1;
        }
    }
    
    println!("Created {} debug beacons", beacons_created);
}

/// Create the player tile beacon - a visual indicator of the player's current tile
/// This beacon follows the player and snaps to tile centers
pub fn create_player_tile_beacon(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    println!("Creating player tile beacon");
    
    // Create a distinctive beacon that will follow the player's tile
    let beacon_mesh = meshes.add(Cylinder::new(0.2, 1.5));
    let beacon_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.0, 0.0), // Bright red
        emissive: Color::srgb(0.5, 0.0, 0.0).into(), // Red glow
        ..default()
    });
    
    commands.spawn((
        Mesh3d(beacon_mesh),
        MeshMaterial3d(beacon_material),
        Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)), // Start at origin
        PlayerTileBeacon,
    ));
}

/// Create the terrain center beacon - shows the center point of terrain generation
pub fn create_terrain_center_beacon(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    println!("Creating terrain center beacon");
    
    // Create a beacon that marks the terrain generation center
    let beacon_mesh = meshes.add(Sphere::new(0.5));
    let beacon_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 1.0, 1.0), // Cyan
        emissive: Color::srgb(0.0, 0.3, 0.3).into(), // Cyan glow
        metallic: 0.8,
        ..default()
    });
    
    commands.spawn((
        Mesh3d(beacon_mesh),
        MeshMaterial3d(beacon_material),
        Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)), // Start at origin, slightly raised
        TerrainCenterBeacon,
    ));
}

