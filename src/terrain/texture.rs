/// Generate a deterministic random value (0.0 to 1.0) based on (i,j,k) coordinates
/// This ensures consistent landscape element placement across terrain regenerations
/// 
use bevy::math::ops::sqrt;
pub fn deterministic_random(i: usize, j: usize, k: usize) -> f64 {
    // Improved hash function with better mixing to avoid patterns
    // Based on xxHash and other high-quality hash functions

    // Convert coordinates to u64 for better mixing
    let mut hash = (i as u64).wrapping_mul(0x9E3779B185EBCA87); // Large prime
    hash ^= (j as u64).wrapping_mul(0xC2B2AE3D27D4EB4F);      // Another large prime
    hash ^= (k as u64).wrapping_mul(0x165667B19E3779F9);      // Another large prime

    // Additional mixing steps to break patterns
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x3C79AC492BA7B653);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0x1C69B3F74AC4AE35);
    hash ^= hash >> 27;

    // Convert to 0.0-1.0 range
    (hash as f64) / (u64::MAX as f64)
}

/// Determine landscape element type based on RGBA channel values and random probability
///
/// # Parameters
/// * `red` - Red channel value (0.0 to 1.0)
/// * `green` - Green channel value (0.0 to 1.0)
/// * `blue` - Blue channel value (0.0 to 1.0)
/// * `alpha` - Alpha channel value (0.0 to 1.0)
/// * `i`, `j`, `k` - Subpixel coordinates for deterministic randomness
///
/// # Returns
/// Option containing (element_type, y_offset) or None if no landscape element
pub fn determine_landscape_element_from_rgba(_red: f64, _green: f64, _blue: f64, alpha: f64, i: usize, j: usize, k: usize) -> Option<(String, f32)> {
    // Get deterministic random value for this position
    let random_value = deterministic_random(i, j, k);

    // Use alpha channel to determine potential landscape element type
    let element_type = if alpha >= 0.8 && alpha <= 1.0 {
        // High alpha values = potential trees
        Some(("tree".to_string(), 0.6))
    } else if alpha >= 0.6 && alpha < 0.8 {
        // Medium-high alpha values = potential rocks
        Some(("rock".to_string(), 0.3))
    } else if alpha >= 0.3 && alpha < 0.6 {
        // Medium alpha values = potential stones
        Some(("stone".to_string(), 0.15))
    } else {
        // Low alpha values = no landscape element
        None
    };

    // If we have a potential element, use random probability to decide if it actually appears
    if let Some((elem_type, y_offset)) = element_type {
        // Different spawn probabilities for different elements (very sparse distribution)
        let spawn_probability = match elem_type.as_str() {
            "tree" => 0.003,  // 0.3% chance for trees
            "rock" => 0.006,  // 0.6% chance for rocks
            "stone" => 0.010, // 1.0% chance for stones
            _ => 0.003,
        };

        if random_value < spawn_probability {
            Some((elem_type, y_offset))
        } else {
            None
        }
    } else {
        None
    }
}

/// Select texture atlas tile index based on RGBA color values from geographic map data
///
/// This is the core texture selection function that determines which texture from the
/// 16x16 texture atlas (256 total textures) should be applied to each terrain subpixel.
/// The selection is based on RGBA color data extracted from sphere_texture.png.
///
/// # How It Works:
/// 1. Each pixel in sphere_texture.png represents a geographic location
/// 2. The RGBA values of that pixel encode terrain type information
/// 3. This function converts those RGBA values into a texture atlas index (0-255)
/// 4. The index determines which 16x16 texture tile from texture_atlas.png is used
///
/// # Current Implementation:
/// - Uses only the RED channel for texture selection (ignoring green, blue, alpha)
/// - Maps red values 0.0-1.0 to texture indices 0-9 (only 10 of 256 available textures)
/// - Uses simple threshold-based selection with 0.1 increments
///
/// # Available Textures (in texture_atlas.png):
/// The atlas contains these terrain textures:
/// - 0-9: deepwater, dirt, drygrass, eastgrass, grass, greenstone, ice, lava, lavastone, moss
/// - 10+: mossystone, northgrass, pavedstone, rawstone, sand, snow, southgrass, water, westgrass
///
/// # Parameters
/// * `red` - Red channel value (0.0 to 1.0) from corresponding map pixel
/// * `_green` - Green channel value (0.0 to 1.0) - currently unused but available
/// * `_blue` - Blue channel value (0.0 to 1.0) - currently unused but available
/// * `_alpha` - Alpha channel value (0.0 to 1.0) - currently unused but available
///
/// # Returns
/// Texture atlas tile index (0 to 255 for a 16x16 atlas, currently returns 0-9)
pub fn select_texture_from_rgba(red: f64, green: f64, blue: f64, alpha: f64) -> usize {
    let alti = crate::planisphere::sampling::rgba_to_alti(red, green, blue, alpha);

    let texture_index = if alti < 0.1 {
        0  // Very dark alpha -> texture 0 (e.g., deep water)
    } else if alti < 0.2 {
        1  // Dark alpha -> texture 1 (e.g., dirt)
    } else if alti < 0.3 {
        2  // Low alpha -> texture 2 (e.g., dry grass)
    } else if alti < 0.4 {
        3  // Medium-low alpha -> texture 3 (e.g., regular grass)
    } else if alti < 0.5 {
        4  // Medium alpha -> texture 4 (e.g., green stone)
    } else if alti < 0.6 {
        5  // Medium-high alpha -> texture 5 (e.g., moss)
    } else if alti < 0.7 {
        6  // High alpha -> texture 6 (e.g., sand)
    } else if alti < 0.8 {
        7  // Higher alpha -> texture 7 (e.g., stone)
    } else if alti < 0.9 {
        8  // Very high alpha -> texture 8 (e.g., snow)
    } else {
        9  // Maximum alpha -> texture 9 (e.g., lava)
    };

    texture_index
}
