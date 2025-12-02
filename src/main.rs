// Import statements - bring in code from external crates and our own modules
use bevy::{prelude::*, render::Render};           // Bevy game engine - the * imports everything commonly used
use bevy_rapier3d::prelude::*;  // Rapier physics engine - handles collision detection and physics
use bevy::pbr::wireframe::{Wireframe, WireframePlugin, WireframeConfig};
use serde::Deserialize;
use std::{collections::btree_set::Range, fs::File};
use std::io::Read;
// Module declarations - tell Rust about our other source files
mod terrain;     // terrain.rs - handles pure terrain mesh generation
mod landscape;   // landscape.rs - handles trees, rocks, items, and decorative elements
mod beacons;     // beacons.rs - handles debug beacons and visualization markers
mod agent;       // agent.rs - handles the autonomous agents that move around
mod camera;      // camera.rs - handles camera controls (zoom, rotation)
mod player;      // player.rs - handles the player character
mod planisphere; // planisphere.rs - handles geographic coordinate conversion and projections
mod ui ;        // ui.rs - handles user interface elements (like text, buttons, etc.)
mod game_object; // game_object.rs - handles object definitions and spawning logic
mod chunk;


// Import the specific functions we need from our modules
// 'use' statements make functions available in this file without the module prefix
use terrain::{create_terrain_gnomonic_rectangular, RenderedSubpixels, TriangleSubpixelMapping, TerrainCenter}; // Pure terrain mesh generation
use camera::{setup_third_person_camera, update_third_person_camera, third_person_camera_rotation, update_camera_light, handle_camera_zoom, handle_camera_height}; // Camera-related functions
use player::{move_player, check_player_sensors, check_player_ground_sensors, terrain_recreation_system}; // Player-related functions
use ui::{setup_ui, update_coordinate_display}; // UI setup function and coordinate display update system
use game_object::{setup_object_templates, cleanup_orphaned_overlays, setup_entity_overlays, 
    update_entity_ui_overlays, setup_player}; // Game object spawning and management
use crate::{game_object::EntitySubpixelPosition, planisphere::Planisphere};
use chunk::{ChunksCenter, Chunk, ChunkPixelStripe};
/// Configuration for terrain generation and management
#[derive(Resource)]
pub struct TerrainConfig {
    pub terrain_radius: usize,           // How far from center to generate terrain (in tiles)
    pub recreation_threshold: usize,     // Distance from center before recreating (auto-calculated as 1/4 radius)
    pub recreation_cooldown: f32,        // Minimum seconds between terrain recreations
    pub landscape_radius: usize,         // Radius for landscape elements (trees, rocks)
    pub item_radius: usize,              // Radius for collectible items
    pub beacon_radius: usize,            // Radius for debug beacons
    pub agent_search_radius: usize,      // Maximum search radius for agent respawning
}

/// Asset tracking for proper cleanup during terrain recreation
#[derive(Resource, Default)]
pub struct TerrainAssetTracker {
    pub terrain_meshes: Vec<Handle<Mesh>>,
    pub terrain_materials: Vec<Handle<StandardMaterial>>,
    pub landscape_meshes: Vec<Handle<Mesh>>,
    pub landscape_materials: Vec<Handle<StandardMaterial>>,
    pub texture_atlas: Option<Handle<Image>>, // Reusable
}

impl Default for TerrainConfig {
    fn default() -> Self {
        let radius = 200; // Main terrain radius setting - change this to adjust terrain size
        Self {
            terrain_radius: radius,
            recreation_threshold: radius / 4,  // Auto-calculate as 1/4 of radius
            recreation_cooldown: 1.0,
            landscape_radius: 3,               // Drastically reduced radius to limit landscape element count
            item_radius: 10,                   // Reduced radius to limit item count
            beacon_radius: 5,                  // Small radius for debug beacons
            agent_search_radius: 5,            // Reduced radius to prevent exponential nested loops
        }
    }
}




impl Resource for Planisphere {
    // This allows Planisphere to be used as a Bevy resource
    // Resources are global data that can be accessed by systems
}

