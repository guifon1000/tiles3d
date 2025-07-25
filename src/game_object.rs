use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::player::calculate_subpixel_center_world_position;
/// Position specification for spawning objects
#[derive(Debug, Clone)]
pub enum ObjectPosition {
    WorldCoordinates(Vec3),
    TileIndices(usize, usize, usize), // (i, j, k)
}

/// Shape specification for creating meshes and colliders
#[derive(Debug, Clone)]
pub enum ObjectShape {
    Cube { size: Vec3 },
    Sphere { radius: f32 },
    Capsule { radius: f32, height: f32 }, 
    Cylinder { radius: f32, height: f32 },
}

/// Collision behavior for physics bodies
#[derive(Debug, Clone)]
pub enum CollisionBehavior {
    None,                    // No collision (beacons)
    Static,                  // Fixed collision (landscape elements)
    Dynamic,                 // Can be moved by physics
}

#[derive(Component, Debug, Clone)]
pub enum ExistenceConditions {
    Always,                 // Always exists
    OnCondition(String),    // Exists based on a specific condition (e.g., player state)
    OnEvent(String),        // Exists when a specific event occurs
    OnFrame,                // Exists for the current frame only
}

/// Unified object definition for spawning various game objects
#[derive(Component, Debug, Clone)]
pub struct ObjectDefinition {
    pub position: ObjectPosition,
    pub shape: ObjectShape,
    pub color: Color,
    pub collision: CollisionBehavior,
    pub existence_conditions: Option<ExistenceConditions>, // Optional conditions for existence
    pub object_type: String,
    pub scale: Vec3,
    pub y_offset: f32,       // Vertical offset from ground
}


/// Marker component for mouse tracker objects
#[derive(Component, Debug, Clone)]
pub struct MouseTrackerObject {
   
}

/// Create a mesh handle from an ObjectShape specification
pub fn create_mesh_from_shape(shape: &ObjectShape, meshes: &mut ResMut<Assets<Mesh>>) -> Handle<Mesh> {
    match shape {
        ObjectShape::Cube { size } => {
            meshes.add(Cuboid::new(size.x, size.y, size.z))
        }
        ObjectShape::Sphere { radius } => {
            meshes.add(Sphere::new(*radius))
        }
        ObjectShape::Capsule { radius, height } => {
            meshes.add(Capsule3d::new(*radius, *height))
        }
        ObjectShape::Cylinder { radius, height } => {
            meshes.add(Cylinder::new(*radius, *height))
        }
    }
}

/// Create a collider from an ObjectShape specification
pub fn create_collider_from_shape(shape: &ObjectShape) -> Collider {
    match shape {
        ObjectShape::Cube { size } => {
            Collider::cuboid(size.x / 2.0, size.y / 2.0, size.z / 2.0)
        }
        ObjectShape::Sphere { radius } => {
            Collider::ball(*radius)
        }
        ObjectShape::Capsule { radius, height } => {
            Collider::capsule_y(*height / 2.0, *radius)
        }
        ObjectShape::Cylinder { radius, height } => {
            Collider::cylinder(*height / 2.0, *radius)
        }
    }
}

pub fn spawn_playertracker_at_tile(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: Option<&crate::planisphere::Planisphere>,
    terrain_center: &crate::terrain::TerrainCenter,
    i: usize,
    j: usize,
    k: usize,
) {
    // Create a beacon object definition
    let object_definition = ObjectDefinition {
        position: ObjectPosition::TileIndices(i, j, k),
        shape: ObjectShape::Capsule { radius: 0.8, height: 1.6 },
        color: Color::srgb(1.0, 0.0, 0.0), // Red color for beacons
        collision: CollisionBehavior::None,
        existence_conditions: Some(ExistenceConditions::OnFrame), // Exists for the current frame only
        object_type: "PlayerTracker".to_string(),
        scale: Vec3::ONE,
        y_offset: 0.0,
    };

    // Spawn the beacon using the unified spawn function
    spawn_unified_object(commands, meshes, materials, planisphere, terrain_center, object_definition);
}



pub fn spawn_mouse_tracker_at_world_position(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: Option<&crate::planisphere::Planisphere>,
    terrain_center: &crate::terrain::TerrainCenter,
    position: Vec3,
) {
    // Create a mouse tracker object definition
    let object_definition = ObjectDefinition {
        position: ObjectPosition::WorldCoordinates(position),
        shape: ObjectShape::Sphere { radius: 0.4 },
        color: Color::srgb(0.0, 0.0, 1.0), // Blue color for mouse trackers
        collision: CollisionBehavior::None,
        existence_conditions: Some(ExistenceConditions::OnFrame), // Exists for the current frame only
        object_type: "MouseTracker_world".to_string(),
        scale: Vec3::ONE,
        y_offset: 0.0,
    };

    // Spawn the beacon using the unified spawn function
    spawn_unified_object(commands, meshes, materials, planisphere, terrain_center, object_definition);
}


