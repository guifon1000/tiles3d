
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::player::{calculate_subpixel_center_world_position, Player, EntitySubpixelPosition};
use crate::planisphere::{self, Planisphere};
use crate::terrain::TerrainCenter;


trait IntoWorldPosition{
    fn into_world_position(&self, planisphere: &planisphere::Planisphere, terrain_center: &crate::terrain::TerrainCenter) -> Vec3;
}

impl IntoWorldPosition for Vec3 {
    fn into_world_position(&self, _planisphere: &planisphere::Planisphere, _terrain_center: &crate::terrain::TerrainCenter) -> Vec3 {
        *self
    }
}

impl IntoWorldPosition for (usize, usize, usize) {
    fn into_world_position(&self, planisphere: &planisphere::Planisphere, terrain_center: &crate::terrain::TerrainCenter) -> Vec3 {
        calculate_subpixel_center_world_position(self.0 as i32, self.1 as i32, self.2 as i32, planisphere, terrain_center)
    }
}


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


#[derive(Clone)]
pub struct ObjectTemplate {
    pub name: String,
    pub scene: Handle<Scene>,  // Use scene instead of mesh_parts
    pub y_offset: f32,
    pub scale: Vec3,
    pub rotation_y: f32,  // Rotation around Y-axis in radians
    pub object_definition: ObjectDefinition, // Default definition for this template
}

#[derive(Resource)]
pub struct ObjectTemplates {
    pub tree: ObjectTemplate,
    pub rock: ObjectTemplate,
    pub robot: ObjectTemplate,
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




pub fn setup_entity_overlays(
    mut commands: Commands,
    new_entities: Query<Entity, (With<EntityInfoOverlay>, Without<EntityUIText>)>,
) {
    for entity in new_entities.iter() {
        println!("Creating overlay for entity {:?}", entity);
        
        // CRITICAL: Mark this entity as having UI created
        commands.entity(entity).insert(EntityUIText { target_entity: entity });
        
        commands.spawn((
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
        });
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
            commands.entity(ui_entity).despawn();
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
        //eprint!("Raycasting from entity {:?} at position {:?}", entname, ray_origin);
        let ray_direction = Vec3::new(0.0, -1.0, 0.0); // Downward raycast
        let filter = QueryFilter::new().exclude_rigid_body(entity_id);
        if let Some((entity, ray_intersection)) = ctx.cast_ray_and_get_normal(ray_origin, ray_direction, f32::MAX, true, filter) {
            if let Ok(tile_entity) = terrain_entities.get(entity) {
                //eprint!("Raycast hit tile entity: {:?}", tile_entity);
            if terrain_entities.contains(tile_entity) {
                //eprintln!("Raycast hit terrain tile entity: {:?}", tile_entity);
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
                //eprintln!("Raycast hit tile: {} {} {}", _subpixel_position.0, _subpixel_position.1, _subpixel_position.2);

                // You can update locator.last_tile here if you want
            }
            }
        }
    }
}



    
    





pub fn spawn_mouse_tracker(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: &crate::planisphere::Planisphere,
    terrain_center: &crate::terrain::TerrainCenter,
) -> Entity {
    let world_pos = Vec3::new(0.0, 10.0, 0.0); // Start above ground
        let object_definition = ObjectDefinition {
        shape: ObjectShape::Sphere { radius: 0.3},
        color: Color::srgb(0.0, 0.3, 0.7), // Red color for player
        collision: CollisionBehavior::None,
        existence_conditions: Some(ExistenceConditions::Always),
        object_type: "MouseTracker".to_string(),
        scale: Vec3::ONE,
        y_offset: 5.0, // Start above ground
        mesh: None,
        material: None,
    };

    let position = IntoWorldPosition::into_world_position(&world_pos, planisphere, terrain_center);

    let entity = spawn_unified_object(
        commands,
        meshes,
        materials,
        planisphere, // No planisphere needed for player
        terrain_center, // Dummy terrain center
        position,
        5.0, // Use player's Y position + offset
        CollisionBehavior::Static, // No collision for mouse tracker
        object_definition,
        (EntitySubpixelPosition::default(), 
            EntityInfoOverlay::default(), 
            RaycastTileLocator{last_tile: None}, 
            MouseTrackerObject{}
        ), // Pass the bundle and physics components
    );

    // Attach player-specific components
    
    // Add children (sensor, marker) as before if needed

    entity
}






// This is a proper Bevy system function that will be scheduled correctly
pub fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    planisphere: Res<Planisphere>,
    terrain_center: ResMut<TerrainCenter>,
    object_templates: Res<ObjectTemplates>,  // This will access the resource only after it exists
) {
    // Call the spawn_player function
    spawn_player(
        &mut commands, 
        &mut materials, 
        &planisphere, 
        &terrain_center, 
        &object_templates
    );

    spawn_mouse_tracker(
        &mut commands,
        &mut meshes,
        &mut materials,
        &planisphere,
        &terrain_center,
    );
    

}




















