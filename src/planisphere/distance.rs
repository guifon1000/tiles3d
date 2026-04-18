use super::Planisphere;

/// Distance calculation method for subpixel selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DistanceMethod {
    /// Manhattan distance (diamond pattern)
    Manhattan,
    /// Euclidean distance (circular pattern)
    Euclidean,
    /// Chebyshev distance (square pattern)
    #[default]
    Chebyshev,
}

impl Planisphere {
    /// Returns all subpixels whose distance from `(center_i, center_j, center_k)` is at most
    /// `max_subpixel_distance`, using the chosen `DistanceMethod`:
    ///
    /// - `Manhattan`  — sum of pixel-grid distances scaled by `subpixel_divisions`
    /// - `Euclidean`  — straight-line distance in continuous subpixel space
    /// - `Chebyshev`  — Euclidean distance in gnomonic world space
    pub fn get_subpixels_by_distance_method(
        &self,
        center_i: usize,
        center_j: usize,
        center_k: usize,
        max_subpixel_distance: usize,
        method: DistanceMethod,
    ) -> Vec<(usize, usize, usize, [(f64, f64); 4])> {
        // --- bounding search rectangle (same for all methods) ---
        let pixel_radius = (max_subpixel_distance / self.subpixel_divisions) + 2;
        let min_i = center_i.saturating_sub(pixel_radius);
        let max_i = center_i + pixel_radius;
        let min_j = center_j.saturating_sub(pixel_radius);
        let max_j = (center_j + pixel_radius).min(self.height_pixels - 1);

        let candidates = self.get_subpixels_in_rectangle(min_i, max_i, min_j, max_j);

        // --- centre in continuous subpixel space (shared by all methods) ---
        let center_sub_i = center_k / self.subpixel_divisions;
        let center_sub_j = center_k % self.subpixel_divisions;
        let cx = (center_i * self.subpixel_divisions + center_sub_i) as f64;
        let cy = (center_j * self.subpixel_divisions + center_sub_j) as f64;
        let max_dist = max_subpixel_distance as f64;

        let mut result = Vec::with_capacity(candidates.len());
        // Centre is always included first
        result.push((center_i, center_j, center_k,
                     self.get_subpixel_corners(center_i, center_j, center_k)));

        for (i, j, k, corners) in candidates {
            if i == center_i && j == center_j && k == center_k {
                continue; // already added above
            }

            // All methods share the same continuous subpixel coordinates
            let sub_i = k / self.subpixel_divisions;
            let sub_j = k % self.subpixel_divisions;
            let x = (i * self.subpixel_divisions + sub_i) as f64;
            let y = (j * self.subpixel_divisions + sub_j) as f64;
            let dx = x - cx;
            let dy = y - cy;

            let in_range = match method {
                // Diamond: sum of absolute distances
                DistanceMethod::Manhattan  => dx.abs() + dy.abs() <= max_dist,
                // Circle: Euclidean distance
                DistanceMethod::Euclidean  => dx.hypot(dy)        <= max_dist,
                // Square: largest of the two axis distances
                DistanceMethod::Chebyshev  => dx.abs().max(dy.abs()) <= max_dist,
            };

            if in_range {
                result.push((i, j, k, corners));
            }
        }

        result
    }

    pub fn get_subpixels_rect_centered_on_subpixel(
        &self,
        center_i: usize,
        center_j: usize,
        center_k: usize,
        nx: i32,
        ny: i32,
    ) -> Vec<(usize, usize, usize, [(f64, f64); 4])> {
        let mut result = Vec::new();

        let half_nx = nx as i32 / 2;
        let half_ny = ny as i32 / 2;
        let (_blc_i, _blc_j, _blc_k) = self.get_neighbour_subpixel(center_i, center_j, center_k, -half_nx, -half_ny);
        for iy in 0..ny {
            for ix in 0..nx {
                // Calculate the offsets from the center subpixel
                let dx = ix as i32 - half_nx;
                let dy = iy as i32 - half_ny;

                // Get the neighbor subpixel coordinates
                let (i, j, k) = self.get_neighbour_subpixel(center_i, center_j, center_k, dx, dy);

                // Get the corners of this subpixel
                let corners = self.get_subpixel_corners(i, j, k);

                // Add to the result
                result.push((i, j, k, corners));
            }
        }

/*         for dy in -half_ny..half_ny {
            for dx in -half_nx..half_nx {
                let (i, j, k) = self.get_neighbour_subpixel(center_i, center_j, center_k, dx, dy);
                eprintln!("Pixel: ({}, {}), Subpixel: {}, Neighbor: ({}, {}, {})", center_i, center_j, center_k, i, j, k);
                let corners = self.get_subpixel_corners(i, j, k);
                result.push((i, j, k, corners));
            }
        } */
        result
    }

    /// Gets a mesh representation of subpixels within a specific distance
    ///
    /// # Parameters
    /// * `center_i` - Center pixel x coordinate
    /// * `center_j` - Center pixel y coordinate
    /// * `center_k` - Center subpixel index
    /// * `max_subpixel_distance` - Maximum subpixel distance from the center to include
    ///
    /// # Returns
    /// A tuple containing:
    /// - A vector of vertices (x, y, z) where x=lon-center_lon, y=lat-center_lat, z=0
    /// - A vector of triangle indices (each triplet forms a triangle)
    /// - A vector of subpixel info (i, j, k) for each quadrilateral in the mesh
    pub fn get_subpixel_mesh_by_distance(&self, center_i: usize, center_j: usize, center_k: usize, max_subpixel_distance: usize)
        -> (Vec<(f64, f64, f64)>, Vec<usize>, Vec<(usize, usize, usize)>) {
        // Default to Manhattan distance for backward compatibility
        self.get_subpixel_mesh_by_distance_method(center_i, center_j, center_k, max_subpixel_distance, DistanceMethod::Manhattan)
    }

