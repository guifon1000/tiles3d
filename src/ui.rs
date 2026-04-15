use bevy::prelude::*;
use crate::planisphere;
use crate::player::Player;
use crate::terrain::TerrainCenter;
use crate::game_object::EntitySubpixelPosition;

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
    player_query: Query<(&Transform, &EntitySubpixelPosition), With<Player>>,
    _terrain_center: Res<TerrainCenter>,
    mut text_query: Query<&mut Text, With<CoordinateDisplay>>,
    planisphere: Res<planisphere::Planisphere>,
) {
    let Ok((transform, ijkpos)) = player_query.single() else { return; };
    let Ok(mut text) = text_query.single_mut() else { return; };

    let world_pos = transform.translation;
    let geo_coords = planisphere.subpixel_to_geo(ijkpos.subpixel.0, ijkpos.subpixel.1, ijkpos.subpixel.2);
    let subpixel: (usize, usize, usize) = ijkpos.subpixel;

    let coordinate_text = format!(
        "Player Position:\n\
        World: ({:.2}, {:.2}, {:.2})\n\
        Geo: ({:.6}°, {:.6}°)\n\
        Tile: ({}, {}, {})",
        world_pos.x, world_pos.y, world_pos.z,
        geo_coords.0, geo_coords.1,
        subpixel.0, subpixel.1, subpixel.2
    );

    **text = coordinate_text;
}