impl TerrainAssetTracker {
    /// Clean up old asset handles before creating new terrain
    pub fn cleanup_assets(
        &mut self,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<StandardMaterial>>,
    ) {
        let total_meshes_before = self.terrain_meshes.len() + self.landscape_meshes.len();
        let total_materials_before = self.terrain_materials.len() + self.landscape_materials.len();
        
        // Remove terrain mesh assets from the asset system
        for mesh_handle in self.terrain_meshes.drain(..) {
            meshes.remove(&mesh_handle);
        }
        
        // Remove terrain material assets from the asset system
        for material_handle in self.terrain_materials.drain(..) {
            materials.remove(&material_handle);
        }
        
        // Remove landscape mesh assets from the asset system
        for mesh_handle in self.landscape_meshes.drain(..) {
            meshes.remove(&mesh_handle);
        }
        
        // Remove landscape material assets from the asset system
        for material_handle in self.landscape_materials.drain(..) {
            materials.remove(&material_handle);
        }
        
        // Note: We keep the texture atlas handle as it's reusable
        
        println!("ASSET CLEANUP: Removed {} meshes and {} materials from asset system", 
                 total_meshes_before, total_materials_before);

    }
}


#[derive(Resource)]
pub struct PlayerStart {
    pub start_ijk: (usize, usize, usize),
}

#[derive(Resource, Default)]
pub struct ChunksCreated {
    pub created: bool,
}





/// Main function - the entry point of our Rust program
/// This is where the program starts running when you execute it
fn main() {
    let sub_k = 8; // Number of subpixels in the vertical direction
    let image_path = "assets/maps/sphere_texture.png";


    // Initialize the Planisphere with the specified size and detail level
    let mut planisphere = Planisphere::from_elevation_map(image_path, sub_k)
        .expect("Failed to load elevation map");

    // Set the radius before making planisphere immutable
    let planisphere_width = planisphere.get_width_pixels();
    let circumference = planisphere_width * sub_k;
    let radius = circumference as f64 / (2. * std::f64::consts::PI);
    planisphere.set_radius(radius);
    eprintln!("Radius set to: {}", radius);

    // Compute initial subpixel from desired geographic coordinates
    let (mut initial_lon, mut initial_lat) = (7.7, -40.25);
    let (mut iplayer,  mut jplayer, mut kplayer) = planisphere.geo_to_subpixel(initial_lon, initial_lat);
    let ijk_coordinates = false;
    if ijk_coordinates == false {
        initial_lon = 7.7;
        initial_lat = -40.25;
        (iplayer, jplayer, kplayer) = planisphere.geo_to_subpixel(initial_lon, initial_lat);
    }
    else {
        let ijk_position = (450, 119, (sub_k+1)*sub_k/2+3);
        (iplayer, jplayer, kplayer) = ijk_position;
        (initial_lon, initial_lat) = planisphere.subpixel_to_geo(iplayer, jplayer, kplayer);
        let dbg_ijk = planisphere.geo_to_subpixel(initial_lon, initial_lat);
        eprintln!("debug  : back to ijk : {} {} {}      ", dbg_ijk.0, dbg_ijk.1, dbg_ijk.2);
    }
    let _subpixel_view_distance = 75;
    let _recreation_threshold  = (0.4 * _subpixel_view_distance as f32) as i32;
    let mut chunks_center_resource = ChunksCenter::new(iplayer, jplayer, 0, planisphere.clone(), (8,8));
    let player_start_resource = PlayerStart{start_ijk: (iplayer, jplayer, kplayer)};
    // Create and configure the Bevy App (the main game engine instance)
    App::new()
        // Add core Bevy plugins that provide essential functionality
        .add_plugins(DefaultPlugins)              // Graphics, audio, input, windowing, etc.
        
        // Add wireframe plugin for debugging
        .add_plugins(WireframePlugin::default())
        
        // Add physics simulation
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default()) // 3D physics with no custom user data
        

        // Uncomment the next line to see physics debug visualization (collision shapes, etc.)
        // .add_plugins(RapierDebugRenderPlugin::default()) // Debug disabled for cleaner visuals
        .insert_resource(planisphere)
        .insert_resource(TerrainConfig::default()) // Terrain configuration settings
        .insert_resource(TerrainAssetTracker::default()) // Asset tracking for cleanup
        // Add shared resources for player tracking and terrain management
         // Initialize Planisphere with size and detail

        //.init_resource::<PlayerSubpixelPosition>()
        .insert_resource(terrain::TerrainCenter {
            longitude: initial_lon,
            latitude: initial_lat,
            subpixel: (iplayer, jplayer, kplayer),
            world_position: Vec3::ZERO, // Initial world position at the center
            max_subpixel_distance: _subpixel_view_distance,
            last_recreation_time: -10.0,
            terrain_recreated: false,
            rendered_subpixels: RenderedSubpixels::new(),                //Vec<(usize, usize, usize, [(f64, f64); 4])>,
            triangle_mapping: TriangleSubpixelMapping::new(),
            mesh_tasks: Vec::new(),
            recreation_spawned: false,
        })
        .insert_resource(chunks_center_resource)
        .insert_resource(player_start_resource)
        .insert_resource(ChunksCreated::default())
        .insert_resource(RenderedSubpixels::new())
        .insert_resource(TriangleSubpixelMapping::default())
        .insert_resource(WireframeConfig {
            global: false,  // Back to component-based wireframe
            default_color: Color::WHITE,
        })
        
        
        // Systems that run once at startup (world setup)
        .add_systems(Startup, setup_third_person_camera) // Setup camera, physics world, and UI
        .add_systems(Startup, (setup_physics, setup_ui))
        .add_systems(Startup, (setup_object_templates, setup_player, game_object::raycast_tile_locator_system,).chain())
        .add_systems(Startup, setup_chunks)
        // Systems that run every frame (game loop) - split into groups to avoid tuple size limit
        .add_systems(Update, terrain_recreation_system)     // Handle terrain recreation with asset cleanup and coordinate sync
        .add_systems(Update, update_coordinate_display)      // Update the UI coordinate display with current player position
        .add_systems(Update, (
            //move_agents,                    // Update agent movement and behavior
            //check_sensors,                  // Handle agent item pickup detection
            //check_ground_sensors,           // Handle agent ground collision detection
            //agent_raycast_system,           // Handle agent raycasting for obstacle detection
            move_player,                    // Handle player movement with keyboard
            check_player_sensors,           // Handle player item pickup detection
            check_player_ground_sensors,    // Handle player ground collision detection
            setup_entity_overlays,          // Setup UI overlays for entities
            cleanup_orphaned_overlays,      // Clean up old UI overlays
            update_entity_ui_overlays,
            //player_raycast_system,          // Handle player raycasting for obstacle detection
        ))
        .add_systems(Update, (
            player::cast_ray_from_camera,
            player::detect_mouse_clicks,
            //track_entities_subpixel_position_raycast,
            game_object::raycast_tile_locator_system,
        ))
        
        .add_systems(Update, (
            update_third_person_camera,     // Update camera to follow player
            third_person_camera_rotation,   // Handle camera rotation with mouse
            handle_camera_zoom,             // Handle mouse wheel zoom
            handle_camera_height,           // Handle keyboard arrow keys for height
            update_camera_light,           // Update light to follow camera
        ))
        //.add_systems(Update, (
            //update_landscape_lod,          // Update landscape element LOD and culling
            //cull_landscape_by_terrain,     // Hide landscape elements outside rendered terrain
            //debug_agent_raycast_system,    // Debug visualization for agent raycasts
            //track_player_subpixel_position_raycast_old, // DEPRECATED: Keep for compatibility (will be removed)
            
        //))
        
        // Start the game loop - this runs until the window is closed
        .run();
}