    /// Get subpixel mesh using the specified distance calculation method
    pub fn get_subpixel_mesh_by_distance_method(&self, center_i: usize, center_j: usize, center_k: usize, max_subpixel_distance: usize, method: DistanceMethod)
        -> (Vec<(f64, f64, f64)>, Vec<usize>, Vec<(usize, usize, usize)>) {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        let mut subpixel_info = Vec::new();

        // Get center coordinates for relative positioning
        let (center_lon, center_lat) = self.subpixel_to_geo(center_i, center_j, center_k);

        // Get subpixels within the specified distance using the selected method
        let subpixels = self.get_subpixels_by_distance_method(center_i, center_j, center_k, max_subpixel_distance, method);

        // Pre-allocate with approximate sizes
        let quad_count = subpixels.len();
        vertices.reserve(quad_count * 4); // Each quad has 4 vertices
        triangles.reserve(quad_count * 6); // Each quad is made of 2 triangles (6 indices)
        subpixel_info.reserve(quad_count);

        // Process each subpixel
        for (i, j, k, corners) in subpixels {
            // Calculate the vertex indices for this quad
            let base_index = vertices.len();

            // Add the four vertices of this subpixel (shifted relative to center)
            // Order: top-left, top-right, bottom-right, bottom-left (clockwise)
            for (lon, lat) in corners.iter() {
                vertices.push((lon - center_lon, lat - center_lat, 0.0));
            }

            // Add the two triangles that form this quad
            // First triangle: top-left, top-right, bottom-right
            triangles.push(base_index);
            triangles.push(base_index + 1);
            triangles.push(base_index + 2);

            // Second triangle: top-left, bottom-right, bottom-left
            triangles.push(base_index);
            triangles.push(base_index + 2);
            triangles.push(base_index + 3);

            // Add the subpixel info
            subpixel_info.push((i, j, k));
        }

        (vertices, triangles, subpixel_info)
    }

    /// Gets a mesh representation of subpixels within a specific distance using gnomonic projection
    ///
    /// # Parameters
    /// * `center_i` - Center pixel x coordinate
    /// * `center_j` - Center pixel y coordinate
    /// * `center_k` - Center subpixel index
    /// * `max_subpixel_distance` - Maximum subpixel distance from the center to include
    /// * `projection_center_lon` - Center longitude for gnomonic projection (degrees)
    /// * `projection_center_lat` - Center latitude for gnomonic projection (degrees)
    /// * `planet_radius` - Radius of the planet for projection scaling
    ///
    /// # Returns
    /// A tuple containing:
    /// - A vector of vertices (x, y, z) where x,y are gnomonic projection coordinates, z=0
    /// - A vector of triangle indices (each triplet forms a triangle)
    /// - A vector of subpixel info (i, j, k) for each quadrilateral in the mesh
    pub fn get_subpixel_mesh_by_distance_gnomonic(&self, center_i: usize, center_j: usize, center_k: usize, max_subpixel_distance: usize,
                                              projection_center_lon: f64, projection_center_lat: f64, _planet_radius: f64)
        -> (Vec<(f64, f64, f64)>, Vec<usize>, Vec<(usize, usize, usize)>) {
        // Default to Manhattan distance for backward compatibility
        self.get_subpixel_mesh_by_distance_gnomonic_method(center_i, center_j, center_k, max_subpixel_distance, projection_center_lon, projection_center_lat, _planet_radius, DistanceMethod::Manhattan)
    }

    /// Get subpixel mesh using gnomonic projection with the specified distance calculation method
    pub fn get_subpixel_mesh_by_distance_gnomonic_method(&self, center_i: usize, center_j: usize, center_k: usize, max_subpixel_distance: usize,
                                              projection_center_lon: f64, projection_center_lat: f64, _planet_radius: f64, method: DistanceMethod)
        -> (Vec<(f64, f64, f64)>, Vec<usize>, Vec<(usize, usize, usize)>) {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        let mut subpixel_info = Vec::new();

        // Get subpixels within the specified distance using the selected method
        let subpixels = self.get_subpixels_by_distance_method(center_i, center_j, center_k, max_subpixel_distance, method);

        // Pre-allocate with approximate sizes
        let quad_count = subpixels.len();
        vertices.reserve(quad_count * 4); // Each quad has 4 vertices
        triangles.reserve(quad_count * 6); // Each quad is made of 2 triangles (6 indices)
        subpixel_info.reserve(quad_count);

        // Process each subpixel
        for (i, j, k, corners) in subpixels {
            // Calculate the vertex indices for this quad
            let base_index = vertices.len();

            // Add the four vertices of this subpixel using gnomonic projection
            // Order: top-left, top-right, bottom-right, bottom-left (clockwise)
            for (lon, lat) in corners.iter() {
                // Convert geographic coordinates to gnomonic projection with planet radius
                let (x, y) = self.geo_to_gnomonic(*lon, *lat, projection_center_lon, projection_center_lat);
                vertices.push((x, y, 0.0));
            }

            // Add the two triangles that form this quad
            // First triangle: top-left, top-right, bottom-right
            triangles.push(base_index);
            triangles.push(base_index + 1);
            triangles.push(base_index + 2);

            // Second triangle: top-left, bottom-right, bottom-left
            triangles.push(base_index);
            triangles.push(base_index + 2);
            triangles.push(base_index + 3);

            // Add the subpixel info
            subpixel_info.push((i, j, k));
        }

        (vertices, triangles, subpixel_info)
    }
}
