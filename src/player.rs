use std::marker;

// use bevy::math::ops::sqrt;
// Import statements - bring in code from other modules and crates
use bevy::prelude::*;           // Bevy game engine core functionality
use bevy::window::PrimaryWindow;
use bevy_rapier3d::prelude::*;  // Physics engine for 3D collision detection
use bevy_rapier3d::plugin::context::systemparams::ReadRapierContext;
use bevy::input::mouse::MouseMotion; // Mouse movement events
use crate::terrain::{self, RenderedSubpixels, Tile, TerrainCenter}; // Import Tile component and resources from terrain module
use crate::landscape::Item; // Import Item from landscape module
use crate::beacons::{PlayerTileBeacon}; // Import beacon components
use crate::game_object::ExistenceConditions;
// use crate::TerrainConfig;
use crate::planisphere::{self, geo_to_gnomonic_helper}; // Import planisphere for coordinate conversion
use crate::game_object::{despawn_unified_object_from_name, spawn_mousetracker_at_tile, spawn_mouse_tracker_at_world_position, 
                        spawn_terraincenter_at_world_position, CollisionBehavior, 
                        MouseTrackerObject, ObjectDefinition, ObjectPosition, ObjectShape,
                        spawn_playertracker_at_tile}; // Import game object definitions
// Note: Terrain configuration is now accessed via TerrainConfig resource instead of constants
// use crate::agent::Agent; // Import Agent component for shared positioning

/// Player Component - Marks an entity as player-controlled
/// Similar to Agent but with keyboard input instead of AI
#[derive(Component)]
pub struct Player {
    pub next_jump_time: f32,      // Timer: when can the player jump again?
    pub is_grounded: bool,        // Boolean: is the player touching the ground?
    pub facing_angle: f32,        // Float: current facing direction in radians (Y-axis rotation)
    pub mouse_sensitivity: f32,   // Float: how sensitive mouse movement is
    pub move_speed: f32,          // Float: how fast the player moves
    pub mouse_ray_hit: ObjectDefinition, // NEW: Object definition for player
}

#[derive(Bundle)]
pub struct PlayerBundle {
    pub mesh: Mesh3d, // 3D shape of the player
    pub material: MeshMaterial3d::<StandardMaterial>, // Material (color) of the player
    pub transform: Transform, // Position and rotation of the player in the world
    pub player: Player,
    pub player_inventory: PlayerInventory,
    pub player_raycast: PlayerRaycast,
    pub entity_position: EntitySubpixelPosition, // NEW: Shared positioning component
}

impl Default for PlayerBundle {
    fn default() -> Self {
        Self {
            mesh: Mesh3d::default(), // Placeholder, will be set later
            material: MeshMaterial3d::default(), // Placeholder, will be set later
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            player: Player {
                next_jump_time: 0.0,
                is_grounded: false,
                facing_angle: 0.0,
                mouse_sensitivity: 0.002,
                move_speed: 15.0,

                mouse_ray_hit: ObjectDefinition {
                    position: ObjectPosition::WorldCoordinates(Vec3::new(0.0, 0.0, 0.0)),
                    shape: ObjectShape::Sphere { radius: 0.1 },
                    color: Color::srgb(1.0, 0.0, 0.0), // Red color for ray hit
                    collision: CollisionBehavior::None,
                    existence_conditions: Some(ExistenceConditions::OnFrame), // Exists for the current frame only
                    object_type: "MouseTracker".to_string(),
                    scale: Vec3::new(1.0, 1.0, 1.0),
                    y_offset: 0.0,
                },
            },
            player_inventory: PlayerInventory::default(),
            player_raycast: PlayerRaycast {
                range: 10.0,
                hit_something: false,
                hit_distance: 0.0,
                hit_point: Vec3::ZERO,
                hit_normal: Vec3::ZERO,
                last_check_time: 0.0,
                check_interval: 0.05,
            },
            entity_position: EntitySubpixelPosition::default(), // NEW: Initialize shared positioning
        }
    }
}

/// PlayerSensor Component - Detects items to pick up for the player
#[derive(Component)]
pub struct PlayerSensor {
    pub parent_entity: Entity,    // Reference to the player that owns this sensor
}

/// PlayerInventory Component - Stores items the player has collected
#[derive(Component, Default, Debug)]
pub struct PlayerInventory {
    pub items: Vec<String>,       // Vec<String> is a dynamic array of text strings
}

/// Raycast Component - Handles raycasting for the player
/// This component allows the player to detect terrain and objects in their path
#[derive(Component)]
pub struct PlayerRaycast {
    pub range: f32,               // Maximum distance to cast the ray
    pub hit_something: bool,      // Whether the last raycast hit something
    pub hit_distance: f32,        // Distance to the hit point
    pub hit_point: Vec3,          // World coordinates of the hit point
    pub hit_normal: Vec3,         // Normal vector at the hit point
    pub last_check_time: f32,     // Time of last raycast check
    pub check_interval: f32,      // How often to perform raycasts (in seconds)
}

/// Marker component for the ray intersection visualization sphere
#[derive(Component)]
pub struct RayIntersectionMarker;

/// Marker component for the triangle's subpixel center visualization sphere
#[derive(Component)]
pub struct TriangleSubpixelMarker;

/// Shared trait for entities that have positioning capabilities

/// Shared component for entities that use raycast positioning
#[derive(Component, Clone, Debug)]
pub struct EntitySubpixelPosition {
    pub subpixel: (usize, usize, usize),
    pub geo_coords: (f64, f64),
    pub world_pos: Vec3,
    pub previous_subpixel: (usize, usize, usize),
    pub last_raycast_time: f32,
}

impl Default for EntitySubpixelPosition {
    fn default() -> Self {
        Self {
            subpixel: (256, 128, 0),
            geo_coords: (0.0, 0.0),
            world_pos: Vec3::ZERO,
            previous_subpixel: (256, 128, 0),
            last_raycast_time: 0.0,
        }
    }
}