pub fn spawn_player(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: &crate::planisphere::Planisphere,
    terrain_center: &crate::terrain::TerrainCenter,
    object_templates: &ObjectTemplates,
) {


    // Build the bundle with the correct transform
    let player_bundle = crate::player::PlayerBundle {
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
    );




    let template = object_templates.robot.clone(); // Use the robot template for player

    let entity =spawn_template_scene(
                    commands,
                    materials,
                    planisphere,
                    terrain_center,
                    &template,
                    Vec3::new(0.0, 10.0, 0.),
                    10.0, // Use player's Y position + offset
                    CollisionBehavior::Dynamic, // Set collision behavior to dynamic for dropped items
                    (
                        player_bundle,
                        physics_bundle, 
                        crate::game_object::RaycastTileLocator{last_tile: None}, 
                        crate::game_object::EntityInfoOverlay::default(),
                    )
                );



    // Attach player-specific components
    
    // Add children (sensor, marker) as before if needed

    
}




pub fn setup_object_templates(mut commands: Commands, asset_server: Res<AssetServer>)  {
    let object_templates = ObjectTemplates {
        tree: ObjectTemplate {
            name: "Tree".to_string(),
            scene: asset_server.load("meshes/tree1.glb#Scene0"),
            y_offset: 0.0,  // No manual offset needed!
            scale: Vec3::ONE,
            rotation_y: 0.0,  // No rotation by default
            object_definition: ObjectDefinition {
                shape: ObjectShape::Cube { size: Vec3::ONE }, // Default shape
                color: Color::srgb(0.0, 1.0, 0.0), // Green color for trees
                collision: CollisionBehavior::Static,
                existence_conditions: Some(ExistenceConditions::Always),
                object_type: "Tree".to_string(),
                scale: Vec3::ONE,
                y_offset: 0.0,
                mesh: None, // No specific mesh for tracker
                material: None, // No specific material for tracker
            },
        },
        rock: ObjectTemplate {
            name: "Stone".to_string(),
            scene: asset_server.load("meshes/stone1.glb#Scene0"),
            y_offset: 0.0,  // No manual offset needed!
            scale: Vec3::ONE,
            rotation_y: 0.0,  // No rotation by default
            object_definition: ObjectDefinition {
                shape: ObjectShape::Cube { size: Vec3::ONE }, // Default shape
                color: Color::srgb(0.0, 1.0, 0.0), // Green color for trees
                collision: CollisionBehavior::Static,
                existence_conditions: Some(ExistenceConditions::Always),
                object_type: "Stone".to_string(),
                scale: Vec3::ONE,
                y_offset: 0.0,
                mesh: None, // No specific mesh for tracker
                material: None, // No specific material for tracker
            },
        },
        robot: ObjectTemplate {
            name: "Player".to_string(),
            scene: asset_server.load("meshes/robot1.glb#Scene0"),
            y_offset: 0.0,  // No manual offset needed!
            scale: 0.04 * Vec3::ONE,
            rotation_y: std::f32::consts::PI,  // 180 degrees in radians
            object_definition: ObjectDefinition {
                shape: ObjectShape::Cube { size: Vec3::ONE }, // Default shape
                color: Color::srgb(0.0, 1.0, 0.0), // Green color for trees
                collision: CollisionBehavior::Dynamic,
                existence_conditions: Some(ExistenceConditions::Always),
                object_type: "Player".to_string(),
                scale: 0.0001*Vec3::ONE,
                y_offset: 0.0,
                mesh: None, // No specific mesh for tracker
                material: None, // No specific material for tracker
            },
        },
    };
    
    commands.insert_resource(object_templates);
}







