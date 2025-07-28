use std::num::NonZero;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_rich_text3d::{Text3d, Text3dPlugin, Text3dStyling, TextAtlas, Text3dSegment};
use crate::player::{calculate_subpixel_center_world_position, PlayerBundle, EntitySubpixelPosition};
use crate::terrain::SubpixelPosition;

#[derive(Component)]
pub struct SubpixelTextTag;
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

#[derive(Component, Debug, Clone)]
pub struct RaycastTileLocator {
    pub last_tile: Option<(usize, usize, usize)>,
}

/// Component pour marquer les entités qui doivent avoir un overlay UI
#[derive(Component)]
pub struct EntityInfoOverlay {
    pub show_subpixel: bool,
    pub show_coordinates: bool,
    pub offset: Vec2, // Offset from entity position in pixels
}

impl Default for EntityInfoOverlay {
    fn default() -> Self {
        Self {
            show_subpixel: true,
            show_coordinates: false,
            offset: Vec2::new(0.0, -50.0), // Au-dessus de l'entité
        }
    }
}

/// Component pour identifier les UI text overlays
#[derive(Component)]
pub struct EntityUIText {
    pub target_entity: Entity,
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
    pub mesh: Option<Handle<Mesh>>,                // NEW: Optional mesh handle
    pub material: Option<Handle<StandardMaterial>>, // NEW: Optional material handle
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
        mesh: None, // No specific mesh for tracker
        material: None, // No specific material for tracker
    };

    // Spawn the beacon using the unified spawn function
    spawn_unified_object(commands, meshes, materials, planisphere, terrain_center, object_definition, ());
}






pub fn get_entity_tile_raycast(
    object_query: Query<(Entity, &ObjectDefinition, &Transform)>,
    rapier_context: ReadRapierContext,
    terrain_center: Res<crate::terrain::TerrainCenter>,
    planisphere: Res<crate::planisphere::Planisphere>,
    mut commands: Commands,
    name: &str,
)-> (usize, usize, usize) {
    for (entity, object_definition, transform) in object_query.iter() {
        if object_definition.object_type == name {
            // Get the world position of the entity
            let world_pos = transform.translation;
        }
    }
    (0, 0, 0) // Placeholder implementation

}





/// Système pour créer les overlays UI pour les nouvelles entités
pub fn setup_entity_overlays(
    mut commands: Commands,
    new_entities: Query<Entity, (With<EntityInfoOverlay>, Without<EntityUIText>)>,
) {
    for entity in new_entities.iter() {
        // Créer un nœud UI pour cet overlay
        let ui_entity = commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Auto,
                height: Val::Auto,
                padding: UiRect::all(Val::Px(5.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            BorderRadius::all(Val::Px(5.0)),
            Visibility::Hidden, // Caché par défaut
            EntityUIText { target_entity: entity },
        )).with_children(|parent| {
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        }).id();
    }
}

// Système de mise à jour simplifié (remplace update_subpixel_text_system)
pub fn update_subpixel_display_system(
    // Plus besoin de parcourir les enfants !
    query: Query<&EntitySubpixelPosition, Changed<EntitySubpixelPosition>>,
) {
    for subpixel in query.iter() {
        // Le système update_entity_ui_overlays s'occupe automatiquement de l'affichage
        eprintln!("Subpixel position updated: {:?}", subpixel.subpixel);
    }
}