pub fn cast_ray_from_camera(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    rapier_context: ReadRapierContext,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    marker_query: Query<Entity, With<RayIntersectionMarker>>,
    object_query: Query<(Entity, &ObjectDefinition)>,

    // subpixel_marker_query removed - using only triangle mapping
    triangle_marker_query: Query<Entity, With<TriangleSubpixelMarker>>,
    planisphere: Res<crate::planisphere::Planisphere>,
    terrain_center: Res<TerrainCenter>,
    triangle_mapping: Res<crate::terrain::TriangleSubpixelMapping>,
    terrain_entities: Query<Entity, With<crate::terrain::Tile>>,
) {
    let Ok(window) = windows.single() else { return; };
    let Ok((camera, camera_transform)) = cameras.single() else { return; };

    if let Some(cursor_position) = window.cursor_position() {
        // Create a ray from the camera to the cursor position
        if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) {
            // Get the rapier context
            let Ok(ctx) = rapier_context.single() else { return; };
            
            // Perform physics raycast
            let max_distance = 100.0;
            let solid = true;
            //let filter = // This is more efficient as it doesn't pre-collect entities
            let filter = QueryFilter::default();

            
            if let Some((entity, ray_intersection)) = ctx.cast_ray_and_get_normal(
                ray.origin,
                *ray.direction,
                max_distance,
                solid,
                filter,
            ) {
                // Calculate hit point
                let hit_point = ray.origin + *ray.direction * ray_intersection.time_of_impact;
                
                // Get triangle index and other intersection details
                let feature_info = format!("{:?}", ray_intersection.feature);
                let triangle_index = match &ray_intersection.feature {
                    _f if feature_info.contains("Face") => {
                        // Extract the numeric ID from the debug string
                        feature_info.chars()
                            .filter(|c| c.is_ascii_digit())
                            .collect::<String>()
                            .parse::<u32>()
                            .unwrap_or(0)
                    },
                    _ => 0,
                };
                let normal = ray_intersection.normal;
                
                // Debug: Show entity and triangle relationship
                let _terrain_count = terrain_entities.iter().count();
                let is_terrain_entity = terrain_entities.contains(entity);
                // eprintln!("from camera raycast: Hit entity {:?} (is_terrain: {}), triangle index {}, mapping size {}, terrain entities: {}", 
                //     entity, is_terrain_entity, triangle_index, triangle_mapping.triangle_to_subpixel.len(), terrain_count);
                
                // Additional debug info for offset detection
                if is_terrain_entity {
                    let mapping_size = triangle_mapping.triangle_to_subpixel.len() as u32;
                    if triangle_index >= mapping_size {
                        let multiple = triangle_index / mapping_size;
                        let remainder = triangle_index % mapping_size;
                        println!("INDEX ANALYSIS: {} = {}×{} + {} (multiple of mapping size + remainder)", 
                            triangle_index, multiple, mapping_size, remainder);
                    } else {
                        println!("INDEX OK: {} is within bounds [0, {})", triangle_index, mapping_size);
                    }
                }
                // Remove existing markers if they exist
                for (entity, def) in object_query.iter() {
                    if def.object_type.contains("MouseTracker") {
                        commands.entity(entity).despawn();
                    }
                }

                // Remove existing markers if they exist
                for marker_entity in marker_query.iter() {
                    commands.entity(marker_entity).despawn();
                }
                // subpixel_marker_query removed - using only triangle mapping
                for marker_entity in triangle_marker_query.iter() {
                    commands.entity(marker_entity).despawn();
                }
                
                // Spawn new intersection marker (red sphere)

                spawn_mouse_tracker_at_world_position(&mut commands, &mut meshes, &mut materials, Some(&planisphere), &terrain_center, hit_point);
                // Find closest subpixel to the hit point
                let closest_subpixel = find_closest_subpixel_to_world_point(
                    hit_point,
                    &planisphere,
                    &terrain_center,
                );
                
                if let Some((i, j, k)) = closest_subpixel {
                    // Calculate the center of this subpixel in world coordinates
                    // Blue marker removed - relying only on triangle mapping for accuracy
                    
                    // Try to get the subpixel from triangle mapping first
                    let mut triangle_subpixel_found = false;
                    
                    // CRITICAL: Detect and correct triangle index offset between Bevy mesh and Rapier collision
                    let adjusted_triangle_index = if is_terrain_entity {
                        // Check if triangle index is out of bounds (indicates offset)
                        if triangle_index >= triangle_mapping.triangle_to_subpixel.len() as u32 {
                            // Dynamic offset detection: find the correct offset
                            let mapping_size = triangle_mapping.triangle_to_subpixel.len() as u32;
                            let potential_offset = (triangle_index / mapping_size) * mapping_size;
                            let corrected_index = triangle_index - potential_offset;
                            
                            //println!("OFFSET DETECTED: Triangle index {} is out of bounds (mapping size: {})", triangle_index, mapping_size);
                            println!("Calculated offset: {}, Corrected index: {} -> {}", potential_offset, triangle_index, corrected_index);
                            
                            // Validate the corrected index is within bounds
                            if corrected_index < mapping_size {
                                corrected_index
                            } else {
                                println!("ERROR: Even after offset correction, index {} is still out of bounds!", corrected_index);
                                triangle_index // Use original if correction fails
                            }
                        } else {
                            triangle_index // Index is within bounds, use as-is
                        }
                    } else {
                        triangle_index // Non-terrain entity, use index directly
                    };
                    
                    // Check if triangle mapping is stale (from different terrain center)
                    let mapping_terrain_center_mismatch = 
                        (triangle_mapping.terrain_center_lon - terrain_center.longitude).abs() > 0.000001 ||
                        (triangle_mapping.terrain_center_lat - terrain_center.latitude).abs() > 0.000001;
                    
                    if mapping_terrain_center_mismatch {
                        println!("WARNING: Triangle mapping is stale! Mapping center: ({:.6}, {:.6}), Current center: ({:.6}, {:.6})",
                            triangle_mapping.terrain_center_lon, triangle_mapping.terrain_center_lat,
                            terrain_center.longitude, terrain_center.latitude);
                        println!("Using current terrain center for coordinate conversion to fix drift issue");
                    }
                    
                    if let Some(triangle_subpixel) = triangle_mapping.triangle_to_subpixel.get(adjusted_triangle_index as usize) {
                        let (tri_i, tri_j, tri_k) = *triangle_subpixel;
                        spawn_mousetracker_at_tile(&mut commands, &mut meshes, &mut materials, Some(&planisphere), &terrain_center, tri_i as usize, tri_j as usize, tri_k as usize);
                       
                        
                        //println!("Ray hit entity {:?} at position {:.2?} (distance: {:.2})", entity, hit_point, ray_intersection.time_of_impact);
                        //println!("Feature: {}, Triangle/Feature index: {} -> {}, Normal: {:.2?}", feature_info, triangle_index, adjusted_triangle_index, normal);
                        //println!("Triangle's subpixel: ({}, {}, {}) at world position {:.2?}", tri_i, tri_j, tri_k, triangle_subpixel_center);
                        //println!("Terrain center: ({:.2}, {:.2}), Mapping timestamp: {:.2}", terrain_center.center_lon, terrain_center.center_lat, triangle_mapping.mesh_generation_time);
                        triangle_subpixel_found = true;
                    }
                    
                    // Removed fallback - relying exclusively on triangle mapping for accuracy
                    if !triangle_subpixel_found {
                        //println!("Ray hit entity {:?} at position {:.2?} (distance: {:.2})", entity, hit_point, ray_intersection.time_of_impact);
                        //println!("Feature: {}, Triangle/Feature index: {}, Normal: {:.2?}", feature_info, triangle_index, normal);
                        //println!("Triangle mapping failed - no green marker will be shown");
                    }
                    
                    //println!("Closest subpixel: ({}, {}, {}) at world position Vec3({:.2}, {:.2}, {:.2})", i, j, k, 
                    //    calculate_subpixel_center_world_position(i, j, k, &planisphere, &terrain_center).x,
                    //    calculate_subpixel_center_world_position(i, j, k, &planisphere, &terrain_center).y,
                    //    calculate_subpixel_center_world_position(i, j, k, &planisphere, &terrain_center).z);
                }
            } else {
                // No hit - remove existing markers
                for marker_entity in marker_query.iter() {
                    commands.entity(marker_entity).despawn();
                }
                // subpixel_marker_query removed - using only triangle mapping
                for marker_entity in triangle_marker_query.iter() {
                    commands.entity(marker_entity).despawn();
                }
            }
        }
    }
}