pub fn despawn_unified_object_from_name(
    commands: &mut Commands,
    object_type: &str,
    query : Query<(Entity, &mut Transform,  &ObjectDefinition), (Without<Player>, Without<MouseTrackerObject>) >,
) {
    for (entity, object_transform, object_definition) in query.iter() {
        if object_definition.object_type == object_type {
            commands.entity(entity).despawn();
        }
    }
}
 
pub fn despawn_unified_objects_from_name(
    commands: &mut Commands,
    object_type: &str,
query : Query<(Entity, &mut Transform,  &ObjectDefinition), (Without<Player>, Without<MouseTrackerObject>) >,
) {
    for (entity, object_transform, object_definition) in query.iter() {
        if object_definition.object_type.contains(object_type) {
            commands.entity(entity).despawn();
        }
    }
}


pub fn spawn_template_scene<Extra: Bundle, T: IntoWorldPosition>(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: &crate::planisphere::Planisphere,
    terrain_center: &crate::terrain::TerrainCenter,
    template: &ObjectTemplate, // Use your existing template
    position : T,
    y_offset: f32,
    collision: CollisionBehavior,
    extra: Extra, // <-- new parameter for extra components/bundles
) -> Entity {
    let world_pos = position.into_world_position(planisphere, terrain_center);
    
    // Create parent entity
    let parent = commands.spawn((
        Transform::from_translation(world_pos+Vec3::new(0.0, y_offset, 0.0)),
        Visibility::default(),
        ObjectDefinition {
            shape: ObjectShape::Cube { size: Vec3::ONE },
            color: Color::WHITE,
            collision,
            existence_conditions: Some(ExistenceConditions::Always),
            object_type: template.name.clone(),
            scale: template.scale,
            y_offset: 0.0,
            mesh: None,
            material: None,
        },
        extra
    )).id();

    // Spawn the scene as a child of the parent entity
    let part_entity = commands.spawn((
        SceneRoot(template.scene.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: template.object_definition.color,
            perceptual_roughness: 0.5,
            metallic: 0.0,
            ..default()
        })),
        Transform::from_translation(Vec3::new(0.0, template.y_offset, 0.0))
            .with_scale(template.scale)
            .with_rotation(Quat::from_rotation_y(template.rotation_y)),

    )).id();

    commands.entity(parent).add_child(part_entity);
    // Spawn the scene from the template
    parent
}


/// Spawn a unified object based on an ObjectDefinition
pub fn spawn_unified_object<Extra: Bundle, T: IntoWorldPosition>(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    planisphere: &crate::planisphere::Planisphere,
    terrain_center: &crate::terrain::TerrainCenter,
    position : T,
    y_offset: f32,
    collision: CollisionBehavior,
    definition: ObjectDefinition,
    extra: Extra, // <-- new parameter for extra components/bundles

) -> Entity {
    // Determine world position
    //eprintln!("Spawning object of type: {}", definition.object_type);
    let world_pos =  position.into_world_position(planisphere, terrain_center);

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