pub fn update_entity_ui_overlays(
    // Entités avec overlay
    entity_query: Query<(Entity, &Transform, &EntitySubpixelPosition, &EntityInfoOverlay)>,
    
    // UI overlays
    mut ui_query: Query<(&mut Node, &mut Visibility, &EntityUIText, &Children)>,
    mut text_query: Query<&mut Text>,
    
    // Camera et window pour la projection
    camera_query: Query<(&Camera, &GlobalTransform)>,
    window_query: Query<&Window>,
) {
    let Ok((camera, camera_transform)) = camera_query.single() else { return; };
    let Ok(window) = window_query.single() else { return; };

    for (mut style, mut visibility, ui_text, children) in ui_query.iter_mut() {
        // Trouver l'entité cible
        if let Ok((entity, transform, subpixel_pos, overlay_config)) = entity_query.get(ui_text.target_entity) {
            
            // Projeter la position 3D vers 2D
            let world_pos = transform.translation;
            
            if let Ok(screen_pos) = camera.world_to_viewport(camera_transform, world_pos) {
                // L'entité est visible à l'écran
                *visibility = Visibility::Visible;
                
                // Positionner l'overlay avec l'offset
                let final_x = screen_pos.x + overlay_config.offset.x;
                let final_y = screen_pos.y + overlay_config.offset.y;
                style.left = Val::Px(final_x);
                style.top = Val::Px(final_y);
                
                // Mettre à jour le texte
                if let Some(child) = children.first() {
                    if let Ok(mut text) = text_query.get_mut(*child) {
                        let mut content = String::new();
                        
                        if overlay_config.show_subpixel {
                            content.push_str(&format!("Tile: ({}, {}, {})", 
                                subpixel_pos.subpixel.0, 
                                subpixel_pos.subpixel.1, 
                                subpixel_pos.subpixel.2
                            ));
                        }
                        
                        if overlay_config.show_coordinates {
                            if !content.is_empty() { content.push('\n'); }
                            content.push_str(&format!("Pos: ({:.1}, {:.1}, {:.1})", 
                                world_pos.x, world_pos.y, world_pos.z
                            ));
                        }
                        
                        **text = content;
                    }
                }
            } else {
                // L'entité n'est pas visible
                *visibility = Visibility::Hidden;
            }
        } else {
            // L'entité cible n'existe plus, cacher l'overlay
            *visibility = Visibility::Hidden;
        }
    }
}


/// Système pour nettoyer les overlays des entités supprimées
pub fn cleanup_orphaned_overlays(
    mut commands: Commands,
    ui_query: Query<(Entity, &EntityUIText)>,
    entity_query: Query<Entity, With<EntityInfoOverlay>>,
) {
    for (ui_entity, ui_text) in ui_query.iter() {
        // Si l'entité cible n'existe plus, supprimer l'overlay
        if entity_query.get(ui_text.target_entity).is_err() {
            commands.entity(ui_entity).despawn_recursive();
        }
    }
}











// Modifiez votre fonction spawn_dynamic_object_with_raycast
pub fn spawn_dynamic_object_with_raycast_with_ui(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: Option<&crate::planisphere::Planisphere>,
    terrain_center: &crate::terrain::TerrainCenter,
    object_definition: ObjectDefinition,
) -> Entity {
    // Votre code existant...
    let entity_subpixel_position = crate::player::EntitySubpixelPosition::default();
    let raycast_tile_locator = RaycastTileLocator { last_tile: None };

    let physics_bundle = (
        RigidBody::Dynamic,
        create_collider_from_shape(&object_definition.shape),
        Velocity { linvel: Vec3::ZERO, angvel: Vec3::ZERO },
        ExternalImpulse::default(),
        GravityScale(1.0),
        Damping { linear_damping: 0.0, angular_damping: 0.1 },
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
        ActiveEvents::COLLISION_EVENTS,
        ActiveCollisionTypes::all(),
    );

    // Ajouter l'overlay UI au lieu du texte 3D
    let extra = (
        entity_subpixel_position,
        raycast_tile_locator,
        physics_bundle,
        EntityInfoOverlay::default(), // <- Ajouter ça !
    );

    // Spawn l'objet sans le texte 3D enfant
    spawn_unified_object(
        commands,
        meshes,
        materials,
        planisphere,
        terrain_center,
        object_definition,
        extra,
    )
}