pub fn calculate_subpixel_center_world_position(
    i: i32, 
    j: i32, 
    k: i32, 
    planisphere: &crate::planisphere::Planisphere, 
    terrain_center: &crate::player::TerrainCenter
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



/// Find the closest subpixel to a world position
fn find_closest_subpixel_to_world_point(
    world_pos: Vec3, 
    planisphere: &crate::planisphere::Planisphere, 
    terrain_center: &TerrainCenter
) -> Option<(i32, i32, i32)> {
    // Convert world position to geographic coordinates using inverse gnomonic projection
    let (lon, lat) = planisphere::gnomonic_to_geo_helper(
        world_pos.x as f64, 
        world_pos.z as f64,  // Note: Bevy uses Z for forward/back
        terrain_center.longitude, 
        terrain_center.latitude, 
        planisphere.radius
    );
    
    // Check if conversion was successful (not NaN)
    if !lon.is_finite() || !lat.is_finite() {
        return None;
    }
    
    // Convert geographic coordinates to subpixel coordinates
    let (i, j, k) = planisphere.geo_to_subpixel(lon, lat);
    
    Some((i as i32, j as i32, k as i32))
}



/// Function to create the player entity in the game world
pub fn create_player(
    commands: &mut Commands,                              // Commands let us spawn/despawn entities
    meshes: &mut ResMut<Assets<Mesh>>,                   // Collection of 3D meshes (shapes)
    materials: &mut ResMut<Assets<StandardMaterial>>,    // Collection of materials (colors/textures)
) -> Entity {
    // Create the 3D shape for our player - a capsule (like a pill shape)
    let player_mesh = meshes.add(Capsule3d::new(0.3, 0.8)); // radius 0.3, height 0.8
    // Create the material (color) for our player - red color
    let player_material = materials.add(Color::srgb(0.9, 0.1, 0.1)); // RGB: high red, low green/blue
    
    // Spawn the main player entity
    let player_id = commands.spawn((
        // Visual components - what the player sees
        PlayerBundle {
            mesh: Mesh3d(player_mesh),
            material: MeshMaterial3d(player_material),
            transform: Transform::from_translation(Vec3::new(0.0, 5.0, 0.0)), // Start 5 units above ground
            ..Default::default()
        },
        
        // Physics components - how the player interacts with the world
        RigidBody::Dynamic,                   // Can move and be affected by forces
        Collider::capsule_y(0.3, 0.4),      // Physics shape for collision detection
        Velocity { linvel: Vec3::new(0.0, -0.1, 0.0), angvel: Vec3::ZERO }, // Initial velocity
        ExternalImpulse::default(),          // Allows applying forces to move the player
        GravityScale(1.0),                   // How much gravity affects this player (1.0 = normal)
        Damping { linear_damping: 0.0, angular_damping: 0.1 }, // Resistance to movement
        
        // Lock rotation on X and Z axes, allow Y rotation for turning
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
        
        // Enable collision detection events
        ActiveEvents::COLLISION_EVENTS,      // Tells physics engine to send collision notifications
        ActiveCollisionTypes::all(),         // Detect all types of collisions
        
        // Add SubpixelPosition for terrain recreation repositioning
        crate::terrain::SubpixelPosition::new(1500, 750, 0), // Default center position (will be updated by tracking system)
    )).id(); // .id() gets the unique ID of the spawned entity

    // Add child entities to the player (sensors and visual markers)
    commands.entity(player_id).with_children(|parent| {
        // Item pickup sensor - invisible sphere that detects nearby items
        parent.spawn((
            Collider::ball(2.0),                        // Invisible sphere with radius 2.0
            PlayerSensor { parent_entity: player_id },  // Links this sensor to its parent player
            bevy_rapier3d::geometry::Sensor,            // Makes it non-solid (things pass through)
            ActiveEvents::COLLISION_EVENTS,             // Detects when items enter/leave
            ActiveCollisionTypes::all(),                // Detect all collision types
            Transform::from_xyz(0.0, 0.0, 0.0),        // Centered on player (relative position)
        ));
        
        // Visual marker to show which way the player is facing
        let marker_mesh = meshes.add(Sphere::new(0.1));                    // Small sphere
        let marker_material = materials.add(Color::srgb(1.0, 1.0, 0.0));   // Yellow color
        parent.spawn((
            Mesh3d(marker_mesh),
            MeshMaterial3d(marker_material),
            Transform::from_xyz(0.0, 0.2, -0.4),
        ));
    });

    player_id
}

/// Function to handle player movement with keyboard and mouse input
pub fn move_player(
    time: Res<Time>,                                    // Bevy's time resource
    keyboard_input: Res<ButtonInput<KeyCode>>,         // Keyboard input state
    mut mouse_motion: EventReader<MouseMotion>,        // Mouse movement events
    mut query: Query<(&mut ExternalImpulse, &mut Transform, &mut Player, &mut Velocity, &PlayerRaycast)>,
) {
    // Removed map_boundary - player can move freely
    let current_time = time.elapsed_secs();            // How many seconds since the game started
    let _delta_time = time.delta_secs();                // Time since last frame
    
    // Process the player entity
    for (_impulse, mut transform, mut player, mut velocity, raycast) in query.iter_mut() {
        
        // MOUSE LOOK - Update facing direction based on mouse movement
        for motion in mouse_motion.read() {
            // Update facing angle based on horizontal mouse movement
            player.facing_angle -= motion.delta.x * player.mouse_sensitivity;
        }
        
        // Always update the visual rotation to match the facing angle
        transform.rotation = Quat::from_rotation_y(player.facing_angle);
        
        // JUMPING BEHAVIOR
        //if keyboard_input.pressed(KeyCode::Space) && player.is_grounded && current_time >= player.next_jump_time {
        if keyboard_input.pressed(KeyCode::Space)  {
            let jump_y = 8.0;  // Upward force
            velocity.linvel.y = jump_y;  // Set upward velocity
            player.next_jump_time = current_time + 0.5;  // 0.5 second jump cooldown
            player.is_grounded = false;  // Player is now airborne
            println!("Player jumped!");
        }
        
        //if player.is_grounded {
        if true {
            // Calculate movement directions relative to CURRENT facing angle
            // This ensures movement direction follows the player's sight in real-time
            // In Bevy's coordinate system: +X is right, +Z is forward, +Y is up
            // Forward direction (negative Z is forward in Bevy)

            let forward_dir = transform.forward();
            let right_dir = transform.right();
            let mut movement = Vec3::ZERO;
            
            // FORWARD/BACKWARD MOVEMENT
            if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
                movement += forward_dir * player.move_speed;  // Forward
            }
            if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
                movement -= forward_dir * player.move_speed * 0.5;  // Backward (slower)
            }
            
            // STRAFE LEFT/RIGHT MOVEMENT
            if keyboard_input.pressed(KeyCode::KeyA) {
                println!("Strafe left pressed!");
                movement -= right_dir * player.move_speed;  // Strafe left
            }
            if keyboard_input.pressed(KeyCode::KeyD) {
                println!("Strafe right pressed!");
                movement += right_dir * player.move_speed;  // Strafe right
            }
            
            // Apply movement if any keys are pressed
            if movement != Vec3::ZERO {
                // Check if we're about to hit something close ahead
                let movement_modifier = if raycast.hit_something && raycast.hit_distance < 2.0 {
                    // Slow down significantly when very close to an obstacle
                    0.3
                } else if raycast.hit_something && raycast.hit_distance < 4.0 {
                    // Slow down moderately when approaching an obstacle
                    0.6
                } else {
                    // Normal movement speed
                    1.0
                };
                
                // Apply movement by setting velocity with obstacle avoidance
                velocity.linvel.x = movement.x * movement_modifier;
                velocity.linvel.z = movement.z * movement_modifier;
                
                // Optional: Print warning when close to obstacles
                if raycast.hit_something && raycast.hit_distance < 2.0 {
                    println!("Warning: Obstacle detected at {:.2} units ahead!", raycast.hit_distance);
                }
            } else {
                // Stop horizontal movement when no keys are pressed
                velocity.linvel.x = 0.0;
                velocity.linvel.z = 0.0;
            }
        } else {
            // AIRBORNE BEHAVIOR (when jumping or falling)
            // Allow some air control but reduce horizontal velocity
            velocity.linvel.x *= 0.98;
            velocity.linvel.z *= 0.98;
        }
    }
}

