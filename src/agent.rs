// Import statements - these bring in code from other modules and crates
use bevy::prelude::*;           // Bevy game engine core functionality
use bevy_rapier3d::prelude::*;  // Physics engine for 3D collision detection
use rand::Rng;                  // Random number generation for agent behavior
use crate::terrain::Tile; // Import Tile and SubpixelPosition components from terrain module
use crate::landscape::Item; // Import Item component from landscape module
use crate::player::EntitySubpixelPosition; // Import shared positioning component
use crate::planisphere; // Import planisphere for coordinate conversion

/// Agent action types - what the agent can choose to do
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AgentAction {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    RotateLeft,
    Stop,
}

/// Agent Component - This struct defines what data each agent entity holds
/// In Rust, #[derive(Component)] tells Bevy this can be attached to entities
/// pub means other modules can see and use this struct
#[derive(Component)]
pub struct Agent {
    pub next_move_time: f32,      // Timer: when can the agent choose a new movement?
    pub current_action: AgentAction, // Current action the agent is performing
    pub is_grounded: bool,        // Boolean: is the agent touching the ground?
    pub facing_angle: f32,        // Float: current facing direction in radians (Y-axis rotation)
}

/// GroundSensor Component - Detects when agent touches ground
/// This is attached to a sensor entity that's a child of the agent
#[derive(Component)]
pub struct GroundSensor {
    pub _parent_entity: Entity,    // Entity is Bevy's ID system - this points to the parent agent (prefixed with _ to indicate intentionally unused)
}

/// Sensor Component - Detects items to pick up
/// This creates an invisible sphere around the agent to detect nearby items
#[derive(Component)]
pub struct Sensor {
    pub parent_entity: Entity,    // Reference to the agent that owns this sensor
}

/// Inventory Component - Stores items the agent has collected
/// #[derive(Component, Default, Debug)] generates useful code automatically:
/// - Component: makes it attachable to entities
/// - Default: creates empty inventory with Default::default()
/// - Debug: allows printing with {:?} for debugging
#[derive(Component, Default, Debug)]
pub struct Inventory {
    pub items: Vec<String>,       // Vec<String> is a dynamic array of text strings
}

/// Raycast Component - Handles raycasting for agents
/// This component allows agents to detect terrain and objects in their path
#[derive(Component)]
pub struct AgentRaycast {
    pub range: f32,               // Maximum distance to cast the ray
    pub hit_something: bool,      // Whether the last raycast hit something
    pub hit_distance: f32,        // Distance to the hit point
    pub hit_point: Vec3,          // World coordinates of the hit point
    pub hit_normal: Vec3,         // Normal vector at the hit point
    pub last_check_time: f32,     // Time of last raycast check
    pub check_interval: f32,      // How often to perform raycasts (in seconds)
}

