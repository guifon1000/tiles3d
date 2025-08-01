# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**tiles3d** is a Rust/Bevy 3D terrain exploration simulation featuring infinite terrain generation, subpixel coordinate precision, and physics-based autonomous agents. The project demonstrates advanced techniques in coordinate system conversion, asset lifecycle management, and real-time terrain recreation.

## Commands

### Building & Running
```bash
cargo run --release              # Run optimized build
cargo check                      # Quick syntax validation
timeout 10s cargo run 2>&1       # Test run with timeout
```

### Development Commands
```bash
# Monitor texture selection debug output
cargo run 2>&1 | grep "DEBUG: red" | head -20

# Track terrain recreation events
cargo run 2>&1 | grep "Recreating terrain"

# Generate texture atlas from individual images
cd assets/textures && python atlas_creator.py
```

### Testing & Debugging
- No formal test suite - use timeout runs for behavior verification
- Wireframe mode available via `RapierDebugRenderPlugin` (currently commented out)
- Console debugging through `eprintln!` statements throughout codebase

## Architecture

### Core System Relationships

The architecture centers around three interconnected coordinate systems:
1. **Geographic Coordinates** (lat/lon degrees) - Real-world positioning
2. **World Coordinates** (Vec3 units) - 3D game space with physics
3. **Subpixel Coordinates** (i,j,k indices) - High-resolution tile grid

**Data Flow:**
```
sphere_texture.png → planisphere.rs (coordinate conversion + RGBA extraction) 
                  → terrain.rs (mesh generation + texture selection)
                  → player.rs (movement + recreation triggers)
```

### Key Systems

**Terrain Generation** (`terrain.rs`):
- `create_terrain_gnomonic_rectangular()` - Primary mesh generation using gnomonic projection
- `select_texture_from_rgba()` - Maps RGBA values to texture atlas indices (currently only RED channel)
- Material setup with enhanced visual properties (metallic: 0.1, roughness: 0.8)

**Asset Management** (`main.rs` + `terrain.rs`):
- `TerrainAssetTracker` resource prevents memory leaks during terrain recreation
- Comprehensive cleanup of mesh/material handles before regeneration
- Proper asset removal from Bevy's asset system

**Coordinate Conversion** (`planisphere.rs`):
- `Planisphere::load_from_file()` - Loads sphere_texture.png and extracts RGBA channels
- `get_rgba_at_subpixel()` - Returns color values for terrain position
- Geographic ↔ World coordinate transformations using gnomonic projection

**Recreation System** (`player.rs`):
- `terrain_recreation_system()` - Triggers when player moves >20 tiles from center
- `coordinate_sync_system()` - Repositions entities after terrain regeneration
- 1-second cooldown prevents rapid recreations

### File Structure

**Core Systems:**
- `main.rs` - Entry point, resource initialization, system registration
- `planisphere.rs` - Geographic coordinate system, RGBA processing, projection math
- `terrain.rs` - Mesh generation, texture selection, material properties
- `player.rs` - Movement controls, terrain recreation triggers, positioning

**Supporting Systems:**
- `camera.rs` - Third-person camera with mouse look and zoom
- `agent.rs` - AI entities with physics-based movement and obstacle detection
- `game_object.rs` - Unified object spawning and entity management
- `ui.rs` - Real-time coordinate display across all three systems
- `landscape.rs` - Decorative elements (trees, rocks, collectible items)
- `beacons.rs` - Debug visualization markers

### Configuration Constants

**Terrain System** (`main.rs` TerrainConfig):
```rust
terrain_radius: 80,              // Terrain size in tiles
recreation_threshold: 20,        // Distance before recreation (1/4 radius)
recreation_cooldown: 1.0,        // Minimum seconds between recreations
```

**Player Movement** (`player.rs`):
```rust
move_speed: 15.0,               // Movement speed
mouse_sensitivity: 0.002,       // Look sensitivity
jump_force: 8.0,               // Upward jump velocity
```

**Subpixel System** (`planisphere.rs`):
```rust
width: 3000, height: 1500,      // Base pixel dimensions
subpixel_divisions: 8,          // Subdivisions per pixel (8×8 = 64 per pixel)
```

## Texture System

### Current Implementation
- 16×16 texture atlas (256 slots available, 19 textures used)
- Selection driven by RED channel only (values 0-9)
- Available textures: deepwater, dirt, drygrass, grass, stone, lava, moss, sand, snow, water, etc.
- Simple threshold-based mapping in `select_texture_from_rgba()`

### Data Source
- `assets/maps/sphere_texture.png` - Dual-purpose map providing elevation AND texture data
- RGBA channels extracted per subpixel position
- `assets/textures/atlas_creator.py` - Generates enhanced contrast atlas from individual images

### Material Properties
Materials configured for enhanced visual appeal:
```rust
base_color: Color::srgb(1.0, 1.0, 1.0),  // Neutral for texture display
metallic: 0.1,                            // Minimal shine
roughness: 0.8,                           // Natural matte surface
```

## Performance & Memory Management

### Asset Lifecycle
**CRITICAL**: The project recently fixed major memory leaks in terrain recreation:
- `TerrainAssetTracker` tracks all mesh/material handles
- `cleanup_assets()` removes old handles before creating new terrain
- Prevents progressive slowdown during gameplay

### Coordinate Systems
**Subpixel Precision**: 3000×1500 pixels × 8×8 subdivisions = 24+ million addressable positions
**Terrain Coverage**: ~6400 subpixels rendered simultaneously (80×80 tiles)
**Physics**: Trimesh colliders with triangle-to-subpixel mapping for ground detection

## Controls
- **WASD**: Movement
- **Mouse**: Look around
- **Mouse Wheel**: Camera zoom
- **Right-Click + Drag**: Camera rotation
- **Space**: Jump (with cooldown)

## Known Issues & Debugging

### Current Limitations
1. **Texture Variety**: Only 10 textures used from 256 available (RED channel only)
2. **Debug Output**: `eprintln!` statements clutter console during texture selection
3. **Simple Selection**: Threshold-based texture mapping could be more sophisticated

### Debug Features
- Real-time coordinate display in UI
- Console output for subpixel positions and terrain recreation events
- Asset cleanup messages: "ASSET CLEANUP: Removed X meshes and Y materials"
- Coordinate sync messages during recreation

### Performance Notes
- Asset management prevents memory leaks ✅
- Terrain recreation properly synchronized ✅
- Physics colliders cleaned up during regeneration ✅

## Development Notes

### When Working on Textures
1. Current selection in `select_texture_from_rgba()` only uses RED channel (lines ~15-30)
2. Expand to use GREEN/BLUE/ALPHA channels for more variety
3. Atlas creator enhances contrast/saturation for vivid appearance
4. Texture indices: 0=deepwater, 1=dirt, 2=drygrass, 3=grass, 4=stone, etc.

### When Working on Terrain
1. Gnomonic projection centers around player position
2. Recreation triggered by Manhattan distance calculation
3. Triangle mapping cleared during recreation to prevent physics issues
4. Mesh generation uses height data from sphere_texture.png

### When Working on Coordinates
1. All systems use shared coordinate conversion functions
2. Player position tracked across terrain recreations
3. Subpixel system provides smooth movement between discrete positions
4. Geographic coordinates enable real-world mapping

## Quick Start for New Sessions
1. Run `cargo check` to verify compilation
2. Brief test run: `timeout 10s cargo run 2>&1` to see current behavior
3. Check recent git commits for context on recent changes
4. Use TodoWrite tool for complex multi-step tasks
5. Focus areas typically involve texture system, terrain generation, or coordinate handling