# 🎮 Tiles3D - Infinite Terrain Exploration with Subpixel Precision

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Game Engine](https://img.shields.io/badge/bevy-0.16-orange?style=for-the-badge)
![Physics](https://img.shields.io/badge/rapier3d-physics-blue?style=for-the-badge)

> A 3D terrain exploration simulation with infinite world generation, precise geographic coordinate tracking, and real-time terrain recreation, built with Rust and Bevy game engine.

## 🌟 Features

- **🌍 Infinite Terrain System**: Dynamic world generation with seamless exploration
  - **Gnomonic Projection**: Geographic coordinate system mapped to 3D world space
  - **Subpixel Precision**: High-resolution tile grid (3000×1500 pixels, 51 subpixel divisions)
  - **Real-time Recreation**: Terrain dynamically regenerates as player explores
  - **Distance-based Triggers**: Terrain recreation when player moves >37 tiles from center
  - **Coordinate Synchronization**: Maintains accurate position tracking across recreations

- **🎯 Player Character**: Full movement control with geographic tracking
  - **WASD Movement**: Forward/backward/strafe with mouse look controls
  - **Jump Mechanics**: Physics-based jumping with cooldown system
  - **Subpixel Tracking**: Real-time conversion between world/geographic coordinates
  - **Tile Beacon**: Visual indicator showing player's current grid position
  - **Infinite Movement**: No world boundaries - seamless terrain transitions

- **🤖 Autonomous Agents**: AI-driven entities with emergent behaviors
  - Random movement patterns and decision making
  - Physics-based jumping and collision detection
  - Item collection and inventory system
  - Ground detection using Rapier physics

- **📷 Interactive Camera**: Third-person camera that follows the player
  - Mouse wheel zoom with distance limits
  - Right-click drag rotation around player
  - Automatic player tracking through terrain recreations

- **🎨 Advanced Texture System**: Dynamic terrain texturing with geographic data
  - **Texture Atlas**: 16×16 grid of 256 terrain textures (deepwater, grass, stone, lava, etc.)
  - **RGBA-based Selection**: Geographic map data drives texture selection via color channels
  - **Flashy Materials**: Enhanced visual appeal with metallic shine and emissive glow
  - **Subpixel Accuracy**: Each terrain tile gets individual texture based on map position
  - **Real-time Processing**: Textures updated during terrain recreation for seamless transitions

- **🗺️ Geographic Coordinate System**:
  - **Manhattan Distance Calculation**: Efficient tile-based distance measurement
  - **Mean Tile Size Estimation**: Dynamic calculation from adjacent subpixel coordinates
  - **Coordinate Conversion**: Seamless translation between geographic and world coordinates
  - **Projection Center Management**: Maintains terrain center for coordinate calculations

## 🚀 Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable version)
- A graphics card that supports Vulkan, DirectX 12, or Metal

### Installation & Running

```bash
# Clone the repository
git clone <your-repo-url>
cd tiles3d

# Run the simulation
cargo run --release
```

## 🎮 Controls

| Control | Action |
|---------|--------|
| **W / ↑** | Move forward |
| **S / ↓** | Move backward |
| **A** | Strafe left |
| **D** | Strafe right |
| **Space** | Jump (with cooldown) |
| **Mouse Movement** | Look around (first-person style) |
| **Mouse Wheel** | Zoom camera in/out |
| **Right Click + Drag** | Rotate camera around player |
| **ESC** | Close application |

## 🏗️ Architecture

The project is organized into clean, modular components:

```
src/
├── main.rs         # Application entry point and resource initialization
├── player.rs       # Player movement, subpixel tracking, and terrain recreation
├── agent.rs        # Agent behavior and AI logic
├── terrain.rs      # Terrain generation, texture selection, and material setup
├── camera.rs       # Third-person camera controls and movement
├── planisphere.rs  # Geographic coordinate system and RGBA texture data processing
└── ui.rs          # User interface and coordinate display

assets/
├── maps/
│   └── sphere_texture.png     # Geographic map providing elevation + texture data
└── textures/
    ├── texture_atlas.png      # 256×256 atlas with 19 terrain textures
    ├── atlas_creator.py       # Tool for generating texture atlas from individual images
    └── img/                   # Individual terrain texture files
        ├── grass.png
        ├── stone.png
        ├── water.png
        └── ... (19 total)
```

### 🎯 Player System
The player character includes:
- **Movement Controller**: WASD + mouse look with physics integration
- **Subpixel Tracker**: Real-time position monitoring in (I,J,K) grid coordinates
- **Geographic Converter**: Seamless translation between world and geographic coordinates
- **Terrain Manager**: Automatic terrain recreation when distance thresholds are exceeded
- **Beacon System**: Visual tile position indicator that snaps to grid centers

### 🌍 Infinite Terrain System
Advanced terrain generation featuring:
- **Gnomonic Projection**: Maps spherical geographic coordinates to flat world space
- **Subpixel Grid**: High-resolution 3000×1500 pixel grid with 51 subdivisions per pixel
- **Dynamic Recreation**: Terrain regenerates when player moves >37 tiles from center
- **Coordinate Persistence**: Player position tracking survives terrain transitions
- **Distance Calculation**: Manhattan distance using mean tile size estimation

### 🗺️ Coordinate Systems
Multiple coordinate systems working in harmony:
- **World Coordinates**: 3D game space (X, Y, Z) with physics simulation
- **Geographic Coordinates**: Real-world latitude/longitude in degrees
- **Subpixel Coordinates**: Discrete grid positions (I, J, K) for tile mapping
- **Projection Center**: Dynamic center point for gnomonic coordinate transformations

### 🎨 Texture System
Advanced terrain texturing featuring:
- **Geographic-driven Textures**: `sphere_texture.png` provides both elevation and color data
- **Dual-purpose Processing**: Single image source for terrain height AND texture selection
- **RGBA Channel Mapping**: Color values converted to texture atlas indices via `select_texture_from_rgba()`
- **Texture Atlas Management**: 256×256 pixel atlas with 19 terrain types (grass, stone, water, lava, etc.)
- **Enhanced Materials**: Flashy visual style with metallic shine, emissive glow, and brightness boost
- **Subpixel Resolution**: Individual texture selection for each terrain quad
- **Seamless Recreation**: Textures recalculated during terrain regeneration

### 🧠 Agent System
Autonomous entities with:
- **Movement AI**: Random decision making for exploration
- **Physics Body**: Realistic movement with gravity and collision
- **Sensors**: Invisible detection spheres for item pickup
- **Orientation**: Visual markers showing facing direction

## 🛠️ Technical Details

### Built With
- **[Bevy](https://bevyengine.org/)** - Modern Rust game engine
- **[Rapier3D](https://rapier.rs/)** - Physics simulation
- **Rust** - Systems programming language

### Key Concepts Demonstrated
- **Entity Component System (ECS)** architecture with Bevy
- **Physics simulation** with rigid bodies and colliders
- **Geographic projections** and coordinate system transformations
- **Infinite world generation** with seamless terrain transitions
- **Real-time coordinate tracking** across multiple reference frames
- **Distance-based algorithms** using Manhattan distance calculations
- **Event-driven systems** for input and collision handling
- **Resource management** for shared state and coordinate synchronization

## 🎓 Learning Resource

This project is heavily commented and designed as a learning resource for:
- **Rust beginners** - Extensive comments explaining Rust syntax and concepts
- **Game development newcomers** - ECS patterns and 3D game basics
- **Bevy learners** - Real-world usage of Bevy systems and components
- **Geographic programming** - Coordinate projections and transformations
- **Infinite world developers** - Techniques for seamless terrain generation

### Educational Highlights
- 📖 **Comprehensive Documentation**: Every function, algorithm, and concept explained
- 🧩 **Modular Design**: Clean separation of concerns with clear interfaces
- 🔧 **Beginner-Friendly**: Assumes no prior game development experience
- 🎯 **Advanced Techniques**: Real implementations of geographic projections and infinite worlds
- 🗺️ **Coordinate Systems**: Detailed examples of multi-coordinate transformations

## 🎨 Customization

Easy to modify and extend:

```rust
// Adjust terrain recreation distance threshold
terrain_center.max_subpixel_distance = 50; // Recreate when 50 tiles from center

// Change planisphere resolution
let planisphere = Planisphere::new(6000, 3000, 100); // Higher resolution grid

// Modify player movement speed
Player {
    move_speed: 25.0,  // Faster movement
    mouse_sensitivity: 0.003,  // More sensitive mouse look
    // ... other properties
}

// Customize texture material appearance (in terrain.rs)
StandardMaterial {
    base_color: Color::srgb(2.0, 2.0, 2.0), // Even brighter textures
    metallic: 0.8,                          // More metallic shine  
    emissive: Color::srgb(0.2, 0.2, 0.2),   // Stronger glow effect
    // ... other properties
}

// Modify texture selection logic (in select_texture_from_rgba)
let texture_index = if red < 0.1 {
    0  // Deep water
} else if red < 0.3 {
    // Use green channel for variety
    if green > 0.5 { 15 } else { 1 }  // Grass vs dirt based on green
} // ... expand to use all RGBA channels

// Adjust terrain recreation cooldown
if distance_in_tiles > threshold && time_since_last > 1.0 { // Faster recreation

// Change agent behavior
Agent {
    turn_speed: 4.0,  // Faster turning
    // ... other properties
}
```

## 🐛 Debugging Features

- **Wireframe Mode**: Visualize terrain mesh topology
- **Physics Debug**: Uncomment `RapierDebugRenderPlugin` to see collision shapes
- **Subpixel Tracking**: Real-time console output of player's (I,J,K) coordinates
- **Distance Monitoring**: Live display of tile distance and recreation thresholds
- **Terrain Recreation Logs**: Detailed output during terrain regeneration events
- **Coordinate Conversion**: Debug output for world ↔ geographic transformations
- **Beacon Visualization**: Red glowing beacon shows player's current tile center

## 🚧 Future Ideas

### 🎨 Texture System Enhancements
- **Multi-Channel Textures**: Utilize all RGBA channels for complex terrain classification
- **Biome-based Selection**: Combine elevation + temperature + humidity for realistic biomes  
- **Texture Blending**: Smooth transitions between different terrain types
- **Seasonal Variations**: Dynamic texture changes based on time/weather
- **Procedural Noise**: Add randomization to avoid repetitive patterns

### 🌍 General Enhancements
- 🗺️ **Real Elevation Data**: Import actual topographic maps for terrain generation
- 🌊 **Ocean Rendering**: Water surfaces with proper sea-level detection
- 🏔️ **Height Variations**: Incorporate elevation data into terrain mesh generation
- 📍 **GPS Integration**: Real-world coordinate system with actual geographic data
- 🧭 **Navigation Tools**: Compass, coordinates display, and waypoint system
- 🌍 **Multiple Projections**: Support for different map projections beyond gnomonic
- 🧬 **Genetic Algorithm**: Evolve agent behaviors over generations
- 🌐 **Neural Networks**: ML-driven agent decision making
- 🌦️ **Environmental Systems**: Weather, day/night cycles
- 👥 **Multi-Agent Interactions**: Communication and cooperation

## 📝 License

This project is open source and available under the [MIT License](LICENSE).

## 🤝 Contributing

Contributions are welcome! This project is particularly suited for:
- Adding new agent behaviors
- Implementing different terrain generation algorithms
- Improving documentation and comments
- Adding visual enhancements

## 📚 Additional Resources

- [Bevy Documentation](https://docs.rs/bevy/latest/bevy/)
- [Rapier3D User Guide](https://rapier.rs/docs/user_guides/rust/getting_started)
- [Rust Book](https://doc.rust-lang.org/book/) - For Rust beginners
- [Game Programming Patterns](https://gameprogrammingpatterns.com/) - Design patterns in games

---

*Built with ❤️ in Rust* 🦀