fn setup_chunks_MISTRAL(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    planisphere: Res<Planisphere>,
    playerstart: Res<PlayerStart>,
    mut chunks_center: ResMut<ChunksCenter>,
    mut chunks_created: ResMut<ChunksCreated>,
    terrain_center: Res<TerrainCenter>,
) {
    eprintln!("=== SETUP_CHUNKS SYSTEM STARTED ===");
    if chunks_created.created {
        eprintln!("Chunks already created, returning");
        return;
    }
    chunks_created.created = true;
    eprintln!("Starting chunk creation process");

    let (i, j, k) = playerstart.start_ijk;
    eprintln!("Player start ijk: {} {} {}", i, j, k);

    // Get chunk indices for player position
    let stripe = ChunkPixelStripe {
        subpixel_origin: planisphere.get_pixel_corners_ijk(i, j)[0],
        chunk_size: chunks_center.chunk_size,
    };
    let (ichunk, jchunk) = stripe.get_chunk_indices(i, j, k, planisphere.clone());
    eprintln!("Player is in chunk indices: ({}, {})", ichunk, jchunk);

    let render_chunk_distance = 1;
    let mut not_rendered_chunks = 0;
    let mut rendered_chunks = 0;

    // Calculate chunk world size and half-size for proper positioning
    let chunk_world_size_x = chunks_center.chunk_size.0 as f32 * planisphere.mean_tile_size as f32;
    let chunk_world_size_z = chunks_center.chunk_size.1 as f32 * planisphere.mean_tile_size as f32;
    let half_chunk_x = chunk_world_size_x / 2.0;
    let half_chunk_z = chunk_world_size_z / 2.0;

    // Process chunks in a grid around the player's chunk
    for di in -render_chunk_distance..=render_chunk_distance {
        for dj in -render_chunk_distance..=render_chunk_distance {
            let chunk_i = ichunk as i32 + di;
            let chunk_j = jchunk as i32 + dj;

            eprintln!("\nProcessing chunk at indices: ({}, {})", chunk_i, chunk_j);

            // Get subpixels for this chunk
            let subpixels = chunks_center.get_chunk_subpixels(chunk_i, chunk_j, planisphere.clone());
            if subpixels.is_empty() {
                eprintln!("Skipping empty chunk at ({}, {})", chunk_i, chunk_j);
                not_rendered_chunks += 1;
                continue;
            }

            // Create chunk mesh
            let chunk = Chunk {
                size: chunks_center.chunk_size,
                chunk_position: (di, dj),
                subpixels,
                refinement_level: 0,
            };

            let mesh = chunk.mesh(&planisphere, (terrain_center.longitude, terrain_center.latitude));

            // Create material
            let material_handle = materials.add(StandardMaterial {
                base_color: Color::srgba(0.2, 0.8, 0.2, 1.0),
                metallic: 0.0,
                perceptual_roughness: 1.0,
                ..default()
            });

            // Calculate world position with proper offset to make chunks touch
            // The key is to position each chunk at its corner, not center
            let world_x = di as f32 * chunk_world_size_x;
            let world_z = dj as f32 * chunk_world_size_z;

            // Spawn entity
            let entity = commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material_handle),
                Transform::from_translation(Vec3::new(world_x, 0.0, world_z)),
            )).id();

            commands.entity(entity).insert(Wireframe);
            rendered_chunks += 1;
        }
    }

    eprintln!("Successfully rendered {} chunks, {} chunks skipped", rendered_chunks, not_rendered_chunks);
}



