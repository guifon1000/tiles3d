# CLAUDE.md - tiles3d Project Context

## 🎯 Project Overview
**tiles3d** is a Rust/Bevy 3D terrain exploration simulation with:
- Infinite terrain generation using gnomonic projection
- Geographic coordinate system with subpixel precision
- Advanced texture system driven by RGBA map data
- Autonomous agents with physics-based movement
- Real-time terrain recreation based on player movement

## 🏗️ Architecture

### Core Files
```
src/
├── main.rs         # Entry point, resource initialization, system registration
├── terrain.rs      # Terrain generation, texture selection, material setup
├── planisphere.rs  # Geographic coordinates, RGBA data processing
├── player.rs       # Player movement, terrain recreation triggers
├── agent.rs        # AI agents with physics and sensors
├── camera.rs       # Third-person camera with mouse/zoom controls
└── ui.rs          # Coordinate display and UI elements

assets/
├── maps/sphere_texture.png     # Geographic map (elevation + texture data)
└── textures/
    ├── texture_atlas.png       # 256×256 atlas (19 terrain textures)
    ├── atlas_creator.py        # Atlas generation tool
    └── img/                    # Individual terrain textures
```

### Key Systems
1. **Terrain Generation**: `create_terrain_gnomonic_rectangular()` - Creates terrain mesh with textures
2. **Asset Management**: `TerrainAssetTracker` - Tracks and cleans up asset handles
3. **Texture Selection**: `select_texture_from_rgba()` - Maps RGBA values to texture indices
4. **Coordinate Conversion**: Geographic ↔ World ↔ Subpixel coordinate systems
5. **Recreation System**: `terrain_recreation_system()` + `coordinate_sync_system()` - Regenerates terrain with proper cleanup
6. **Performance Optimization**: Asset cleanup prevents memory leaks and progressive slowdown

## 🎨 Texture System Architecture

### Data Flow
```
sphere_texture.png → planisphere.rs (RGBA extraction) → terrain.rs (texture selection) → GPU
```

### Key Functions
- `Planisphere::load_from_file()` - Loads image, extracts RGBA channels
- `get_rgba_at_subpixel()` - Returns RGBA values for terrain position  
- `select_texture_from_rgba()` - Converts RGBA to texture atlas index (0-9 currently)
- Material setup with flashy properties (metallic=0.5, emissive glow, brightness=1.75)

### Current Limitations
- Only uses RED channel (0-9 textures of 256 available)
- Simple threshold-based selection
- Debug output clutters console

## 🚀 Common Tasks

### Building & Running
```bash
cargo run --release          # Run optimized build
cargo check                  # Quick syntax check
timeout 10s cargo run 2>&1   # Run with timeout for testing
```

### Key Commands for Development
```bash
# View texture debug output
cargo run 2>&1 | grep "DEBUG: red" | head -20

# Check terrain recreation
cargo run 2>&1 | grep "Recreating terrain"

# Generate new texture atlas
cd assets/textures && python atlas_creator.py
```

### Controls
- WASD: Movement
- Mouse: Look around  
- Mouse wheel: Zoom
- Right-click drag: Camera rotation
- Space: Jump

## 🔧 Configuration

### Key Constants (player.rs)
```rust
const TERRAIN_RADIUS: usize = 20;        # Terrain size
const RECREATION_THRESHOLD: usize = 5;   # Recreation distance (tiles)
const RECREATION_COOLDOWN: f32 = 2.0;    # Recreation delay (seconds)
```

### Texture Material Settings (terrain.rs ~line 1660)
```rust
base_color: Color::srgb(1.75, 1.75, 1.75),  # Brightness multiplier
metallic: 0.5,                               # Metallic shine
emissive: Color::srgb(0.1, 0.1, 0.1),       # Glow effect
```

## ✅ Recent Fixes & Improvements