/// Function to handle item pickup when player touches items
pub fn check_player_sensors(
    mut commands: Commands,                    // To despawn picked-up items
    mut collision_events: EventReader<CollisionEvent>, // Physics collision events
    sensor_query: Query<&PlayerSensor>,       // Find all player sensor entities
    mut inventory_query: Query<&mut PlayerInventory>, // Find all player inventory components
    item_query: Query<(Entity, &Item)>,       // Find all item entities
) {
    // Process each collision event that happened this frame
    for collision_event in collision_events.read() {
        // Only care about collisions that just started
        if let CollisionEvent::Started(entity1, entity2, _) = collision_event {
            // Complex pattern matching to find if a player sensor hit an item
            let (parent_entity, item_entity, item) = 
                if let Ok(sensor) = sensor_query.get(*entity1) {
                    // entity1 is a player sensor, check if entity2 is an item
                    if let Ok((item_e, item_c)) = item_query.get(*entity2) {
                        (sensor.parent_entity, item_e, item_c)
                    } else { continue; }
                } else if let Ok(sensor) = sensor_query.get(*entity2) {
                    // entity2 is a player sensor, check if entity1 is an item
                    if let Ok((item_e, item_c)) = item_query.get(*entity1) {
                        (sensor.parent_entity, item_e, item_c)
                    } else { continue; }
                } else { continue; };

            // Try to add the item to the player's inventory
            if let Ok(mut inventory) = inventory_query.get_mut(parent_entity) {
                println!("Player picked up item: {}", item.item_type);
                inventory.items.push(item.item_type.clone());
                println!("Player inventory: {:?}", inventory);
                commands.entity(item_entity).despawn();  // Remove the item from the world
            }
        }
    }
}

