# CLAUDE.md — Planet Terrain Game

## Project Overview

A Bevy-based planet exploration game. The player navigates a procedurally rendered
terrain mapped onto a sphere at real-world scale. The terrain is derived from a
pre-computed RGBA bitmap (the "planisphere") using a reduced latitude grid.

---

## Core Concepts

### Reduced Latitude Grid

The planisphere uses a reduced grid: each pixel has a fixed number of sub-pixel
rows (2^k north-south), but the number of sub-pixel columns decreases toward the
poles: `round(2^k * cos(φ))`. This avoids over-sampling near the poles.

- `N` pixels in latitude, `2N` pixels in longitude (at equator)
- `k` — global subdivision level, driven by player altitude (high altitude → low k)
- Near the poles, special rendering (polar discs) is reserved for future work

### Sub-pixels

A sub-pixel is the atomic terrain unit — one vertex in the mesh. It is addressed by:
- `ipixel` — pixel row (latitude axis)
- `jpixel` — pixel column (longitude axis)
- `ksouspixel` — linear index of the sub-pixel within the pixel (decomposable into `ki`, `kj`)

Key function (already implemented): `subpixel_to_indices(sub_row, sub_col, k) -> (ipixel, jpixel, ksouspixel)`

### Chunks

A chunk is a `2^k2 × 2^k2` sub-pixel window, independent of pixel boundaries.
`k2 <= k` is a fixed parameter defining chunk size in sub-pixels.

Chunks are the unit of async mesh generation and LOD management.

```rust
struct ChunkCoordGlobal {
    sub_row: u32,  // NW corner, global sub-pixel row
    sub_col: u32,  // NW corner, global sub-pixel col
}

struct ChunkCoordLocal {
    ci: i32,  // signed, relative to gnomon, in chunk units (north-south)
    cj: i32,  // signed, relative to gnomon, in chunk units (east-west)
}

struct Chunk {
    coord: ChunkCoordGlobal,
    k: u8,
    k2: u8,
    data: Vec<SubPixel>,  // (1<<k2)^2 entries, row-major
}

struct SubPixel {
    height: f32,
    // additional pixelfields (color, biome, etc.) TBD
}
```

### Gnomon

The gnomon is the anchor of the local chunk grid. It is a fixed geographic point
expressed in global sub-pixel coordinates. The chunk grid is centered on it.

```rust
struct Gnomon {
    sub_row: u32,
    sub_col: u32,
}
```

- `ChunkCoordLocal` → `ChunkCoordGlobal` conversion goes through the Gnomon
- The gnomon moves only occasionally (floating origin pattern), not every frame
- When the gnomon moves, chunk recycling must avoid a full regeneration spike — **this is an open problem, handle with care**

### Coordinate conversion

```rust
fn local_to_global(local: ChunkCoordLocal, gnomon: &Gnomon, k2: u8) -> ChunkCoordGlobal {
    let size = 1i64 << k2;
    ChunkCoordGlobal {
        sub_row: (gnomon.sub_row as i64 + local.ci as i64 * size) as u32,
        sub_col: (gnomon.sub_col as i64 + local.cj as i64 * size) as u32,
    }
}

fn n_subcols_at_pixel_row(ipixel: u32, k: u8, n_pixels_lat: u32) -> u32 {
    let phi = pixel_row_to_lat(ipixel, n_pixels_lat);
    (phi.cos() * (1u32 << k) as f32).round().max(1.0) as u32
}

fn subpixel_to_indices(sub_row: u32, sub_col: u32, k: u8) -> (u32, u32, u32) {
    let subdivision = 1u32 << k;
    let ipixel = sub_row / subdivision;
    let ki = sub_row % subdivision;
    let spp = n_subcols_at_pixel_row(ipixel, k, N_PIXELS_LAT);
    let jpixel = sub_col / spp;
    let kj = sub_col % spp;
    let ksouspixel = ki * subdivision + kj;
    (ipixel, jpixel, ksouspixel)
}
```

---

## Rendering Architecture

### Gnomonic Projection

The terrain is rendered in a local flat plane using a gnomonic projection centered
on the gnomon. The gnomon follows the player (with hysteresis — it jumps, not slides).
This makes the terrain locally flat, which is visually correct at ground level.