/// Function to create multiple agent entities in the game world
/// In Rust, 'mut' means the parameter can be modified
/// ResMut<T> is Bevy's way of getting mutable access to a resource
pub fn create_agents(
    commands: &mut Commands,                              // Commands let us spawn/despawn entities
    meshes: &mut ResMut<Assets<Mesh>>,                   // Collection of 3D meshes (shapes)
    materials: &mut ResMut<Assets<StandardMaterial>>,    // Collection of materials (colors/textures)
    agent_count: usize,                                  // usize is Rust's type for array indices/counts
    planisphere: &planisphere::Planisphere,              // Planisphere for coordinate conversion
    terrain_center_lon: f64,                             // Current terrain center longitude
    terrain_center_lat: f64,                             // Current terrain center latitude
) {
    // Create the 3D shape for our agents - a capsule (like a pill shape)
    let agent_mesh = meshes.add(Capsule3d::new(0.3, 0.8)); // radius 0.3, height 0.8
    // Create the material (color) for our agents - red color
    let agent_material = materials.add(Color::srgb(0.8, 0.1, 0.1)); // RGB: high red, low green/blue

    // Loop to create multiple agents
    // 'for i in 0..agent_count' means: for each number from 0 to agent_count-1
    for i in 0..agent_count {
        // Calculate position for each agent in a grid pattern
        // % is modulo operator (remainder after division)
        // / is integer division (no decimal part)
        let x_offset = (i % 3) as f32 * 4.0 - 4.0; // Spread across X: -4, 0, 4
        let z_offset = (i / 3) as f32 * 4.0 - 4.0; // Spread across Z: -4, 0, 4
        
        // Skip creating agent at player spawn position (0, 0)
        if x_offset == 0.0 && z_offset == 0.0 {
            continue; // Skip this iteration and go to next agent
        }
        
        // Convert world position to geographic coordinates then to subpixel coordinates
        // Use the inverse gnomonic projection to get geographic coordinates
        let agent_lon = terrain_center_lon; // Start with terrain center
        let agent_lat = terrain_center_lat; // Start with terrain center
        
        // For now, use approximate geographic offset based on world coordinates
        // This is simplified - a more accurate conversion would use inverse gnomonic projection
        let approx_lon = agent_lon + (x_offset as f64 * 0.001); // Rough conversion
        let approx_lat = agent_lat + (z_offset as f64 * 0.001); // Rough conversion
        
        // Convert to subpixel coordinates
        let (agent_i, agent_j, agent_k) = planisphere.geo_to_subpixel(approx_lon, approx_lat);
        
        // Spawn the main agent entity - split into multiple parts to avoid tuple size limit
        let agent_id = commands.spawn((
            // Visual components - what the player sees
            Mesh3d(agent_mesh.clone()),           // The 3D shape
            MeshMaterial3d(agent_material.clone()), // The color/material
            Transform::from_xyz(x_offset, 5.0, z_offset), // Position in 3D space (x, y, z)
            
            // Physics components - how the agent interacts with the world
            RigidBody::Dynamic,                   // Can move and be affected by forces
            Collider::capsule_y(0.4, 0.3),      // Physics shape for collision detection
            Velocity { linvel: Vec3::new(0.0, -0.1, 0.0), angvel: Vec3::ZERO }, // Initial velocity
            ExternalImpulse::default(),          // Allows applying forces to move the agent
            GravityScale(1.0),                   // How much gravity affects this agent (1.0 = normal)
            Damping { linear_damping: 0.0, angular_damping: 0.1 }, // Resistance to movement
            
            // Lock rotation on X and Z axes, allow Y rotation for turning
            // | is bitwise OR - combines multiple flags
            LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
            
            // Enable collision detection events
            ActiveEvents::COLLISION_EVENTS,      // Tells physics engine to send collision notifications
            ActiveCollisionTypes::all(),         // Detect all types of collisions
        )).id(); // .id() gets the unique ID of the spawned entity
        
        // Add the remaining components separately to avoid tuple size limit
        commands.entity(agent_id).insert((
            // Our custom Agent component with initial values
            Agent {
                next_move_time: 0.0,             // Can make decision immediately
                current_action: AgentAction::Stop, // Start by not moving
                is_grounded: false,              // Start in the air (will fall and land)
                facing_angle: 0.0,               // Start facing forward (negative Z direction)
            },
            Inventory::default(),                // Start with empty inventory
            
            EntitySubpixelPosition {             // NEW: Shared positioning component
                subpixel: (agent_i, agent_j, agent_k),
                geo_coords: (approx_lon, approx_lat),
                world_pos: Vec3::new(x_offset, 5.0, z_offset),
                previous_subpixel: (agent_i, agent_j, agent_k),
                last_raycast_time: 0.0,
            },
            AgentRaycast {
                range: 5.0,                      // Can detect objects 5 units ahead
                hit_something: false,            // No hit initially
                hit_distance: 0.0,               // No distance initially
                hit_point: Vec3::ZERO,           // No hit point initially
                hit_normal: Vec3::ZERO,          // No normal initially
                last_check_time: 0.0,            // No previous check
                check_interval: 0.1,             // Check every 0.1 seconds
            },
        ));

        // Add child entities to the agent (sensors and visual markers)
        // with_children creates entities that move with the parent
        commands.entity(agent_id).with_children(|parent| {
            // Item pickup sensor - invisible sphere that detects nearby items
            parent.spawn((
                Collider::ball(2.0),                        // Invisible sphere with radius 2.0
                Sensor { parent_entity: agent_id },         // Links this sensor to its parent agent
                bevy_rapier3d::geometry::Sensor,            // Makes it non-solid (things pass through)
                ActiveEvents::COLLISION_EVENTS,             // Detects when items enter/leave
                ActiveCollisionTypes::all(),                // Detect all collision types
                Transform::from_xyz(0.0, 0.0, 0.0),        // Centered on agent (relative position)
            ));
            
            // Visual marker to show which way the agent is facing
            let marker_mesh = meshes.add(Sphere::new(0.1));                    // Small sphere
            let marker_material = materials.add(Color::srgb(1.0, 1.0, 0.0));   // Yellow color
            parent.spawn((
                Mesh3d(marker_mesh),                        // 3D shape
                MeshMaterial3d(marker_material),            // Yellow material
                Transform::from_xyz(0.0, 0.2, -0.4),       // In front of the agent
            ));
        });
    } // End of for loop - all agents are now created
}