### 🔥 Major Performance Fix (Asset Memory Leaks)
**FIXED**: Asset handle accumulation causing progressive slowdown
- **Root Cause**: Every terrain recreation created new mesh/material handles but never cleaned up old ones
- **Solution**: Comprehensive asset tracking and cleanup system
- **Files Changed**: `main.rs`, `terrain.rs`, `landscape.rs`, `player.rs`

#### Technical Implementation:
```rust
// New TerrainAssetTracker resource tracks all asset handles
pub struct TerrainAssetTracker {
    pub terrain_meshes: Vec<Handle<Mesh>>,
    pub terrain_materials: Vec<Handle<StandardMaterial>>,
    pub landscape_meshes: Vec<Handle<Mesh>>,
    pub landscape_materials: Vec<Handle<StandardMaterial>>,
    pub texture_atlas: Option<Handle<Image>>, // Reusable
}

// Asset cleanup before terrain recreation
asset_tracker.cleanup_assets(&mut meshes, &mut materials);
```

### 🎯 Terrain Recreation System Redesign
**IMPROVED**: Split monolithic system to avoid Bevy's 16-parameter limit
- **New Systems**: `terrain_recreation_system()` + `coordinate_sync_system()`
- **Proper Coordination**: Player/agent/beacon positioning after recreation
- **Enhanced Tracking**: Better coordinate synchronization

#### Key Features:
- Asset cleanup prevents memory leaks
- Player repositioned to (0,0,0) relative to new terrain center
- Agents repositioned based on their subpixel coordinates
- Beacons properly synchronized (player beacon + terrain center beacon)
- Cooldown increased to 5 seconds to prevent rapid recreations

## 🐛 Known Issues & Debugging

### Current Status
1. **Performance Issue** - ✅ RESOLVED: Asset memory leaks fixed
2. **Beacon Positioning** - ✅ RESOLVED: Proper synchronization implemented  
3. **Limited texture variety** - Only 10 textures used, mostly similar colors
4. **Debug spam** - `eprintln!` in texture selection clutters output

### Debug Features  
- Wireframe mode available via RapierDebugRenderPlugin
- Console output for subpixel positions and terrain recreation
- Asset cleanup messages: "ASSET CLEANUP: Removed X meshes and Y materials"
- Coordinate sync messages: "Repositioned N agents relative to new terrain center"

### Performance Notes
- **Asset Management**: Old asset handles properly removed from Bevy's asset system
- **Memory Usage**: No longer accumulates indefinitely during terrain recreation
- **Recreation Optimization**: Triangle mapping cleared, physics colliders cleaned up
- **Coordinate Tracking**: Synchronized across all entities (player, agents, beacons)

## 📝 Development Notes

### When Working on Textures
1. Current texture selection in `select_texture_from_rgba()` only uses red channel
2. Available textures: deepwater, dirt, drygrass, grass, stone, lava, moss, sand, snow, water, etc.
3. Atlas creator tool enhances contrast/saturation for flashy look
4. Material properties make textures more vivid than standard

### When Working on Terrain
1. Gnomonic projection centers terrain around player
2. Subpixel system: 3000×1500 pixels with 8 subdivisions each
3. Recreation threshold prevents rapid regeneration
4. Triangle mapping tracks physics colliders to subpixels

### When Working on Coordinates
1. Three systems: World (Vec3), Geographic (lat/lon), Subpixel (i,j,k)  
2. Coordinate conversions in planisphere.rs
3. Player position tracked across terrain recreations

## 🎯 Quick Start for New Sessions
1. `cargo check` - Verify compilation
2. Check recent git commits for context
3. Run briefly to see current behavior
4. Focus area usually indicated by user's specific request
5. Use TodoWrite tool for complex multi-step tasks

## 💡 Improvement Areas
- Expand texture selection to use all RGBA channels
- Utilize more of the 256 texture atlas slots  
- Add texture blending for smoother transitions
- Implement biome-based selection logic
- Remove debug output for cleaner console