/// Function to detect when player touches or leaves the ground
pub fn check_player_ground_sensors(
    mut collision_events: EventReader<CollisionEvent>, // Physics collision events
    mut player_query: Query<&mut Player>,              // Find all player entities
    tile_query: Query<Entity, With<Tile>>,            // Find all terrain tile entities
    landscape_query: Query<Entity, With<crate::landscape::LandscapeElement>>, // Find all landscape elements
) {
    // Process each collision event
    for collision_event in collision_events.read() {
        match collision_event {
            // Collision just started - player might have landed
            CollisionEvent::Started(entity1, entity2, _) => {
                // Check if entity1 is a player and entity2 is ground (tile or landscape element)
                if let Ok(mut player) = player_query.get_mut(*entity1) {
                    if tile_query.get(*entity2).is_ok() || landscape_query.get(*entity2).is_ok() {
                        player.is_grounded = true;
                        println!("Player became grounded!");
                    }
                } else if let Ok(mut player) = player_query.get_mut(*entity2) {
                    // Check the opposite order: entity2 is player, entity1 is ground
                    if tile_query.get(*entity1).is_ok() || landscape_query.get(*entity1).is_ok() {
                        player.is_grounded = true;
                        println!("Player became grounded!");
                    }
                }
            },
            // Collision just ended - player might have become airborne
            CollisionEvent::Stopped(entity1, entity2, _) => {
                if let Ok(mut player) = player_query.get_mut(*entity1) {
                    if tile_query.get(*entity2).is_ok() || landscape_query.get(*entity2).is_ok() {
                        player.is_grounded = false;
                        println!("Player became airborne!");
                    }
                } else if let Ok(mut player) = player_query.get_mut(*entity2) {
                    if tile_query.get(*entity1).is_ok() || landscape_query.get(*entity1).is_ok() {
                        player.is_grounded = false;
                        println!("Player became airborne!");
                    }
                }
            }
        }
    }
}

/// Shared resource to store player's current subpixel position in the planisphere grid
/// This resource is used to track the player's position across different coordinate systems:
/// - Subpixel coordinates (I, J, K) in the planisphere grid
/// - Geographic coordinates (longitude, latitude) on the planet surface  
/// - World coordinates (X, Y, Z) in the 3D game space
#[derive(Resource)]
pub struct PlayerSubpixelPosition {
    /// Current subpixel position as (I, J, K) coordinates in planisphere grid
    pub subpixel: (usize, usize, usize),
    /// Geographic coordinates as (longitude, latitude) in degrees
    pub geo_coords: (f64, f64),
    /// Current world position in 3D game space
    pub world_pos: Vec3,
    /// Previous subpixel position - used to detect when player moves to new tile for beacon updates
    pub previous_subpixel: (usize, usize, usize),
}


impl Default for PlayerSubpixelPosition {
    fn default() -> Self {
        Self {
            subpixel: (1500, 750, 0), // Default to center
            geo_coords: (0.0, 45.0),
            world_pos: Vec3::ZERO,
            previous_subpixel: (1500, 750, 0), // Start with same as current
        }
    }
}