/// Function that runs every frame to move agents around
/// Query<T> is Bevy's way to find all entities that have certain components
pub fn move_agents(
    time: Res<Time>,
    mut query: Query<(&mut ExternalImpulse, &mut Transform, &mut Agent, &mut Velocity, &AgentRaycast)>,
) {
    // Removed map_boundary - agents can move freely
    let current_time = time.elapsed_secs(); // How many seconds since the game started
    let mut rng = rand::thread_rng();     // Random number generator for unpredictable behavior
    
    // Process each agent that matches our query
    for (_impulse, mut transform, mut agent, mut velocity, raycast) in query.iter_mut() {
        
        // Safety check: prevent agents from falling through the world
        if transform.translation.y < -50.0 {
            println!("CRITICAL: Agent fell through world! Repositioning to Y=10.0");
            transform.translation.y = 10.0;
            velocity.linvel = Vec3::new(0.0, -2.0, 0.0); // Gentle fall
            agent.is_grounded = false;
        }
        
        // Debug output - print agent status every so often
        // if (current_time * 0.5) as i32 % 5 == 0 && (current_time * 10.0) as i32 % 10 == 0 {
        //     println!("Agent Y: {:.2}, VelY: {:.2}, Grounded: {}", 
        //         transform.translation.y, velocity.linvel.y, agent.is_grounded);
        // }
        
        // DECISION MAKING - Choose one action when it's time
        if current_time >= agent.next_move_time && agent.is_grounded {
            // Check if there's an obstacle ahead
            let obstacle_ahead = raycast.hit_something && raycast.hit_distance < 3.0;
            
            let movement_choice = if obstacle_ahead {
                // If obstacle detected, avoid moving forward and prefer turning or moving sideways
                rng.gen_range(0..10)
            } else {
                // Normal movement pattern
                rng.gen_range(0..12)
            };
            
            agent.current_action = match movement_choice {
                0 | 1 | 2 => {
                    if obstacle_ahead {
                        // Turn left instead of moving forward
                        AgentAction::RotateLeft
                    } else {
                        AgentAction::MoveForward
                    }
                },
                3 | 4 | 5 => AgentAction::MoveBackward,     // Backward movement (25%)  
                6 | 7 | 8 => AgentAction::MoveLeft,         // Left movement (25%)
                9 | 10 | 11 => AgentAction::MoveRight,      // Right movement (25%)
                _ => AgentAction::Stop,                     // Stop (fallback)
            };
            
            // If very close to obstacle, turn more frequently
            let time_multiplier = if obstacle_ahead { 0.5 } else { 1.0 };
            agent.next_move_time = current_time + rng.gen_range(1.0..3.0) * time_multiplier;
            
            // Optional: Print debug info when avoiding obstacles
            // if obstacle_ahead {
            //     println!("Agent avoiding obstacle at {:.2} units ahead", raycast.hit_distance);
            // }
        }

        // EXECUTE THE CHOSEN ACTION
        if agent.is_grounded {
            match agent.current_action {
                AgentAction::MoveForward => {
                    // Additional safety check - don't move forward if very close to obstacle
                    if raycast.hit_something && raycast.hit_distance < 1.5 {
                        // Force stop instead of moving forward
                        velocity.linvel.x = 0.0;
                        velocity.linvel.z = 0.0;
                        agent.current_action = AgentAction::Stop;
                    } else {
                        let forward_dir = transform.forward();
                        let movement_speed = 3.0; // Full speed forward
                        velocity.linvel.x = forward_dir.x * movement_speed;
                        velocity.linvel.z = forward_dir.z * movement_speed;
                    }
                }
                AgentAction::MoveBackward => {
                    let forward_dir = transform.forward();
                    let movement_speed = 1.5; // Half speed backward
                    velocity.linvel.x = -forward_dir.x * movement_speed;
                    velocity.linvel.z = -forward_dir.z * movement_speed;
                }
                AgentAction::MoveLeft => {
                    let right_dir = transform.right();
                    let movement_speed = 1.5; // Half speed left (strafe)
                    velocity.linvel.x = -right_dir.x * movement_speed;
                    velocity.linvel.z = -right_dir.z * movement_speed;
                }
                AgentAction::MoveRight => {
                    let right_dir = transform.right();
                    let movement_speed = 1.5; // Half speed right (strafe)
                    velocity.linvel.x = right_dir.x * movement_speed;
                    velocity.linvel.z = right_dir.z * movement_speed;
                }
                AgentAction::RotateLeft => {
                    // Rotate left by 15 degrees (π/12 radians)
                    agent.facing_angle += std::f32::consts::PI / 12.0;
                    transform.rotation = Quat::from_rotation_y(agent.facing_angle);
                    agent.current_action = AgentAction::Stop; // Reset action after rotating
                }
                AgentAction::Stop => {
                    // Stop horizontal movement
                    velocity.linvel.x = 0.0;
                    velocity.linvel.z = 0.0;
                }
            }
        } else {
            // AIRBORNE BEHAVIOR (when jumping or falling)
            // Gradually reduce horizontal velocity to simulate air resistance
            velocity.linvel.x *= 0.98;
            velocity.linvel.z *= 0.98;
        }
    }
}


