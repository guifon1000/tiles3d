use bevy::prelude::*;
use crate::player::{PlayerSubpixelPosition, EntitySubpixelPosition, Player};
use crate::terrain::TerrainCenter;

/// Component to mark the coordinate display text entity
#[derive(Component)]
pub struct CoordinateDisplay;

/// Setup the UI system with a coordinate display panel
/// This creates a semi-transparent panel in the top-left corner showing player position
/// in all three coordinate systems: World (X,Y,Z), Geographic (Lon,Lat), and Subpixel (I,J,K)
pub fn setup_ui(mut commands: Commands) {
    // Create the root UI node that covers the entire screen
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::FlexStart,
            ..default()
        },
        Name::new("Root UI Node"),
    )).with_children(|parent| {
        // Create a coordinate display panel in the top-left corner
        parent.spawn((
            Node {
                // Panel positioning and size
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                width: Val::Px(350.0),
                height: Val::Px(120.0),
                
                // Panel layout
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                padding: UiRect::all(Val::Px(10.0)),
                
                ..default()
            },
            // Semi-transparent dark background
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            Name::new("Coordinate Panel"),
        )).with_children(|panel| {
            // Coordinate display text
            panel.spawn((
                Text::new("Player Position:\nWorld: (0.0, 0.0, 0.0)\nGeo: (0.000°, 0.000°)\nTile: (0, 0, 0)"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                CoordinateDisplay, // Mark this entity for updates
                Name::new("Coordinate Text"),
            ));
        });
    });
}

/// Update system that refreshes the coordinate display with current player position
/// This system runs every frame and updates the text to show:
/// - World coordinates (X, Y, Z) in 3D game space
/// - Geographic coordinates (Longitude, Latitude) in degrees
/// - Subpixel coordinates (I, J, K) in the tile grid
/// UPDATED: Display coordinates using new shared positioning component
pub fn update_coordinate_display(
    // Try to get player position from new shared component first, fallback to old resource
    player_query: Query<&EntitySubpixelPosition, With<Player>>,
    player_subpixel_resource: Res<PlayerSubpixelPosition>, // Fallback for compatibility
    _terrain_center: Res<TerrainCenter>,
    mut text_query: Query<&mut Text, With<CoordinateDisplay>>,
) {
    for mut text in text_query.iter_mut() {
        // Get coordinates from new shared component if available, otherwise use old resource
        let (world_pos, geo_coords, subpixel, source) = if let Ok(player_position) = player_query.single() {
            // Use new shared component
            (player_position.world_pos, player_position.geo_coords, player_position.subpixel, "RAYCAST")
        } else {
            // Fallback to old resource for compatibility
            println!("UI: Player component not found, using fallback resource");
            (player_subpixel_resource.world_pos, player_subpixel_resource.geo_coords, player_subpixel_resource.subpixel, "TILES")
        };
        
        let (lon, lat) = geo_coords;
        let (i, j, k) = subpixel;
        
        // Format the coordinate information into a readable display
        let coordinate_text = format!(
            "Player Position ({}):\n\
            World: ({:.2}, {:.2}, {:.2})\n\
            Geo: ({:.6}°, {:.6}°)\n\
            Tile: ({}, {}, {})",
            source,                                   // Show which method was used
            world_pos.x, world_pos.y, world_pos.z,  // World coordinates with 2 decimal places
            lon, lat,                                 // Geographic coordinates with 6 decimal places
            i, j, k                                   // Subpixel coordinates as integers
        );
        
        // Update the text content
        **text = coordinate_text;
    }
}