- Projection is re-defined each time the mesh is reloaded
- At very high altitude (k → 0), a switch to spherical rendering is planned but not yet implemented

### LOD

- `k` drives global subdivision (altitude-dependent)
- `k2` drives chunk size (fixed parameter)
- Same chunk coord + different k = different resolution, same geographic coverage
- Chunk generation is a pure function: `generate_chunk(bitmap, coord, k, k2) -> Chunk`

### Chunk Lifecycle (target architecture)

Each chunk has a status:

```rust
enum ChunkStatus {
    Pending,
    Generating,
    Ready,
    Stale,
}
```

Chunk generation must run on `AsyncComputeTaskPool` — never on the main thread.
Generation priority: chunks in the player's direction of travel first.

---

## Pixelfields

The bitmap is RGBA. Each channel is a "pixelfield" with its own semantic.
Pixelfields can also be computed (not just read from the bitmap).
Sub-pixel values are interpolated from pixel values — interpolation logic already exists.

**Do not assume channel semantics** — ask before using r/g/b/a for specific purposes.

---

## Debug: Chunk Wireframe Overlay

During chunk system development, a wireframe overlay visualizes the chunk grid
superimposed on the existing terrain. This keeps the game fully playable while
the chunk architecture is being built.

### Design

- A separate set of Bevy entities, tagged with a marker component:

```rust
#[derive(Component)]
struct ChunkDebugWireframe;
```

- Each debug chunk entity is a mesh that **exactly follows terrain height** —
  same geometry pipeline as the terrain, same gnomonic projection, same heights.
- A small vertical offset (`+y`, a few cm in world units) avoids z-fighting with
  the terrain mesh. No collision on these entities.
- Rendered with Bevy's `WireframePlugin` / `Wireframe` component.
- Toggled via a debug key (e.g. F3) — despawn/spawn all `ChunkDebugWireframe` entities.

### LOD of debug chunks

Debug chunks reflect the target LOD strategy. The effective `k2` for a chunk at
distance `d` chunks from the player is:

```
k2(d) = max(0, k - floor(d / R))
```

where `R` is the LOD ring radius in chunks (exposed as a parameter `LOD_RING_RADIUS`).

- At `d = 0` (player's chunk): `k2 = k` — maximum resolution
- Each additional `R` chunks of distance: `k2` drops by 1
- At `d >= k * R`: `k2 = 0` — chunk is a single quad

This produces concentric LOD rings around the player. The debug overlay makes
these rings directly visible.

### What NOT to do for debug wireframe

- Do not reuse terrain mesh entities — debug entities are always separate
- Do not add physics/collision to debug wireframe entities
- Do not generate debug meshes on the main thread
- Do not hardcode the vertical offset — expose it as a parameter `DEBUG_WIREFRAME_Y_OFFSET`

---

## Current State vs Target

| Concern | Current | Target |
|---|---|---|
| Chunk definition | pixel = chunk | independent `Chunk` struct |
| Chunk generation | blocking, main thread | async, `AsyncComputeTaskPool` |
| Gnomon movement | full remesh on trigger | incremental chunk recycling |
| Debug wireframe | none | wireframe overlay with LOD rings |
| Polar rendering | none | polar disc, future work |
| High-altitude view | none | spherical fallback at k=0 |

---

## What NOT to do

- Do not conflate pixels and chunks — they are independent grids
- Do not generate meshes on the main thread
- Do not move the gnomon and regenerate everything synchronously
- Do not assume chunks align with pixel boundaries
- Do not implement polar disc rendering yet — it is explicitly deferred
- Do not implement k=0 spherical fallback yet — deferred

---

## Key Parameters

| Name | Meaning |
|---|---|
| `N` | number of pixels in latitude |
| `k` | global subdivision level (altitude-driven) |
| `k2` | chunk size parameter, `1<<k2` sub-pixels per side, `k2 <= k` |
| `N_PIXELS_LAT` | = N, used in coordinate conversions |
| `LOD_RING_RADIUS` | radius in chunks of each LOD ring, used in `k2(d)` formula |
| `DEBUG_WIREFRAME_Y_OFFSET` | vertical offset of debug wireframe above terrain (world units) |