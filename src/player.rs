

// use bevy::math::ops::sqrt;
// Import statements - bring in code from other modules and crates
use bevy::prelude::*;           // Bevy game engine core functionality
use bevy::window::PrimaryWindow;
use bevy_rapier3d::prelude::*;  // Physics engine for 3D collision detection
use bevy_rapier3d::plugin::context::systemparams::ReadRapierContext;
use bevy::input::mouse::{MouseMotion, MouseButton}; 

// Mouse movement events
use crate::terrain::{RenderedSubpixels, Tile, TerrainCenter, entities_in_rendered_subpixels}; // Import Tile component and resources from terrain module
use crate::landscape::Item; // Import Item from landscape module
// use crate::TerrainConfig;
use crate::planisphere::{self}; // Import planisphere for coordinate conversion
use crate::game_object::{ObjectTemplate, CollisionBehavior, 
                        spawn_template_scene, ObjectDefinition, ObjectTemplates, MouseTrackerObject}; // Import game object definitions
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
}

#[derive(Bundle)]
pub struct PlayerBundle {
    pub player: Player,
    pub player_inventory: PlayerInventory,
    pub entity_position: EntitySubpixelPosition, // NEW: Shared positioning component
}

impl Default for PlayerBundle {
    fn default() -> Self {
        Self {
            player: Player {
                next_jump_time: 0.0,
                is_grounded: false,
                facing_angle: 0.0,
                mouse_sensitivity: 0.002,
                move_speed: 15.0,


            },
            player_inventory: PlayerInventory::default(),
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

pub fn detect_mouse_clicks(
    mut commands: Commands,
    materials: ResMut<Assets<StandardMaterial>>,
    object_templates: Res<ObjectTemplates>,
    mousetracker_query: Query<(Entity, &Transform, &EntitySubpixelPosition),
        With<MouseTrackerObject>>,
    player_query: Query<(Entity, &Transform, &EntitySubpixelPosition), With<Player>>,
    planisphere: Res<planisphere::Planisphere>,
    terrain_center: Res<TerrainCenter>,
    // Add mouse button input resource to detect clicks
    mouse_button_input: Res<ButtonInput<MouseButton>>,
) {
    // Check for left mouse button press
    if mouse_button_input.just_pressed(MouseButton::Left) {
        println!("Left mouse button was clicked!");
        drop_stone(
            commands, 
            materials, 
            &object_templates.rock, // Use rock template for stone
            mousetracker_query, 
            player_query,
            planisphere, 
            terrain_center
        );
        // Your left click action code here
    }
    
    // Check for right mouse button press
    if mouse_button_input.just_pressed(MouseButton::Right) {
        println!("Right mouse button was clicked!");
        // Your right click action code here
    }
    
    // You can also check for:
    // - mouse_button_input.just_released(MouseButton::Left)
    // - mouse_button_input.pressed(MouseButton::Left) - true as long as the button is held down
}






pub fn cast_ray_from_camera(
    //commands: &mut Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    rapier_context: ReadRapierContext,
    mut mouse_tracker_query: Query<(Entity, &mut Transform), With<MouseTrackerObject>>,
){
    let Ok(window) = windows.single() else { return ; };
    let Ok((camera, camera_transform)) = cameras.single() else { return ; };
    let mut hit_point = Vec3::ZERO; // Default hit point if no intersection occurs
    if let Some(cursor_position) = window.cursor_position() {
        // Create a ray from the camera to the cursor position
        if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) {
            // Get the rapier context
            let Ok(ctx) = rapier_context.single() else { return ; };
            
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
                hit_point = ray.origin + *ray.direction * ray_intersection.time_of_impact;
            }
        }
    }
    for (marker_entity, mut transform) in mouse_tracker_query.iter_mut() {
        // Reset the mouse tracker position to the raycast hit point
        transform.translation = hit_point;
    }
}

pub fn drop_stone(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
        template: &ObjectTemplate,
        mousetracker_query: Query<(Entity, &Transform, &EntitySubpixelPosition), With<MouseTrackerObject>>,
        player_query: Query<(Entity, &Transform, &EntitySubpixelPosition), With<Player>>,
        planisphere: Res<planisphere::Planisphere>,
        terrain_center: Res<TerrainCenter>,
    )
    {   for (player_entity, player_transform, player_ijkpos) in player_query.iter() {
            for (mousetracker_entity, mousetracker_transform, mousetracker_ijkpos) in mousetracker_query.iter() {
                // Get the subpixel coordinates from the mouse tracker
                let mousetracker_subpixel = mousetracker_ijkpos.subpixel;
                let player_subpixel = player_ijkpos.subpixel;
                // Calculate the world position of the subpixel center
                let mousetracker_world_pos = calculate_subpixel_center_world_position(
                    mousetracker_subpixel.0 as i32, 
                    mousetracker_subpixel.1 as i32, 
                    mousetracker_subpixel.2 as i32, 
                    &planisphere, 
                    &terrain_center
                );
                let player_world_pos = calculate_subpixel_center_world_position(
                    player_subpixel.0 as i32, 
                    player_subpixel.1 as i32, 
                    player_subpixel.2 as i32, 
                    &planisphere, 
                    &terrain_center
                );
                let player_to_target = Vec3::new(
                    mousetracker_world_pos.x - player_world_pos.x,
                    0.0, // Keep Y at 0 for ground level
                    mousetracker_world_pos.z - player_world_pos.z,
                );
                let distance = player_to_target.length();
                let force = 13.0;
                let dmax = 10.0; // Maximum distance for the stone to be thrown
                let velocity = Velocity {
                    linvel: player_to_target.normalize() * 0.67 * force + 0.33  * force * Vec3::Y, // Adjust speed as needed
                    angvel: Vec3::ZERO,
                };
                let physics_bundle = (
                    RigidBody::Dynamic,
                    crate::game_object::create_collider_from_shape(&crate::game_object::ObjectShape::Cube { size: Vec3::ONE }),
                    velocity,
                    ExternalImpulse::default(),
                    GravityScale(1.0),
                    Damping { linear_damping: 0.0, angular_damping: 0.1 },
                    //None, //LockedAxes::default() | LockedAxes::default(),
                    //LockedAxes::default()
                    ActiveEvents::COLLISION_EVENTS,
                    ActiveCollisionTypes::all(),
                    );
                // Spawn a stone at the mouse tracker position
                spawn_template_scene(
                    &mut commands,
                    &mut materials,
                    &planisphere,
                    &terrain_center,
                    template,
                    player_transform.translation + player_to_target * 0.5, // Position it halfway between player and mouse tracker
                    player_transform.translation.y + template.y_offset, // Use player's Y position + offset
                    CollisionBehavior::Dynamic, // Set collision behavior to dynamic for dropped items
                    (physics_bundle, 
                        //crate::game_object::RaycastTileLocator{last_tile: None}, 
                        //crate::game_object::EntityInfoOverlay::default(),
                        //EntitySubpixelPosition::default(),
                    )
                );
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





/// Function to handle player movement with keyboard and mouse input
pub fn move_player(
    time: Res<Time>,                                    // Bevy's time resource
    keyboard_input: Res<ButtonInput<KeyCode>>,         // Keyboard input state
    mut mouse_motion: EventReader<MouseMotion>,        // Mouse movement events
    mut query: Query<(&mut ExternalImpulse, &mut Transform, &mut Player, &mut Velocity)>,
) {
    // Removed map_boundary - player can move freely
    let current_time = time.elapsed_secs();            // How many seconds since the game started
    let _delta_time = time.delta_secs();                // Time since last frame
    
    // Process the player entity
    for (_impulse, mut transform, mut player, mut velocity) in query.iter_mut() {
        
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
            //println!("Player jumped!");
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
                //println!("Strafe left pressed!");
                movement -= right_dir * player.move_speed;  // Strafe left
            }
            if keyboard_input.pressed(KeyCode::KeyD) {
                //println!("Strafe right pressed!");
                movement += right_dir * player.move_speed;  // Strafe right
            }
            velocity.linvel.x = movement.x;
            velocity.linvel.z = movement.z;
           
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
                        //println!("Player became grounded!");
                    }
                } else if let Ok(mut player) = player_query.get_mut(*entity2) {
                    // Check the opposite order: entity2 is player, entity1 is ground
                    if tile_query.get(*entity1).is_ok() || landscape_query.get(*entity1).is_ok() {
                        player.is_grounded = true;
                        //println!("Player became grounded!");
                    }
                }
            },
            // Collision just ended - player might have become airborne
            CollisionEvent::Stopped(entity1, entity2, _) => {
                if let Ok(mut player) = player_query.get_mut(*entity1) {
                    if tile_query.get(*entity2).is_ok() || landscape_query.get(*entity2).is_ok() {
                        player.is_grounded = false;
                        //println!("Player became airborne!");
                    }
                } else if let Ok(mut player) = player_query.get_mut(*entity2) {
                    if tile_query.get(*entity1).is_ok() || landscape_query.get(*entity1).is_ok() {
                        player.is_grounded = false;
                        //println!("Player became airborne!");
                    }
                }
            }
        }
    }
}