pub fn spawn_terraincenter_at_world_position(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: Option<&crate::planisphere::Planisphere>,
    terrain_center: &crate::terrain::TerrainCenter,
    position: Vec3,
) {
    // Create a terrain center object definition
    let object_definition = ObjectDefinition {
        position: ObjectPosition::WorldCoordinates(position),
        shape: ObjectShape::Sphere { radius: 0.5 },
        color: Color::srgb(0.0, 1.0, 1.0), // Cyan color for terrain center
        collision: CollisionBehavior::None,
        existence_conditions: Some(ExistenceConditions::Always), // Always exists
        object_type: "TerrainCenter".to_string(),
        scale: Vec3::ONE,
        y_offset: 0.0,
    };

    // Spawn the beacon using the unified spawn function
    spawn_unified_object(commands, meshes, materials, planisphere, terrain_center, object_definition);
}

pub fn spawn_landscape_element_at_tile(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: Option<&crate::planisphere::Planisphere>,
    terrain_center: &crate::terrain::TerrainCenter,
    i: usize,
    j: usize,
    k: usize,
) {
    // Create a landscape element definition
    let object_definition = ObjectDefinition {
        position: ObjectPosition::TileIndices(i, j, k),
        shape: ObjectShape::Cube { size: Vec3::new(1.0, 2.0, 1.0) }, // Example shape
        color: Color::srgb(0.5, 0.5, 0.5), // Gray color for landscape elements
        collision: CollisionBehavior::Static,
        existence_conditions: Some(ExistenceConditions::Always), // Always exists
        object_type: "LandscapeElement".to_string(),
        scale: Vec3::ONE,
        y_offset: 0.0,
    };

    // Spawn the landscape element using the unified spawn function
    spawn_unified_object(commands, meshes, materials, planisphere, terrain_center, object_definition);
}

pub fn spawn_mousetracker_at_tile(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: Option<&crate::planisphere::Planisphere>,
    terrain_center: &crate::terrain::TerrainCenter,
    i: usize,
    j: usize,
    k: usize,
) {
    // Create a beacon object definition

        
    let object_definition=ObjectDefinition {
        position: ObjectPosition::TileIndices(i, j, k),
        shape: ObjectShape::Sphere { radius: 0.8 },
        color: Color::srgb(1.0, 0.0, 0.0), // Red color for beacons
        collision: CollisionBehavior::None,
        existence_conditions: Some(ExistenceConditions::OnFrame), // Exists for the current frame only
        object_type: "MouseTracker".to_string(),
        scale: Vec3::ONE,
        y_offset: 0.0,
    };


    
    // Spawn the beacon using the unified spawn function
    spawn_unified_object(commands, meshes, materials, planisphere, terrain_center, object_definition);
}

pub fn despawn_unified_object_from_name(
    commands: &mut Commands,
    object_type: &str,
    query : Query<(Entity, &ObjectDefinition)>,
) {
    for (entity, object_definition) in query.iter() {
        if object_definition.object_type == object_type {
            commands.entity(entity).despawn();
        }
    }
}



/// Spawn a unified object based on an ObjectDefinition
pub fn spawn_unified_object(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: Option<&crate::planisphere::Planisphere>,
    terrain_center: &crate::terrain::TerrainCenter,
    definition: ObjectDefinition,
) {
    // Determine world position
    //eprintln!("Spawning object of type: {}", definition.object_type);
    let world_pos = match definition.position {
        ObjectPosition::WorldCoordinates(pos) => pos,
        ObjectPosition::TileIndices(i, j, k) => {
        if let Some(planisphere) = planisphere {
            //eprintln!("Spawning object at tile indices: ({}, {}, {})", i, j, k);
            
            calculate_subpixel_center_world_position(i as i32, j as i32, k as i32, planisphere, terrain_center) // planisphere.subpixel_to_world(i, j, k, center_lat)
        } else {
            eprintln!("Planisphere not available, using default position");
            Vec3::ZERO
        }
        }
    };
    //eprintln!("mean_tile_size: {}", planisphere.map_or(1.0, |p| p.mean_tile_size as f32));
    //eprintln!(  "World position for object {}: {:?}", definition.object_type, world_pos);
    // Apply scale and y_offset
    let final_position = Vec3::new(
        world_pos.x,
        world_pos.y + definition.y_offset,
        world_pos.z
    );
    
    // Create mesh and material
    let mesh_handle = create_mesh_from_shape(&definition.shape, meshes);
    let material_handle = materials.add(StandardMaterial {
        base_color: definition.color,
        ..default()
    });
    
    // Create transform with scale
    let transform = Transform {
        translation: final_position,
        scale: definition.scale,
        ..default()
    };
    
    // Build entity components
    let mut entity_commands = commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        transform,
        definition.clone(),
    ));
    
    // Add physics components based on collision behavior
    match definition.collision {
        CollisionBehavior::None => {
            // No physics components
        }
        CollisionBehavior::Static => {
            entity_commands.insert((
                RigidBody::Fixed,
                create_collider_from_shape(&definition.shape),
            ));
        }
        CollisionBehavior::Dynamic => {
            entity_commands.insert((
                RigidBody::Dynamic,
                create_collider_from_shape(&definition.shape),
            ));
        }
    }
    
    //println!("Spawned {} object at {:?}", definition.object_type, final_position);
}