/// System to track player's subpixel coordinates using direct world coordinate search
/// 
/// This system continuously updates the PlayerSubpixelPosition resource by:
/// 1. Getting the player's current world position
/// 2. Converting nearby subpixels to world coordinates using gnomonic projection
/// 3. Finding which subpixel bounds contain the player position
/// 4. Updating the resource with subpixel coordinates, geographic coordinates, and world position
///
/// The system uses the current terrain center from TerrainCenter resource to ensure
/// coordinate calculations remain synchronized after terrain recreation.
/// UNIFIED: Track entity positions using vertical raycast (works for players and agents)
pub fn track_entities_subpixel_position_raycast(
    // Query both players and agents with the shared positioning component
    mut entity_query: Query<(Entity, &Transform, &mut EntitySubpixelPosition), Or<(With<Player>, With<crate::agent::Agent>)>>,
    // Separate queries to check entity types
    player_query: Query<&Player>,
    agent_query: Query<&crate::agent::Agent>,
    // Additional queries for player-specific actions
    mut player_subpixel: ResMut<PlayerSubpixelPosition>,
    _player_transform_query: Query<&mut Transform, (With<Player>, Without<EntitySubpixelPosition>)>,
    // Query to update player's SubpixelPosition component for terrain recreation
    mut player_basic_subpixel_query: Query<&mut crate::terrain::SubpixelPosition, (With<Player>, Without<crate::agent::Agent>)>,
    // Query for beacon updates
    mut beacon_query: Query<&mut Transform, (With<PlayerTileBeacon>, Without<Player>, Without<EntitySubpixelPosition>)>,
    rapier_context: ReadRapierContext,
    terrain_center: Res<TerrainCenter>,
    planisphere: Res<planisphere::Planisphere>,

    triangle_mapping: Res<crate::terrain::TriangleSubpixelMapping>,
    terrain_entities: Query<Entity, With<crate::terrain::Tile>>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs();
    

    for (entity_id, transform, mut entity_position) in entity_query.iter_mut() {

        // Throttle raycasting - only raycast every 0.1 seconds to prevent per-frame execution
        if entity_position.last_raycast_time > 0.0 && 
           current_time - entity_position.last_raycast_time < 0.1 {
            continue;
        }

        // eprintln!("RAYCASTING MOVING ENTITY ID: {:?}", entity_id);
        
        // Check if this entity is a player or agent
        let is_player = player_query.get(entity_id).is_ok();
        let is_agent = agent_query.get(entity_id).is_ok();
        
        if is_player {
            // This is the player entity
            // println!("Found player entity: {:?}", entity_id);
            
            // Player-specific actions when detected in raycast:
            
            // 1. Update global player subpixel position resource
            // This maintains compatibility with existing systems that use PlayerSubpixelPosition
            if entity_position.subpixel != player_subpixel.subpixel {
                player_subpixel.previous_subpixel = player_subpixel.subpixel;
                player_subpixel.subpixel = entity_position.subpixel;
                player_subpixel.geo_coords = entity_position.geo_coords;
                player_subpixel.world_pos = entity_position.world_pos;
                println!("Updated global player subpixel position: ({}, {}, {})", 
                         entity_position.subpixel.0, entity_position.subpixel.1, entity_position.subpixel.2);
                         
                // Also update the basic SubpixelPosition component for terrain recreation repositioning
                if let Ok(mut basic_subpixel) = player_basic_subpixel_query.single_mut() {
                    basic_subpixel.i = entity_position.subpixel.0;
                    basic_subpixel.j = entity_position.subpixel.1;
                    basic_subpixel.k = entity_position.subpixel.2;
                    println!("  -> Also updated basic SubpixelPosition component for terrain recreation");
                }
            }
            
            // 2. Trigger player-specific events or state changes
            // For example, you could:
            // - Play footstep sounds based on terrain type
            // - Update player UI with current location
            // - Trigger environmental effects
            // - Log player movement for analytics
            
            // 3. Enhanced position tracking for the player
            // The player gets more detailed tracking compared to agents
            //println!("Player precise position: World({:.2}, {:.2}, {:.2}), Geo({:.6}, {:.6})", 
            //         entity_position.world_pos.x, entity_position.world_pos.y, entity_position.world_pos.z,
            //         entity_position.geo_coords.0, entity_position.geo_coords.1);

        } else if is_agent {
            // This is an Agent entity
            println!("Found agent entity: {:?}", entity_id);
            
            // Agent-specific actions when detected in raycast:
            // - Update AI navigation targets
            // - Trigger agent state changes
            // - Handle agent-specific positioning logic
            println!("Agent position updated: ({}, {}, {})", 
                     entity_position.subpixel.0, entity_position.subpixel.1, entity_position.subpixel.2);
        }
        let entity_pos = transform.translation;
        
        // Throttle raycasting to avoid performance issues (every 0.1 seconds)
        // Skip throttling on first run (when last_raycast_time is 0.0)
        //if entity_position.last_raycast_time > 0.0 && current_time - entity_position.last_raycast_time < 0.1 {
        //    continue;
        //}
        
        // Create vertical ray from entity position downward
        // FIX: Use entity position but shoot from higher up with longer range


        let ray_direction = Vec3::new(0.0, -1.0, 0.0); // Straight down
        let ray_origin = Vec3::new(transform.translation.x, 100.0,
            transform.translation.z);
       // Get the rapier context
       let Ok(ctx) = rapier_context.single() else { continue; };

       // Perform vertical raycast to ground
        let max_distance = 400.0; // Look down 400m max to ensure we hit terrain
        let solid = true;
        // Use multiple raycasts to get past entity colliders
        let filter = QueryFilter::new().exclude_rigid_body(entity_id);
        // eprintln!("for entity {:?} (ID: {:?})", entity_position, entity_id);
        // eprintln!("RAYCASTING PLAYER ({:.2}, {:.2}, {:.2}) downwards", ray_origin.x, ray_origin.y, ray_origin.z);
        
        // Keep raycasting through non-terrain entities until we hit terrain
        let mut current_ray_origin = ray_origin;
        let mut remaining_distance = max_distance;
        
        for _attempt in 0..5 { // Reduced attempts from 20 to 5 to prevent infinite loops
            if let Some((entity, ray_intersection)) = ctx.cast_ray_and_get_normal(
                current_ray_origin,
                ray_direction,
                remaining_distance,
                solid,
                filter,
            ) {
                // eprintln!("RAYCASTING PLAYER Attempt {}: Hit entity {:?} at distance {:.2}", attempt + 1, entity, ray_intersection.time_of_impact);
                
                // Check if this entity has a Tile component (is terrain)
                if terrain_entities.contains(entity) {
                    // eprintln!("RAYCASTING PLAYER Found terrain entity {:?} after {} attempts", entity, attempt + 1);
                // Extract triangle index from physics feature (same method as green marker)
                let feature_info = format!("{:?}", ray_intersection.feature);                  
                //eprintln!("RAYCASTING PLAYER Feature: {}", feature_info);
                let triangle_index = match &ray_intersection.feature {      

                    _f if feature_info.contains("Face") => {
                        // Extract the numeric ID from the debug string
                        feature_info.chars()
                            .filter(|c| c.is_ascii_digit())
                            .collect::<String>()
                            .parse::<u32>()
                            .unwrap_or(0)
                    },
                    _ => {
                        continue; // Skip non-triangle hits
                    }
                };
                // Apply same offset detection as green marker
                let adjusted_triangle_index = if triangle_index >= triangle_mapping.triangle_to_subpixel.len() as u32 {
                    let mapping_size = triangle_mapping.triangle_to_subpixel.len() as u32;
                    let potential_offset = (triangle_index / mapping_size) * mapping_size;
                    let corrected_index = triangle_index - potential_offset;
                    
                    if corrected_index < mapping_size {
                        corrected_index
                    } else {
                        triangle_index
                    }
                } else {
                    triangle_index
                };
                
                // Check if triangle mapping is stale (from different terrain center)
                let mapping_terrain_center_mismatch = 
                    (triangle_mapping.terrain_center_lon - terrain_center.longitude).abs() > 0.000001 ||
                    (triangle_mapping.terrain_center_lat - terrain_center.latitude).abs() > 0.000001;
                
                if mapping_terrain_center_mismatch {
                    // eprintln!("WARNING: Triangle mapping is stale! Mapping center: ({:.6}, {:.6}), Current center: ({:.6}, {:.6})",
                    //     triangle_mapping.terrain_center_lon, triangle_mapping.terrain_center_lat,
                    //     terrain_center.center_lon, terrain_center.center_lat);
                    // eprintln!("Skipping raycast until triangle mapping is updated");
                    continue;
                }
                
                // Get subpixel from triangle mapping
                if let Some(&(i, j, k)) = triangle_mapping.triangle_to_subpixel.get(adjusted_triangle_index as usize) {
                    //eprintln!("RAYCASTING PLAYER Triangle index: {} -> {}", triangle_index, adjusted_triangle_index);
                    let new_subpixel = (i, j, k);

                    // Update entity position using raycast result
                    if new_subpixel != entity_position.subpixel {
                        let (lon, lat) = planisphere.subpixel_to_geo(i, j, k);
                        
                        entity_position.previous_subpixel = entity_position.subpixel;
                        entity_position.subpixel = new_subpixel;
                        entity_position.geo_coords = (lon, lat);
                        entity_position.world_pos = entity_pos;
                        entity_position.last_raycast_time = current_time;
                        
                        //println!("RAYCAST Entity subpixel: triangle {}", adjusted_triangle_index);
                        
                        // If this is a player, check distance from terrain center and update beacon
                        if is_player {
                            eprintln!("RAYCASTING PLAYER: Entity {:?} hit terrain tile at subpixel ({}, {}, {}) with geo coords ({:.6}, {:.6})",
                                     entity_id, i, j, k, lon, lat);
                            // Calculate Manhattan distance in world coordinates
                            let player_world_x = entity_pos.x;
                            let player_world_z = entity_pos.z;
                            let (terrain_center_world_x, terrain_center_world_y) = crate::planisphere::geo_to_gnomonic_helper(
                                terrain_center.longitude, 
                                terrain_center.latitude, 
                                terrain_center.longitude, 
                                terrain_center.latitude, 
                                &planisphere
                            );
                            
                            //let manhattan_distance_world = (player_world_x - terrain_center_world_x as f32).abs() + 
                            //                             (player_world_z - terrain_center_world_y as f32).abs();
                            let square_distance_to_terrain_center = (terrain_center_world_x - player_world_x as f64).powi(2) + (terrain_center_world_y - player_world_z as f64).powi(2);
                            let distance_to_terrain_center = square_distance_to_terrain_center.sqrt() as f64;
                            //let distance_in_tiles = (manhattan_distance_world / mean_tile_size.max(0.001)) as usize;
                            let distance_in_tiles = (distance_to_terrain_center / planisphere.mean_tile_size.max(0.001) as f64) as usize;
                            // Check if player raycast hit is further than max_subpixel_distance
                            eprintln!("RAYCAST: Player hit tile at distance {} tiles from terrain center ({}, {}, {})", 
                                     distance_in_tiles, i, j, k);
                            if distance_in_tiles > terrain_center.max_subpixel_distance {
                                eprintln!("RAYCAST: Player hit tile at distance {} > threshold {}, updating beacon to ({}, {}, {})", 
                                         distance_in_tiles, terrain_center.max_subpixel_distance, i, j, k);
                                
                                // Update beacon position to the hit tile
                                if let Ok(mut beacon_transform) = beacon_query.single_mut() {
                                    // Get the exact subpixel tile center coordinates
                                    let corners = planisphere.get_subpixel_corners(i, j, k);
                                    let subpixel_center_lon = (corners[0].0 + corners[2].0) / 2.0;
                                    let subpixel_center_lat = (corners[0].1 + corners[2].1) / 2.0;
                                    
                                    // Convert to world coordinates
                                    let (beacon_world_x, beacon_world_y) = crate::planisphere::geo_to_gnomonic_helper(
                                        subpixel_center_lon,
                                        subpixel_center_lat,
                                        terrain_center.longitude,
                                        terrain_center.latitude,
                                        &planisphere,
                                    );
                                    
                                    // Update beacon position
                                    beacon_transform.translation = Vec3::new(beacon_world_x as f32, 0.0, beacon_world_y as f32);
                                    // debug_beacon.subpixel_info = Some((i, j, k)); // Field removed in refactoring
                                    
                                    println!("Beacon updated to tile ({}, {}, {}) at world position ({:.2}, 0.0, {:.2})", 
                                             i, j, k, beacon_world_x, beacon_world_y);
                                }
                            }
                        }
                    }
                }
                
                // Successfully processed terrain, break out of attempt loop
                break;
            } else {
                // eprintln!("RAYCASTING PLAYER Hit non-terrain entity {:?}, continuing through it", entity);
                // Move ray origin just past this entity and continue
                let advance_distance = ray_intersection.time_of_impact + 0.1;
                
                // Safety checks to prevent infinite loops
                if advance_distance <= 0.001 {
                    // eprintln!("RAYCASTING PLAYER Advance distance too small, breaking");
                    break;
                }
                
                current_ray_origin = current_ray_origin + ray_direction * advance_distance;
                remaining_distance -= advance_distance;
                
                if remaining_distance <= 0.1 {
                    // eprintln!("RAYCASTING PLAYER Ran out of distance after {} attempts", attempt + 1);
                    break;
                }
            }
        } else {
            // eprintln!("RAYCASTING PLAYER No hit found after {} attempts", attempt + 1);
            break;
        }
    } // End of attempt loop
        
        // Update timestamp even if no hit (to throttle attempts)
        //entity_position.last_raycast_time = current_time;
    }
}