fn setup_chunks(
    
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    planisphere: Res<planisphere::Planisphere>,
    playerstart: Res<PlayerStart>,
    chunks_center: ResMut<ChunksCenter>,
    mut chunks_created: ResMut<ChunksCreated>,
    terrain_center: Res<TerrainCenter>,
    
)
{
    eprintln!("=== SETUP_CHUNKS SYSTEM STARTED ===");
    if chunks_created.created {
        eprintln!("Chunks already created, returning");
        return;
    }
    chunks_created.created = true;
    eprintln!("Starting chunk creation process");
    
    let (i,j,_k) = playerstart.start_ijk;
    eprintln!("Player start ijk: {} {} {}", i, j, _k);
    let (_lon,lat) = planisphere.subpixel_to_geo(i, j, _k);
    let ijk_corners = planisphere.get_pixel_corners_ijk(i, j);
    eprintln!("ijk corners : {:?} ",ijk_corners);
    let stripe = ChunkPixelStripe{subpixel_origin: ijk_corners[0], chunk_size: chunks_center.chunk_size};
    let (ichunk, jchunk) = stripe.get_chunk_indices(i, j, _k, planisphere.clone());
    eprintln!("Player is in chunk indices: ({}, {})", ichunk, jchunk);
    let render_chunk_distance = 1; // Reduce for debugging
    let mut not_rendered_chunks = 0;
    let mut rendered_chunks = 0;
    for di in -render_chunk_distance..(render_chunk_distance +1){
        for dj in -render_chunk_distance..(render_chunk_distance +1){
            let chunk_i = ichunk as i32 + di as i32;
            let chunk_j = jchunk as i32 + dj as i32;
            
            // Allow negative chunk indices - they represent valid neighboring chunks
            eprintln!("\n************Processing chunk at indices: ({}, {})************\n", chunk_i, chunk_j);
            
            //let start_vec = chunks_center.get_chunk_start_subpixel(chunk_i, chunk_j, planisphere.clone());
            //eprintln!("Chunk start subpixel ijk: {:?}", start_vec);
            let vec = chunks_center.get_chunk_subpixels(chunk_i, chunk_j, planisphere.clone());
            if vec.is_empty() {
                eprintln!("Skipping chunk at ({}, {}) - empty subpixel vector", di, dj);
                not_rendered_chunks += 1;
                continue;
            }
            eprintln!("Chunk at offset ({}, {}) has {} subpixels : {:?}", di, dj, vec.len(), vec);
            let chunk = Chunk {
                size: chunks_center.chunk_size,
                chunk_position: (di,dj),
                subpixels: vec,
                refinement_level:0,
            };
            let chunk_mesh = chunk.mesh(&planisphere.clone(),(terrain_center.longitude, terrain_center.latitude));
            eprintln!("Chunk mesh created with {} vertices and {} triangles", 
                chunk_mesh.count_vertices(), 
                chunk_mesh.indices().map(|i| i.len() / 3).unwrap_or(0));
            
            let terrain_material_handle = materials.add(StandardMaterial {
                base_color: Color::srgba(0.2, 0.8, 0.2, 1.0), // Darker green to better see white wireframe
                metallic: 0.0,
                perceptual_roughness: 1.0,
                cull_mode: None,
                alpha_mode: AlphaMode::Opaque,
                unlit: false, // Enable lighting for better depth perception
                ..default()
            });
            
            let terrain_mesh_handle = meshes.add(chunk_mesh);
            let terrain_entity_id = commands.spawn((
                Mesh3d(terrain_mesh_handle.clone()),
                MeshMaterial3d(terrain_material_handle.clone()),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            )).id();
            
            // Add wireframe component after entity creation
            commands.entity(terrain_entity_id).insert(Wireframe);
        
            eprintln!("**********Created chunk entity {:?} at position ({}, {})**********\n", terrain_entity_id, di, dj);
            rendered_chunks += 1;

        }
    }
    eprintln!("Successfully rendered {} chunks, {} chunks skipped", rendered_chunks, not_rendered_chunks);
    


    
    //panic!("metal");
    // === MATERIAL SETUP FOR TERRAIN TEXTURES ===
    // Configure the standard material for terrain rendering

        // Spawn single terrain entity





    //chunks_center.get_chunk_subpixels(1, 0, planisphere.clone());
    //chunks_center.get_chunk_subpixels(0, 1, planisphere.clone());
    //chunks_center.get_chunk_subpixels(1, 1, planisphere.clone());
    //chunks_center.get_chunk_subpixels(0, 1000, planisphere.clone());

    //panic!("this is the end...{:?} {:?}",ichunk,jchunk);
/*     let mut geo_corners: [(f64,f64);4] = [(0.,0.);4];
    for (ic, corner) in ijk_corners.iter().enumerate(){
        geo_corners[ic] = planisphere.subpixel_to_geo(corner.0, corner.1, corner.2);
    }
    eprintln!("geo corners : {:?}", geo_corners);
    for (ic,_corner) in geo_corners.iter().enumerate(){
        let d2 = (lon - _corner.0).powi(2) + (lat-_corner.1).powi(2);
        let d = ops::sqrt(d2 as f32);
        if d < min_d {center_corner = ijk_corners[ic]; min_d = d;}
        eprintln!("CORNERS {:?} distance {}",_corner,d);
    }
    eprintln!("chosen center : {:?}", center_corner); */

    
//for w in 0..64
//{    
    let k = _k; //+w;    
    let pixel_lon_divisions = planisphere.get_lon_subdivisons(lat);
    let nw = (k/planisphere.subpixel_divisions)  / chunks_center.chunk_size.0;
    let nh = (k%planisphere.subpixel_divisions)  / chunks_center.chunk_size.1;
    let chunk_start_subpixel_k = nw* chunks_center.chunk_size.0 * planisphere.subpixel_divisions + nh * chunks_center.chunk_size.1;

    eprintln!("*************************************************************************");
    eprintln!("Player position : {} {} {} *** subk {} **** nw {} nh {}", i,j,k,planisphere.subpixel_divisions,nw,nh);
    eprintln!("Player position : {} {}  -----  ",(k/planisphere.subpixel_divisions),  (k%planisphere.subpixel_divisions));
    eprintln!("k={}          ********** subpixel_chunk_height= {} subpixel_chunk_width= {} ",k,chunks_center.chunk_size.1,chunks_center.chunk_size.1);
    eprintln!("chunk starts at {}", chunk_start_subpixel_k);
    eprintln!("pixel {} {} has {} longitudinal subdivisions", i, j, pixel_lon_divisions);
    eprintln!("*************************************************************************");

    
//}
    
}