pub fn entity_replacement_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rendered_subpixels: ResMut<RenderedSubpixels>,
    object_query : Query<(Entity, &mut Transform,  &ObjectDefinition), (Without<Player>, Without<MouseTrackerObject>)>,

    terrain_center: ResMut<TerrainCenter>,
    planisphere: Res<planisphere::Planisphere>,
    object_templates: Res<ObjectTemplates>,
) {
        //despawn_unified_objects_from_name(&mut commands, "LandCubes", object_query);
        entities_in_rendered_subpixels(&mut commands, &mut meshes, &mut materials, rendered_subpixels, planisphere, terrain_center, object_templates, object_query);
}









/// Terrain recreation system - handles asset cleanup and terrain generation
pub fn terrain_recreation_system(
    time: Res<Time>,
    mut terrain_center: ResMut<TerrainCenter>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut player_query: Query<(Entity, &mut Transform, &EntitySubpixelPosition , &Player)>,
    terrain_query: Query<Entity, With<crate::terrain::Tile>>,
    landscape_query: Query<Entity, With<crate::landscape::LandscapeElement>>,
    mut object_query: Query<(Entity, &mut Transform, &ObjectDefinition),(Without<Player>, Without<MouseTrackerObject>)>,
    planisphere: Res<planisphere::Planisphere>,
    mut rendered_subpixels: ResMut<RenderedSubpixels>,
    mut triangle_mapping: ResMut<crate::terrain::TriangleSubpixelMapping>,
    mut asset_tracker: ResMut<crate::TerrainAssetTracker>,
    object_templates: Res<ObjectTemplates>,
) {
    let current_time = time.elapsed_secs();
    let time_since_last_recreation = current_time - terrain_center.last_recreation_time;
    let mut needs_recreation = false; // Default to false, will be set if conditions are met
    let mut next_terrain_center_tile = (0,0,0); // Default tile for terrain center
    // Calculate distance from terrain center
    for (player_entity, player_transform, player_subpixel_position, _player) in player_query.iter_mut() {
        let player_world_pos = player_transform.translation;
        let center_world_pos = Vec3::new(0.0,  player_transform.translation.y, 0.0);// eprintln!("Player entity: {:?}, Position: ({:.2}, {:.2}, {:.2})", player_entity, player_transform.translation.x, player_transform.translation.y, player_transform.translation.z);
        let distance_tiles = (player_world_pos - center_world_pos).length()/planisphere.mean_tile_size as f32;
        if distance_tiles > terrain_center.max_subpixel_distance as f32 {
            eprintln!("Player is too far from terrain center! Distance: {:.2} tiles, max allowed: {}", distance_tiles, terrain_center.max_subpixel_distance);
            needs_recreation = true; // Set flag to recreate terrain
            next_terrain_center_tile = player_subpixel_position.subpixel; // Use player's subpixel as new center
    }
}


    //despawn_unified_object_from_name(&mut commands, "TerrainCenter", object_query);
    //spawn_terraincenter_at_world_position(&mut commands, &mut meshes, &mut materials, Some(&planisphere), &terrain_center, Vec3::new(0.0, 0.0, 0.0));


    if needs_recreation {

        println!("Player distance from terrain center exceeds threshold. Recreating terrain... (last recreation: {:.1}s ago)", time_since_last_recreation);
        

        // Store player positions and calculate the offset needed to move them to origin
        let player_offset = if let Some((_, player_transform, _, _)) = player_query.iter().next() {
            // The offset needed to move the player to origin
            Vec3::new(
                -player_transform.translation.x,
                0.0,  // We don't change Y position (height)
                -player_transform.translation.z
            )
        } else {
            Vec3::ZERO
        };





        // Use player's current geographic coordinates as new terrain center
        let (new_center_lon, new_center_lat) = planisphere.subpixel_to_geo(next_terrain_center_tile.0, next_terrain_center_tile.1, next_terrain_center_tile.2 );
        
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
            40, // Use terrain radius
            &planisphere,
            &terrain_center,
            Some(&mut rendered_subpixels),
            Some(&mut triangle_mapping),
            Some(&mut asset_tracker)
        );

        // Move player to origin (0, 0, 0) while keeping their Y position
        for (_, mut player_transform, _, _) in player_query.iter_mut() {
            player_transform.translation.x += player_offset.x;
            player_transform.translation.z += player_offset.z;
            // Y position remains unchanged
        }

        // Move all other objects by the same offset to maintain relative positions to player
        for (_, mut transform, _) in object_query.iter_mut() {
            transform.translation.x += player_offset.x;
            transform.translation.z += player_offset.z;
            // Y position remains unchanged
        }
        println!("Terrain recreation completed successfully");
        // we need to print the 5 first elements of the triangle mapping and the first elements of the rendered subpixels
        println!("Triangle mapping size: {}, first 5 entries: {:?}", 
            triangle_mapping.triangle_to_subpixel.len(), 
            &triangle_mapping.triangle_to_subpixel[..5]);
        println!("Rendered subpixels size: {}", //
            rendered_subpixels.subpixels.len());
        println!("First 5 rendered subpixels:");
        for tuple in rendered_subpixels.subpixels.iter().take(5) {
            println!("Subpixel: ({}, {}, {})",
                tuple.0, tuple.1, tuple.2
            );
        }
        entity_replacement_system(commands, meshes, materials, rendered_subpixels, object_query, terrain_center, planisphere, object_templates);
    }
}






