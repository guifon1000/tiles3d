// Import statements - bring in code from other modules and crates
use bevy::prelude::*;                               // Bevy game engine core functionality
use bevy::input::mouse::{MouseMotion, MouseButton, MouseScrollUnit, MouseWheel}; // Mouse input handling
use bevy::input::keyboard::KeyCode; // Keyboard input handling
use bevy::input::ButtonInput; // Button input handling
use crate::player::Player;                         // Import Player component

// Removed unused CameraController component

/// ThirdPersonCamera Component - Marks a camera as third person following the player
#[derive(Component)]
pub struct ThirdPersonCamera {
    pub distance: f32,       // Distance behind the player
    pub height: f32,         // Height above the player
    pub follow_speed: f32,   // How fast the camera follows the player
    pub min_distance: f32,   // Minimum zoom distance
    pub max_distance: f32,   // Maximum zoom distance
    pub zoom_speed: f32,     // Speed of zoom changes
    pub min_height: f32,     // Minimum camera height above player
    pub max_height: f32,     // Maximum camera height above player
    pub height_speed: f32,   // Speed of height changes
}

/// CameraLight Component - Marks a light that follows the camera
#[derive(Component)]
pub struct CameraLight;

// Removed unused setup_camera function

/// Setup the third person camera that follows the player
pub fn setup_third_person_camera(mut commands: Commands) {
    // Spawn the third person camera entity
    commands.spawn((
        Camera3d::default(),  // This makes it a 3D camera
        
        // Set initial camera position (will be updated to follow player)
        Transform::from_xyz(0.0, 5.0, 8.0)  // Start position: behind and above player
            .looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y), // Look at player height
        
        // Add our custom third person camera controller
        ThirdPersonCamera {
            distance: 20.0,         // Distance behind the player
            height: 14.0,           // Height above the player
            follow_speed: 5.0,      // How fast the camera follows
            min_distance: 3.0,      // Minimum zoom distance
            max_distance: 50.0,     // Maximum zoom distance
            zoom_speed: 2.0,        // Speed of zoom changes
            min_height: 2.0,        // Minimum height above player
            max_height: 50.0,       // Maximum height above player
            height_speed: 15.0,     // Speed of height changes
        },
    ));
    
    // Add a directional light that follows the camera
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.95, 0.8),  // Warm white light
            illuminance: 15000.0,                  // Brightness (lux)
            shadows_enabled: true,                 // Enable shadows
            ..default()
        },
        Transform::from_xyz(0.0, 5.0, 8.0)       // Start at camera position
            .looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y), // Point in same direction
        CameraLight,  // Mark it as a camera light
    ));
}

// Removed unused camera_zoom and camera_rotation functions

/// Update third person camera to follow the player
/// This function runs every frame and makes the camera follow the player smoothly
pub fn update_third_person_camera(
    time: Res<Time>,
    player_query: Query<(&Transform, &Player), Without<ThirdPersonCamera>>,
    mut camera_query: Query<(&mut Transform, &ThirdPersonCamera), With<ThirdPersonCamera>>,
) {
    // Get the player's transform and player component
    if let Ok((player_transform, player)) = player_query.single() {
        // Get the camera's transform and controller
        if let Ok((mut camera_transform, controller)) = camera_query.single_mut() {
            let delta_time = time.delta_secs();
            
            // Calculate desired camera position based on player position and facing direction
            let player_pos = player_transform.translation;
            
            // Use the player's facing angle for camera positioning
            let facing_angle = player.facing_angle;
            
            // Calculate camera position behind and above the player
            let camera_offset = Vec3::new(
                facing_angle.sin() * controller.distance,  // Behind player in X
                controller.height,                          // Above player
                facing_angle.cos() * controller.distance,  // Behind player in Z
            );
            
            let desired_pos = player_pos + camera_offset;
            
            // Smoothly interpolate camera position
            let follow_speed = controller.follow_speed;
            camera_transform.translation = camera_transform.translation
                .lerp(desired_pos, follow_speed * delta_time);
            
            // Look at the player (slightly above their position)
            let look_target = player_pos + Vec3::new(0.0, 2.0, 0.0);
            camera_transform.look_at(look_target, Vec3::Y);
        }
    }
}