/// Function to handle item pickup when agents touch items
/// EventReader<T> lets us process events that happened this frame
pub fn check_sensors(
    mut commands: Commands,                    // To despawn picked-up items
    mut collision_events: EventReader<CollisionEvent>, // Physics collision events
    sensor_query: Query<&Sensor>,            // Find all sensor entities
    mut inventory_query: Query<&mut Inventory>, // Find all inventory components
    item_query: Query<(Entity, &Item)>,      // Find all item entities
) {
    // Process each collision event that happened this frame
    for collision_event in collision_events.read() {
        // Only care about collisions that just started
        if let CollisionEvent::Started(entity1, entity2, _) = collision_event {
            // Complex pattern matching to find if a sensor hit an item
            // This checks both possible orders: sensor-item or item-sensor
            let (parent_entity, item_entity, item) = 
                if let Ok(sensor) = sensor_query.get(*entity1) {
                    // entity1 is a sensor, check if entity2 is an item
                    if let Ok((item_e, item_c)) = item_query.get(*entity2) {
                        (sensor.parent_entity, item_e, item_c) // Found sensor-item collision
                    } else { continue; } // entity2 is not an item, skip this collision
                } else if let Ok(sensor) = sensor_query.get(*entity2) {
                    // entity2 is a sensor, check if entity1 is an item
                    if let Ok((item_e, item_c)) = item_query.get(*entity1) {
                        (sensor.parent_entity, item_e, item_c) // Found item-sensor collision
                    } else { continue; } // entity1 is not an item, skip this collision
                } else { continue; }; // Neither entity is a sensor, skip this collision

            // Try to add the item to the agent's inventory
            if let Ok(mut inventory) = inventory_query.get_mut(parent_entity) {
                println!("Entity picked up item: {}", item.item_type);
                inventory.items.push(item.item_type.clone()); // clone() creates a copy of the string
                println!("Inventory: {:?}", inventory);
                commands.entity(item_entity).despawn();  // Remove the item from the world
            }
        }
    }
}