/// Terrain recreation system - handles asset cleanup and terrain generation
pub fn terrain_recreation_system(
    time: Res<Time>,
    mut terrain_center: ResMut<TerrainCenter>,
    player_subpixel: Res<PlayerSubpixelPosition>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    terrain_query: Query<Entity, With<crate::terrain::Tile>>,
    landscape_query: Query<Entity, With<crate::landscape::LandscapeElement>>,
    object_query: Query<(Entity, &ObjectDefinition)>,
    planisphere: Res<planisphere::Planisphere>,
    mut rendered_subpixels: ResMut<RenderedSubpixels>,
    mut triangle_mapping: ResMut<crate::terrain::TriangleSubpixelMapping>,
    mut asset_tracker: ResMut<crate::TerrainAssetTracker>,
) {
    let current_time = time.elapsed_secs();
    let time_since_last_recreation = current_time - terrain_center.last_recreation_time;
    
    // Calculate distance from terrain center
    let player_geo = player_subpixel.geo_coords;
    let center_geo = (terrain_center.longitude, terrain_center.latitude);
    let player_world_pos = player_subpixel.world_pos;
    let center_world_pos = Vec3::new(0.0, 0.0, 0.0);
    let distance_tiles = (player_world_pos - center_world_pos).length()/planisphere.mean_tile_size as f32;
    let terrain_center_geo = (terrain_center.longitude, terrain_center.latitude);
    let _terrain_center_world_pos = crate::planisphere::geo_to_gnomonic_helper(
        terrain_center.longitude, 
        terrain_center.latitude, 
        terrain_center.longitude, 
        terrain_center.latitude, 
        &planisphere
    );
    let terrain_center_world_pos = Vec3::new(
        terrain_center_geo.0 as f32 * planisphere.mean_tile_size as f32,
        0.0, // Y position is not used in gnomonic projection
        terrain_center_geo.1 as f32 * planisphere.mean_tile_size as f32,
    );
    despawn_unified_object_from_name(&mut commands, "TerrainCenter", object_query);
    spawn_terraincenter_at_world_position(&mut commands, &mut meshes, &mut materials, Some(&planisphere), &terrain_center, terrain_center_world_pos);

    // Simple distance calculation (approximation)
    // TODO: distance in tiles !
    
    let distance_lon = (player_geo.0 - center_geo.0).abs();
    let distance_lat = (player_geo.1 - center_geo.1).abs();
    let max_distance = 0.01; // Approximately 1km in degrees
    
    let needs_recreation = (distance_tiles as usize > terrain_center.max_subpixel_distance) 
        && time_since_last_recreation > 1.0;
    
    if needs_recreation {
        println!("Player distance from terrain center exceeds threshold. Recreating terrain... (last recreation: {:.1}s ago)", time_since_last_recreation);
        
        // Use player's current geographic coordinates as new terrain center
        let (new_center_lon, new_center_lat) = player_subpixel.geo_coords;
        
        // Update terrain center resource
        terrain_center.longitude = new_center_lon;
        terrain_center.latitude = new_center_lat;
        terrain_center.last_recreation_time = current_time;

        // Clear old triangle mapping
        triangle_mapping.triangle_to_subpixel.clear();
        
        // CRITICAL: Clean up old asset handles from Bevy's asset system to prevent memory leaks
        asset_tracker.cleanup_assets(&mut meshes, &mut materials);
        
        // Remove existing terrain and landscape entities
        for terrain_entity in terrain_query.iter() {
            commands.entity(terrain_entity).despawn(); // Use despawn() instead of despawn_recursive()
        }
        for landscape_entity in landscape_query.iter() {
            commands.entity(landscape_entity).despawn();
        }
        
        // Create new terrain
        crate::terrain::create_terrain_gnomonic_rectangular(
            &mut commands,
            &mut meshes,
            &mut materials,
            &asset_server,
            new_center_lon,
            new_center_lat,
            40, // Use terrain radius
            &planisphere,
            Some(&mut rendered_subpixels),
            Some(&mut triangle_mapping),
            Some(&mut asset_tracker)
        );
        println!("Terrain recreation completed successfully");
    }
}