/// Setup function for physics world and game objects
/// This function is called once at startup to create the initial game world
/// 
/// Parameters:
/// - commands: Bevy's entity spawning system
/// - meshes: Storage for 3D shapes (meshes)
/// - materials: Storage for surface materials (colors, textures, etc.)
fn setup_physics(
    mut commands: Commands,                              // Entity spawning and management
    mut meshes: ResMut<Assets<Mesh>>,                   // 3D mesh asset storage
    mut materials: ResMut<Assets<StandardMaterial>>,    // Material asset storage
    mut terrain_center: ResMut<TerrainCenter>,          // Terrain center resource
    terrain_config: Res<TerrainConfig>,                 // Terrain configuration settings
    asset_server: Res<AssetServer>,                     // Asset server resource
    planisphere: Res<planisphere::Planisphere>,
    mut rendered_subpixels: ResMut<RenderedSubpixels>,  // Rendered subpixels resource
    mut triangle_mapping: ResMut<TriangleSubpixelMapping>, // Triangle to subpixel mapping
    mut asset_tracker: ResMut<TerrainAssetTracker>,     // Asset tracker for cleanup
    time: Res<Time>,                                    // Time resource for profiling
) {
    // Create a small planisphere for gnomonic projection terrain

    // Initialize terrain center resource to match initial terrain
    //terrain_center.longitude = 0.0;   // Greenwich meridian
    //terrain_center.latitude = 0.0;  // 45° North
    //terrain_center.max_subpixel_distance = terrain_config.recreation_threshold; // Sync with TerrainConfig
    terrain_center.last_recreation_time = -10.0; // Allow immediate recreation if needed
    
    // setup_object_templates is now handled by Startup systems

    create_terrain_gnomonic_rectangular(
        &mut commands, 
        &mut meshes, 
        &mut materials,
        &asset_server,            // Center latitude
        &planisphere,    
        &mut terrain_center,                    // Planisphere reference (mutable)
        Some(&mut asset_tracker),               // Pass asset tracker for cleanup
        &time                                   // Pass time resource for profiling
    );

    // Create the terrain center beacon at the gnomonic projection center
    //create_terrain_center_beacon(
    //    &mut commands,
    //    &mut meshes,
    //    &mut materials,
    //);
    
    // Create the agents (autonomous entities that move around the terrain)
    // This spawns 5 agents in a grid pattern on the terrain
    //create_agents(&mut commands, &mut meshes, &mut materials, 1, &planisphere, terrain_center.center_lon, terrain_center.center_lat);
    
    // Create the player (red capsule controlled by keyboard)


    //spawn_dynamic_object_with_raycast_with_ui(&mut commands, &mut meshes, &mut materials, Some(&planisphere), &terrain_center, object_definition);

    // Create collectible items in the world
    // Currently creates a single "Magic Stone" that agents can pick up
    //create_items(&mut commands, &mut meshes, &mut materials, &planisphere, terrain_center.center_lon, terrain_center.center_lat, &terrain_config, &triangle_mapping);
    
    // Create landscape elements (decorative objects like stones, trees, rocks)
    //create_landscape_elements(&mut commands, &mut meshes, &mut materials, &planisphere, terrain_center.center_lon, terrain_center.center_lat, &terrain_config, &triangle_mapping, Some(&mut asset_tracker));
    
    // Create debug beacons to visualize tile structure and player position
    //create_debug_beacons(&mut commands, &mut meshes, &mut materials, &planisphere, terrain_center.center_lon, terrain_center.center_lat, &terrain_config);
    
    // Create the player tile beacon that follows the player's current tile
    //create_player_tile_beacon(&mut commands, &mut meshes, &mut materials);
}

// Additional explanation for beginners:
//
// BEVY CONCEPTS:
// - App: The main game engine instance that manages everything
// - Plugins: Pre-built modules that add functionality (graphics, physics, etc.)
// - Systems: Functions that run regularly to update the game state
// - Components: Data attached to entities (like Transform, Velocity, etc.)
// - Entities: Game objects that can have components attached
// - Resources: Global data shared across systems (like Time, Assets, etc.)
//
// SYSTEM SCHEDULING:
// - Startup systems run once when the game starts
// - Update systems run every frame (usually 60 times per second)
// - Systems can read and modify components and resources
//
// PHYSICS INTEGRATION:
// - RapierPhysicsPlugin adds realistic physics simulation
// - Colliders define the shape of objects for collision detection
// - RigidBodies define how objects respond to forces (Dynamic, Fixed, etc.)
// - Velocity components control how fast objects move
//
// COORDINATE SYSTEM:
// - X axis: left (-) to right (+)
// - Y axis: down (-) to up (+) 
// - Z axis: into screen (-) to out of screen (+)
// - This follows Bevy's right-handed coordinate system