pub fn spawn_dynamic_object_with_raycast(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: Option<&crate::planisphere::Planisphere>,
    terrain_center: &crate::terrain::TerrainCenter,
    object_definition: ObjectDefinition,
) -> Entity {
    // Create the components you want to attach
    let entity_subpixel_position = crate::player::EntitySubpixelPosition::default();
    let entity_subpixel_position_for_text = entity_subpixel_position.clone();
    let raycast_tile_locator = RaycastTileLocator { last_tile: None };

    // Physics bundle (customize shape as needed)
    let physics_bundle = (
        RigidBody::Dynamic,
        create_collider_from_shape(&object_definition.shape),
        Velocity { linvel: Vec3::ZERO, angvel: Vec3::ZERO },
        ExternalImpulse::default(),
        GravityScale(1.0),
        Damping { linear_damping: 0.0, angular_damping: 0.1 },
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
        ActiveEvents::COLLISION_EVENTS,
        ActiveCollisionTypes::all(),
    );



    // Compose all extra components as a tuple
    let extra = (
        entity_subpixel_position,
        raycast_tile_locator,
        physics_bundle,
    );

    // Spawn the object using the unified function
    let entity = spawn_unified_object(
        commands,
        meshes,
        materials,
        planisphere,
        terrain_center,
        object_definition,
        extra,
    );

    let mat = materials.add(StandardMaterial {
        base_color_texture: Some(TextAtlas::DEFAULT_IMAGE.clone_weak()),
        alpha_mode: AlphaMode::Mask(0.5),
        unlit: true,
        cull_mode: None,
        ..Default::default()
    });

    let esp = entity_subpixel_position_for_text.subpixel.clone();
    // Spawn the text as a child
    commands.entity(entity).with_children(|parent| {
        parent.spawn((
        Text3d::new(format!("{:?}", entity_subpixel_position_for_text.clone().subpixel)),
        Text3dStyling {
            size: 64.,
            stroke: NonZero::new(10),
            color: Srgba::new(1., 0., 0., 1.),
            stroke_color: Srgba::BLACK,
            world_scale: Some(Vec2::splat(0.25)),
            layer_offset: 0.001,
            ..Default::default()
        },
        Mesh3d::default(),
        MeshMaterial3d(mat.clone()),
        Transform {
            translation: Vec3::new(0., 2., 0.),
            rotation: Quat::IDENTITY,//Quat::from_axis_angle(Vec3::Y, -30.),
            scale: Vec3::ONE,
        },
        
        SubpixelTextTag,
    ));
    });
    entity


}


pub fn update_subpixel_text_system(
    parent_query: Query<(&EntitySubpixelPosition, &Children)>,
    mut text_query: Query<(&mut Text3d, &SubpixelTextTag)>,
) {
    eprint!(    "Updating subpixel text for entities...\n");
    for (subpixel, children) in parent_query.iter() {
        //eprint!("Updating subpixel text for entity with subpixel: {:?}\n", subpixel.subpixel);
        for child in children.iter() {
            if let Ok((mut text, _)) = text_query.get_mut(child) {
                // Update the text content directly
                eprintln!("Updating text for child entity {:?} with subpixel: {:?}", child, subpixel.subpixel);
                *text = Text3d::new(format!("{:?}", subpixel.subpixel));
            }
        }
    }
}

pub fn raycast_tile_locator_system(
    mut query: Query<(Entity, &Transform, &mut RaycastTileLocator, &mut EntitySubpixelPosition, &mut ObjectDefinition)>,
    rapier_context: ReadRapierContext,
    triangle_mapping: Res<crate::terrain::TriangleSubpixelMapping>,
    terrain_entities: Query<Entity, With<crate::terrain::Tile>>,
) {
    let Ok(ctx) = rapier_context.single() else { return; };

    for (entity_id, transform, mut locator, mut subpixel_position, object_definition) in query.iter_mut() {
        // Perform raycast from the entity's position
        let entname = object_definition.object_type.clone();
        let ray_origin = transform.translation + Vec3::new(0.0, 10.0, 0.0); // Start raycast slightly above the entity
        eprint!("Raycasting from entity {:?} at position {:?}", entname, ray_origin);
        let ray_direction = Vec3::new(0.0, -1.0, 0.0); // Downward raycast
        let filter = QueryFilter::new().exclude_rigid_body(entity_id);
        if let Some((entity, ray_intersection)) = ctx.cast_ray_and_get_normal(ray_origin, ray_direction, f32::MAX, true, filter) {
            if let Ok(tile_entity) = terrain_entities.get(entity) {
                eprint!("Raycast hit tile entity: {:?}", tile_entity);
            if terrain_entities.contains(tile_entity) {
                eprintln!("Raycast hit terrain tile entity: {:?}", tile_entity);
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
                let _subpixel_position = triangle_mapping.triangle_to_subpixel[adjusted_triangle_index as usize];
                subpixel_position.subpixel.0 = _subpixel_position.0;
                subpixel_position.subpixel.1 = _subpixel_position.1;
                subpixel_position.subpixel.2 = _subpixel_position.2;
                eprintln!("Raycast hit tile: {} {} {}", _subpixel_position.0, _subpixel_position.1, _subpixel_position.2);

                // You can update locator.last_tile here if you want
            }
            }
        }
    }
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
        shape: ObjectShape::Sphere { radius: 0.1 },
        color: Color::srgb(0.0, 1.0, 1.0), // Blue color for mouse trackers
        collision: CollisionBehavior::None,
        existence_conditions: Some(ExistenceConditions::OnFrame), // Exists for the current frame only
        object_type: "MouseTracker_world".to_string(),
        scale: Vec3::ONE,
        y_offset: 0.0,
        mesh: None, // No specific mesh for tracker
        material: None, // No specific material for tracker
    };

    // Spawn the beacon using the unified spawn function
    spawn_unified_object(commands, meshes, materials, planisphere, terrain_center, object_definition, ());
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
        mesh: None, // No specific mesh for tracker
        material: None, // No specific material for tracker
    };

    // Spawn the beacon using the unified spawn function
    spawn_unified_object(commands, meshes, materials, planisphere, terrain_center, object_definition, ());
}