/// Function to detect when agents touch or leave the ground
/// This uses collision events between agents and terrain tiles or landscape elements
pub fn check_ground_sensors(
    mut collision_events: EventReader<CollisionEvent>, // Physics collision events
    mut agent_query: Query<&mut Agent>,                // Find all agent entities
    tile_query: Query<Entity, With<Tile>>,            // Find all terrain tile entities
    landscape_query: Query<Entity, With<crate::landscape::LandscapeElement>>, // Find all landscape elements
) {
    // Process each collision event
    for collision_event in collision_events.read() {
        match collision_event { // match handles different types of collision events
            // Collision just started - agent might have landed
            CollisionEvent::Started(entity1, entity2, _) => {
                // Check if entity1 is an agent and entity2 is ground (tile or landscape element)
                if let Ok(mut agent) = agent_query.get_mut(*entity1) {
                    if tile_query.get(*entity2).is_ok() || landscape_query.get(*entity2).is_ok() {
                        agent.is_grounded = true;
                        // println!("Agent became grounded via collision!");
                    }
                } else if let Ok(mut agent) = agent_query.get_mut(*entity2) {
                    // Check the opposite order: entity2 is agent, entity1 is ground
                    if tile_query.get(*entity1).is_ok() || landscape_query.get(*entity1).is_ok() {
                        agent.is_grounded = true;
                        // println!("Agent became grounded via collision!");
                    }
                }
            },
            // Collision just ended - agent might have become airborne
            CollisionEvent::Stopped(entity1, entity2, _) => {
                // Same logic as above, but set is_grounded to false
                if let Ok(mut agent) = agent_query.get_mut(*entity1) {
                    if tile_query.get(*entity2).is_ok() || landscape_query.get(*entity2).is_ok() {
                        agent.is_grounded = false;
                        // println!("Agent became airborne via collision!");
                    }
                } else if let Ok(mut agent) = agent_query.get_mut(*entity2) {
                    if tile_query.get(*entity1).is_ok() || landscape_query.get(*entity1).is_ok() {
                        agent.is_grounded = false;
                        // println!("Agent became airborne via collision!");
                    }
                }
            }
        }
    }
}

/// System to perform raycasting for agents
/// This system casts rays forward from each agent to detect terrain and objects
pub fn agent_raycast_system(
    time: Res<Time>,
    mut raycast_query: Query<(&Transform, &mut AgentRaycast), With<Agent>>,
) {
    let current_time = time.elapsed_secs();
    
    for (transform, mut raycast) in raycast_query.iter_mut() {
        // Only perform raycast if enough time has passed
        if current_time - raycast.last_check_time < raycast.check_interval {
            continue;
        }
        
        // Simplified raycast simulation for now
        // In a real implementation, you would use bevy_rapier3d's RapierContext or SpatialQuery
        // For now, we'll simulate obstacle detection based on agent position
        
        // Simulate hitting terrain at low Y positions
        let obstacle_ahead = transform.translation.y < 2.0;
        
        if obstacle_ahead {
            // Simulate hitting something close
            raycast.hit_something = true;
            raycast.hit_distance = 2.0;
            raycast.hit_point = transform.translation + *transform.forward() * 2.0;
            raycast.hit_normal = Vec3::Y;
            
            // Optional: Print debug info
            // if raycast.hit_distance < 2.0 {
            //     println!("Agent raycast hit at distance: {:.2}", raycast.hit_distance);
            // }
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

/// System to render debug rays for agents (optional visual aid)
/// This system creates visual line representations of the raycasts
pub fn debug_agent_raycast_system(
    mut gizmos: Gizmos,
    raycast_query: Query<(&Transform, &AgentRaycast), With<Agent>>,
) {
    for (transform, raycast) in raycast_query.iter() {
        let ray_origin = transform.translation + Vec3::Y * 0.5;
        let ray_direction = transform.forward();
        
        // Draw the ray
        let ray_end = if raycast.hit_something {
            ray_origin + *ray_direction * raycast.hit_distance
        } else {
            ray_origin + *ray_direction * raycast.range
        };
        
        // Color the ray based on hit status
        let ray_color = if raycast.hit_something {
            if raycast.hit_distance < 2.0 {
                Color::srgb(1.0, 0.0, 0.0) // Red for close hits
            } else {
                Color::srgb(1.0, 1.0, 0.0) // Yellow for hits
            }
        } else {
            Color::srgb(0.0, 1.0, 0.0) // Green for no hits
        };
        
        gizmos.line(ray_origin, ray_end, ray_color);
        
        // Draw a small sphere at hit point if there's a hit
        if raycast.hit_something {
            gizmos.sphere(raycast.hit_point, 0.1, ray_color);
        }
    }
}