/// Handle third person camera rotation using mouse drag
/// This function runs every frame and handles mouse button presses and mouse movement
/// NOTE: This system is currently disabled since the player uses mouse look directly
pub fn third_person_camera_rotation(
    _mouse_button: Res<ButtonInput<MouseButton>>,
    mut _mouse_motion: EventReader<MouseMotion>,
    mut _camera_query: Query<&mut ThirdPersonCamera>,
) {
    // Camera rotation is disabled because the player uses mouse look directly
    // The camera follows the player's orientation automatically
}

/// Handle mouse wheel zoom for the third person camera
/// This function adjusts the camera distance based on mouse scroll input
pub fn handle_camera_zoom(
    time: Res<Time>,
    mut scroll_events: EventReader<MouseWheel>,
    mut camera_query: Query<&mut ThirdPersonCamera>,
) {
    // Get the camera controller
    if let Ok(mut camera) = camera_query.single_mut() {
        let delta_time = time.delta_secs();
        
        // Process mouse wheel scroll events
        for scroll_event in scroll_events.read() {
            let scroll_delta = match scroll_event.unit {
                MouseScrollUnit::Line => scroll_event.y,        // Standard mouse wheel (lines)
                MouseScrollUnit::Pixel => scroll_event.y * 0.1, // Touchpad or high-precision scroll (pixels)
            };
            
            // Calculate zoom change (negative scroll = zoom in, positive = zoom out)
            let zoom_change = -scroll_delta * camera.zoom_speed * delta_time * 10.0; // Scale factor for responsiveness
            
            // Update distance and clamp to min/max bounds
            camera.distance = (camera.distance + zoom_change).clamp(camera.min_distance, camera.max_distance);
            
            // Optional: Print zoom level for debugging
            if scroll_delta != 0.0 {
                println!("Camera zoom: {:.1} (range: {:.1} - {:.1})", camera.distance, camera.min_distance, camera.max_distance);
            }
        }
    }
}

/// Handle camera height control using up/down arrow keys
/// This function adjusts the camera height while keeping it focused on the player
pub fn handle_camera_height(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut ThirdPersonCamera>,
) {
    // Get the camera controller
    if let Ok(mut camera) = camera_query.single_mut() {
        let delta_time = time.delta_secs();
        let mut height_change = 0.0;
        
        // Check for up/down arrow key presses
        if keyboard_input.pressed(KeyCode::ArrowUp) {
            height_change += camera.height_speed * delta_time;
        }
        if keyboard_input.pressed(KeyCode::ArrowDown) {
            height_change -= camera.height_speed * delta_time;
        }
        
        // Apply height change and clamp to min/max bounds
        if height_change != 0.0 {
            camera.height = (camera.height + height_change).clamp(camera.min_height, camera.max_height);
            
            // Optional: Print height level for debugging
            println!("Camera height: {:.1} (range: {:.1} - {:.1})", camera.height, camera.min_height, camera.max_height);
        }
    }
}

/// Update camera light to follow the camera position and direction
/// This function runs every frame and keeps the light synchronized with the camera
pub fn update_camera_light(
    camera_query: Query<&Transform, (With<ThirdPersonCamera>, Without<CameraLight>)>,
    mut light_query: Query<&mut Transform, (With<CameraLight>, Without<ThirdPersonCamera>)>,
) {
    // Get the camera's transform
    if let Ok(camera_transform) = camera_query.single() {
        // Get the light's transform
        if let Ok(mut light_transform) = light_query.single_mut() {
            // Position the light slightly behind the camera for better lighting
            let light_offset = camera_transform.back() * 2.0; // 2 units behind camera
            light_transform.translation = camera_transform.translation + light_offset;
            
            // Make the light point in the same direction as the camera
            light_transform.rotation = camera_transform.rotation;
        }
    }
}
