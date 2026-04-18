use crate::planisphere;
use super::texture::select_texture_from_rgba;

pub fn terrain_mesh(
    planisphere: &planisphere::Planisphere,
    subpixels: Vec<(usize, usize, usize, [(f64, f64); 4])>,
    lonlat_gnomocenter: (f64, f64),
) -> (Vec<[f32; 3]>, Vec<u32>, Vec<[f32; 2]>, Vec<(usize, usize, usize)>) {
    let mut vertices = Vec::<[f32; 3]>::new();
    let mut indices = Vec::<u32>::new();
    let mut uvs = Vec::<[f32; 2]>::new();
    let mut vertex_index = 0u32;
    let mut triangle_mapping = Vec::<(usize, usize, usize)>::new();
    for (_i, _j, _k, _corners) in subpixels.iter() {
        let (i, j, k) = (*_i, *_j, *_k);
        let corners = *_corners;
        let current_pixel_norm_lat = j as f64 / planisphere.height_pixels as f64;
        let current_latitude = current_pixel_norm_lat * 180.0 - 90.0;
        let current_lon_subdivisions = (planisphere.subpixel_divisions as f64 * current_latitude.to_radians().cos()).max(1.0) as usize;
        // Create vertices for this subpixel — each corner gets its own altitude
        let corner_altis = planisphere.get_altitude_at_subpixel_corners(i as i32, j as i32, k);
        for ((lon, lat), alti) in corners.iter().zip(corner_altis.iter()) {
            let (x, y) = planisphere.geo_to_gnomonic(*lon, *lat, lonlat_gnomocenter.0, lonlat_gnomocenter.1);
            vertices.push([x as f32, (5.0 as f32) * alti, y as f32]);
        }
        let atlas_size = crate::config::atlas::SIZE;

        // Texture selection mode - set to true for RGBA-based, false for border-based
        let use_rgba_texture_selection = true;

        let tile_index = if use_rgba_texture_selection {
            // RGBA-based texture selection
            let (red, green, blue, alpha) = planisphere.get_rgba_at_subpixel(i as i32, j as i32, k);
            select_texture_from_rgba(red, green, blue, alpha)
        } else {
            // Original border-based texture selection
            let mut tile_index = 5; // default texture

            //north border
            if k % planisphere.subpixel_divisions == 0 {
                tile_index = 15; //north
            }

            //south border
            if k % planisphere.subpixel_divisions == planisphere.subpixel_divisions - 1 {
                tile_index = 12;
            }

            //west border
            if k / planisphere.subpixel_divisions == 0 {
                tile_index = 13;
            }

            //east border
            if k / planisphere.subpixel_divisions == current_lon_subdivisions - 1 {
                tile_index = 7;
            }

            tile_index
        };

        let tile_u = (tile_index % atlas_size) as f32 / atlas_size as f32;
        let tile_v = (tile_index / atlas_size) as f32 / atlas_size as f32;
        let tile_size = 1.0 / atlas_size as f32;

        // UVs for this quad
        uvs.push([tile_u, tile_v]); // bottom-left
        uvs.push([tile_u + tile_size, tile_v]); // bottom-right
        uvs.push([tile_u + tile_size, tile_v + tile_size]); // top-right
        uvs.push([tile_u, tile_v + tile_size]); // top-left

        // Create triangles (two triangles per quad)
        indices.extend_from_slice(&[
            vertex_index, vertex_index + 1, vertex_index + 2,
            vertex_index, vertex_index + 2, vertex_index + 3
        ]);

        // Map both triangles to this subpixel (i, j, k)
        triangle_mapping.push((i, j, k)); // Triangle 1
        triangle_mapping.push((i, j, k)); // Triangle 2

        vertex_index += 4;
    }
    (vertices, indices, uvs, triangle_mapping)
}