/// Coordinate synchronization system - handles positioning after terrain recreation
pub fn coordinate_sync_system(
    terrain_center: Res<TerrainCenter>,
    mut player_subpixel: ResMut<PlayerSubpixelPosition>,
    mut player_query: Query<&mut Transform, (With<Player>, Without<crate::beacons::PlayerTileBeacon>, Without<crate::beacons::TerrainCenterBeacon>, Without<crate::agent::Agent>)>,
    mut agent_query: Query<(&mut Transform, &mut EntitySubpixelPosition), (With<crate::agent::Agent>, Without<Player>, Without<crate::beacons::PlayerTileBeacon>, Without<crate::beacons::TerrainCenterBeacon>)>,
    planisphere: Res<planisphere::Planisphere>,
) {
    // Only run coordinate sync when terrain center has changed
    if !terrain_center.is_changed() {
        return;
    }
    
    let new_center_lon = terrain_center.longitude;
    let new_center_lat = terrain_center.latitude;

    // Reposition player to (0,0,0) world coordinates relative to new terrain center
    if let Ok(mut player_transform) = player_query.single_mut() {
        player_transform.translation = Vec3::new(0.0, player_transform.translation.y, 0.0);
        println!("Repositioned player to (0, {:.2}, 0) relative to new terrain center", player_transform.translation.y);
    }
    
    // Update player's subpixel coordinate tracking
    let (player_new_i, player_new_j, player_new_k) = planisphere.geo_to_subpixel(new_center_lon, new_center_lat);
    spawn_mousetracker_at_tile(commands, meshes, materials, planisphere, terrain_center, player_new_i, player_new_j, player_new_k);

    player_subpixel.subpixel = (player_new_i, player_new_j, player_new_k);
    player_subpixel.geo_coords = (new_center_lon, new_center_lat);
    player_subpixel.world_pos = Vec3::ZERO;
    player_subpixel.previous_subpixel = (player_new_i, player_new_j, player_new_k);
    
    
    // Reposition agents based on their subpixel coordinates relative to new terrain center
    let mut repositioned_agents = 0;
    for (mut agent_transform, mut agent_position) in agent_query.iter_mut() {
        // Convert agent's subpixel coordinates to world coordinates relative to new terrain center
        let (agent_lon, agent_lat) = planisphere.subpixel_to_geo(
            agent_position.subpixel.0, agent_position.subpixel.1, agent_position.subpixel.2
        );
        
        // Use the geo_to_gnomonic_helper function from the planisphere
        let (agent_world_x, agent_world_z) = crate::planisphere::geo_to_gnomonic_helper(
            agent_lon, agent_lat, new_center_lon, new_center_lat, &planisphere
        );
        
        // Update agent world position (keep Y coordinate for physics)
        agent_transform.translation.x = agent_world_x as f32;
        agent_transform.translation.z = agent_world_z as f32;
        
        // Update agent's coordinate tracking
        agent_position.world_pos = agent_transform.translation;
        agent_position.geo_coords = (agent_lon, agent_lat);
        
        repositioned_agents += 1;
    }
    
    println!("Repositioned {} agents relative to new terrain center", repositioned_agents);
    println!("Coordinate synchronization completed successfully");
}






pub fn player_raycast_system(
    time: Res<Time>,
    mut raycast_query: Query<(&Transform, &mut PlayerRaycast), With<Player>>,
) {
    let current_time = time.elapsed_secs();
    
    for (transform, mut raycast) in raycast_query.iter_mut() {
        if current_time - raycast.last_check_time >= raycast.check_interval {
            let ray_start = transform.translation + Vec3::Y * 0.5;
            let ray_direction = transform.forward().normalize();
            
            // Simplified raycast simulation for now
            // TODO: Use proper RapierContext when Bevy-Rapier API is updated
            // For now, we'll simulate obstacle detection based on player position and direction
            
            // Simulate hitting terrain at low Y positions (ground level detection)
            let _ground_height = 0.0;
            let obstacle_ahead = transform.translation.y < 3.0 && ray_direction.y < 0.0;
            
            // Simulate hitting objects based on position (simple heuristic)
            let forward_point = transform.translation + ray_direction * raycast.range;
            let estimated_terrain_height = 0.0; // In real implementation, sample terrain height
            
            if obstacle_ahead || forward_point.y <= estimated_terrain_height {
                // Simulate hitting something
                let estimated_distance = if obstacle_ahead {
                    2.0 // Close obstacle
                } else {
                    // Calculate distance to ground intersection
                    let height_diff = transform.translation.y - estimated_terrain_height;
                    if ray_direction.y < -0.1 {
                        height_diff / -ray_direction.y
                    } else {
                        raycast.range * 0.8 // Far obstacle
                    }
                };
                
                raycast.hit_something = true;
                raycast.hit_distance = estimated_distance.min(raycast.range);
                raycast.hit_point = ray_start + ray_direction * raycast.hit_distance;
                raycast.hit_normal = Vec3::Y; // Assume ground normal
                
                // Debug output when close to obstacles
                if raycast.hit_distance < 5.0 {
                    println!("Player raycast simulated hit at distance: {:.2}", raycast.hit_distance);
                }
            } else {
                // No obstacle detected
                raycast.hit_something = false;
                raycast.hit_distance = raycast.range;
                raycast.hit_point = Vec3::ZERO;
                raycast.hit_normal = Vec3::ZERO;
            }
            
            raycast.last_check_time = current_time;
        }
    }
}