pub fn spawn_player(
    commands: &mut Commands,    
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: Option<&crate::planisphere::Planisphere>,
    terrain_center: &crate::terrain::TerrainCenter,
) -> Entity {

    let world_pos = Vec3::new(0.0, 10.0, 0.0);


    // Create mesh and material
    let player_mesh = meshes.add(Capsule3d::new(0.3, 0.8));
    let player_material = materials.add(Color::srgb(0.9, 0.1, 0.1));

    let raycast_tile_locator = RaycastTileLocator {
        last_tile: None, // Initialize with no last tile
    };
    // Build the bundle with the correct transform
    let bundle = crate::player::PlayerBundle {
        ..Default::default()
    };
    let physics_bundle = (
        RigidBody::Dynamic,
        Collider::capsule_y(0.3, 0.4),
        Velocity { linvel: Vec3::new(0.0, -0.1, 0.0), angvel: Vec3::ZERO },
        ExternalImpulse::default(),
        GravityScale(1.0),
        Damping { linear_damping: 0.0, angular_damping: 0.1 },
        LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
        ActiveEvents::COLLISION_EVENTS,
        ActiveCollisionTypes::all(),
        raycast_tile_locator, // Attach the raycast tile locator
    );



    let object_definition = ObjectDefinition {
        position: ObjectPosition::WorldCoordinates(world_pos),
        shape: ObjectShape::Capsule { radius: 0.3, height: 0.8 },
        color: Color::srgb(0.9, 0.1, 0.1), // Red color for player
        collision: CollisionBehavior::Dynamic,
        existence_conditions: Some(ExistenceConditions::Always),
        object_type: "Player".to_string(),
        scale: Vec3::ONE,
        y_offset: 5.0, // Start above ground
        mesh: Some(player_mesh),
        material: Some(player_material),
    };


    let entity = spawn_unified_object(
        commands,
        meshes,
        materials,
        planisphere, // No planisphere needed for player
        terrain_center, // Dummy terrain center
        object_definition,
        (bundle, physics_bundle), // Pass the bundle and physics components
    );

    // Attach player-specific components
    
    // Add children (sensor, marker) as before if needed

    entity
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
        mesh: None, // No specific mesh for tracker
        material: None, // No specific material for tracker
    };

    // Spawn the landscape element using the unified spawn function
    spawn_unified_object(commands, meshes, materials, planisphere, terrain_center, object_definition, ());
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
        shape: ObjectShape::Sphere { radius: 0.3 },
        color: Color::srgb(0.0, 0.0, 1.0), // Red color for beacons
        collision: CollisionBehavior::None,
        existence_conditions: Some(ExistenceConditions::OnFrame), // Exists for the current frame only
        object_type: "MouseTracker".to_string(),
        scale: Vec3::ONE,
        y_offset: 0.0,
        mesh: None, // No specific mesh for tracker
        material: None, // No specific material for tracker
    };


    
    // Spawn the beacon using the unified spawn function
    spawn_unified_object(commands, meshes, materials, planisphere, terrain_center, object_definition,());
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
pub fn spawn_unified_object<Extra: Bundle>(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: Option<&crate::planisphere::Planisphere>,
    terrain_center: &crate::terrain::TerrainCenter,
    definition: ObjectDefinition,
    extra: Extra, // <-- new parameter for extra components/bundles

) -> Entity {
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
    let mesh_handle = if let Some(mesh) = &definition.mesh {
        mesh.clone()
    } else {
        create_mesh_from_shape(&definition.shape, meshes)
    };
    let material_handle = if let Some(material) = &definition.material {
        material.clone()
    } else {
        materials.add(StandardMaterial {
            base_color: definition.color,
            perceptual_roughness: 0.5,
            metallic: 0.0,
            ..default()
        })
    };
    
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
        extra, // Insert extra components/bundles here
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
    entity_commands.id()
    //println!("Spawned {} object at {:?}", definition.object_type, final_position);
}