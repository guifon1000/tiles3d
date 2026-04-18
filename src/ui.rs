use bevy::prelude::*;
use crate::planisphere::{self, DistanceMethod};
use crate::player::Player;
use crate::game_object::EntitySubpixelPosition;
use crate::terrain::TerrainCenter;

// ── Marker components ────────────────────────────────────────────────────────

#[derive(Component)]
pub struct CoordinateDisplay;

/// Attached to each method button so the handler knows which method it represents.
#[derive(Component, Clone, Copy)]
pub struct MethodButton(pub DistanceMethod);

// ── Setup ────────────────────────────────────────────────────────────────────

pub fn setup_ui(mut commands: Commands) {
    // --- coordinate info panel (top-left) ---
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
    )).with_children(|panel| {
        panel.spawn((
            Text::new(""),
            TextFont { font_size: 14.0, ..default() },
            TextColor(Color::WHITE),
            CoordinateDisplay,
        ));
    });

    // --- distance method selector (top-left, below the info panel) ---
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(110.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
    )).with_children(|panel| {
        panel.spawn((
            Text::new("Distance method:"),
            TextFont { font_size: 12.0, ..default() },
            TextColor(Color::srgb(0.7, 0.7, 0.7)),
        ));
        for (label, method) in [
            ("Chebyshev (square)",  DistanceMethod::Chebyshev),
            ("Euclidean (circle)",  DistanceMethod::Euclidean),
            ("Manhattan (diamond)", DistanceMethod::Manhattan),
        ] {
            panel.spawn((
                Button,
                Node { padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)), ..default() },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.9)),
                MethodButton(method),
            )).with_children(|btn| {
                btn.spawn((
                    Text::new(label),
                    TextFont { font_size: 13.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });
        }
    });
}

// ── Systems ───────────────────────────────────────────────────────────────────

/// Handles clicks on the distance-method buttons and triggers terrain recreation.
pub fn handle_method_buttons(
    interaction_query: Query<(&Interaction, &MethodButton), Changed<Interaction>>,
    mut terrain_center: ResMut<TerrainCenter>,
) {
    for (interaction, btn) in &interaction_query {
        if *interaction == Interaction::Pressed {
            if terrain_center.distance_method != btn.0 {
                terrain_center.distance_method = btn.0;
                terrain_center.force_recreation = true;
            }
        }
    }
}

/// Colours buttons to show which method is active and highlights hovered ones.
pub fn update_method_button_colors(
    terrain_center: Res<TerrainCenter>,
    mut button_query: Query<(&Interaction, &MethodButton, &mut BackgroundColor)>,
) {
    for (interaction, btn, mut bg) in &mut button_query {
        *bg = if btn.0 == terrain_center.distance_method {
            // Active method — bright highlight
            BackgroundColor(Color::srgb(0.1, 0.5, 0.9))
        } else if *interaction == Interaction::Hovered {
            BackgroundColor(Color::srgba(0.4, 0.4, 0.4, 0.9))
        } else {
            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.9))
        };
    }
}

/// Updates the coordinate text with current player position.
pub fn update_coordinate_display(
    player_query: Query<(&Transform, &EntitySubpixelPosition), With<Player>>,
    mut text_query: Query<&mut Text, With<CoordinateDisplay>>,
    planisphere: Res<planisphere::Planisphere>,
    terrain_center: Res<TerrainCenter>,
) {
    let Ok((transform, ijkpos)) = player_query.single() else { return; };
    let Ok(mut text) = text_query.single_mut() else { return; };

    let (lon, lat) = planisphere.subpixel_to_geo(ijkpos.subpixel.0, ijkpos.subpixel.1, ijkpos.subpixel.2);
    let (i, j, k) = ijkpos.subpixel;
    let Vec3 { x, y, z } = transform.translation;

    **text = format!(
        "World: ({x:.2}, {y:.2}, {z:.2})\nGeo: ({lon:.6}°, {lat:.6}°)\nTile: ({i}, {j}, {k})"
    );
}
