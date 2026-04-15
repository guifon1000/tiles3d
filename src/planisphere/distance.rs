use bevy::math::ops::sqrt;
use super::Planisphere;

/// Distance calculation method for subpixel selection
#[derive(Debug, Clone, Copy)]
pub enum DistanceMethod {
    /// Manhattan distance (diamond pattern) - original method
    Manhattan,
    /// Euclidean distance (circular pattern) - more natural
    Euclidean,
    /// Chebyshev distance (rectangular pattern) - square selection
    Chebyshev,
}

impl Planisphere {
    /// Gets subpixels within a specific distance from a center point, limiting the search to a 3x3 grid of pixels
    ///
    /// # Parameters
    /// * `center_i` - Center pixel x coordinate
    /// * `center_j` - Center pixel y coordinate
    /// * `center_k` - Center subpixel index
    /// * `max_subpixel_distance` - Maximum subpixel distance from the center to include
    ///
    /// # Returns
    /// A vector of tuples containing (i, j, k, corners) for subpixels in the region
    pub fn get_subpixels_by_distance(&self, center_i: usize, center_j: usize, center_k: usize, max_subpixel_distance: usize)
        -> Vec<(usize, usize, usize, [(f64, f64); 4])> {
        let mut result = Vec::new();

        // Calculate how many pixels we need to include in each direction based on the max_subpixel_distance
        // Allow for distance to go beyond a single pixel
        let pixel_radius = (max_subpixel_distance / self.subpixel_divisions) + 1;

        // Get the grid of pixels centered on the player's pixel, sized based on the distance
        let min_i = if center_i > pixel_radius { center_i - pixel_radius } else { 0 };
        //let max_i = std::cmp::min(center_i + pixel_radius, self.width_pixels - 1);
        let max_i =center_i + pixel_radius;
        let min_j = if center_j > pixel_radius { center_j - pixel_radius } else { 0 };
        let max_j = std::cmp::min(center_j + pixel_radius, self.height_pixels - 1);
        //eprint!("Subpixel search area: ({}, {}) to ({}, {})\n", min_i, min_j, max_i, max_j);
        // Get all subpixels in the pixel grid
        let subpixels = self.get_subpixels_in_rectangle(min_i, max_i, min_j, max_j);

        // Add the center subpixel first
        result.push((center_i, center_j, center_k, self.get_subpixel_corners(center_i, center_j, center_k)));

        // Pre-allocate with approximate capacity
        let approx_subpixels_per_pixel = self.subpixel_divisions * self.subpixel_divisions;
        let approx_total = (max_i - min_i + 1) * (max_j - min_j + 1) * approx_subpixels_per_pixel;
        result.reserve(approx_total);

        // Use a simpler but more consistent distance metric
        for (i, j, k, corners) in subpixels {
            // Skip the center (already added)
            if i == center_i && j == center_j && k == center_k {
                continue;
            }

            // Calculate distance using pixel and subpixel coordinates
            let dist_i = if i > center_i { i - center_i } else { center_i - i };
            let dist_j = if j > center_j { j - center_j } else { center_j - j };

            // Calculate the base distance in terms of full pixels
            let pixel_distance = dist_i + dist_j;

            // If in a different pixel, add the subpixel component
            let subpixel_distance = pixel_distance * self.subpixel_divisions;

            // If in the same pixel, calculate subpixel distance directly
            if i == center_i && j == center_j {
                let center_sub_i = center_k / self.subpixel_divisions;
                let center_sub_j = center_k % self.subpixel_divisions;
                let sub_i = k / self.subpixel_divisions;
                let sub_j = k % self.subpixel_divisions;

                let sub_dist_i = if sub_i > center_sub_i { sub_i - center_sub_i } else { center_sub_i - sub_i };
                let sub_dist_j = if sub_j > center_sub_j { sub_j - center_sub_j } else { center_sub_j - sub_j };

                let _subpixel_distance = sub_dist_i + sub_dist_j;
            } else {
                // For neighboring pixels (including diagonal ones), calculate a more accurate distance
                let center_sub_i = center_k / self.subpixel_divisions;
                let center_sub_j = center_k % self.subpixel_divisions;
                let sub_i = k / self.subpixel_divisions;
                let sub_j = k % self.subpixel_divisions;

                // For each dimension, calculate how many subpixel steps it takes to reach the target
                let mut total_steps = 0;

                // Handle the horizontal component
                if i != center_i {
                    // Calculate horizontal steps
                    if i > center_i {
                        // Moving right: distance is proportional to how far right in center pixel
                        // and how far left in target pixel
                        total_steps += (self.subpixel_divisions - center_sub_i) + sub_i;
                    } else {
                        // Moving left: distance is proportional to how far left in center pixel
                        // and how far right in target pixel
                        total_steps += center_sub_i + (self.subpixel_divisions - sub_i);
                    }

                    // If moving more than 1 pixel, add the intermediate pixel distance
                    if dist_i > 1 {
                        total_steps += (dist_i - 1) * self.subpixel_divisions;
                    }
                } else {
                    // Same column, just calculate subpixel distance
                    total_steps += if sub_i > center_sub_i { sub_i - center_sub_i } else { center_sub_i - sub_i };
                }

                // Handle the vertical component
                if j != center_j {
                    // Calculate vertical steps
                    if j > center_j {
                        // Moving down: distance is proportional to how far down in center pixel
                        // and how far up in target pixel
                        total_steps += (self.subpixel_divisions - center_sub_j) + sub_j;
                    } else {
                        // Moving up: distance is proportional to how far up in center pixel
                        // and how far down in target pixel
                        total_steps += center_sub_j + (self.subpixel_divisions - sub_j);
                    }

                    // If moving more than 1 pixel, add the intermediate pixel distance
                    if dist_j > 1 {
                        total_steps += (dist_j - 1) * self.subpixel_divisions;
                    }
                } else {
                    // Same row, just calculate subpixel distance
                    total_steps += if sub_j > center_sub_j { sub_j - center_sub_j } else { center_sub_j - sub_j };
                }

                // For Manhattan distance, we take the sum of the distances
                let _subpixel_distance = total_steps;
            }

            // Include if within the maximum subpixel distance
            if subpixel_distance <= max_subpixel_distance {
                result.push((i, j, k, corners));
            }
        }

        result
    }

    /// Get subpixels within a circular region using Euclidean distance
    /// This creates a more natural circular selection pattern instead of diamond
    pub fn get_subpixels_by_circular_distance(&self, center_i: usize, center_j: usize, center_k: usize, max_subpixel_distance: usize)
        -> Vec<(usize, usize, usize, [(f64, f64); 4])> {
        let mut result = Vec::new();

        // Calculate how many pixels we need to include in each direction based on the max_subpixel_distance
        // Use a slightly larger radius to ensure we don't miss edge cases
        let pixel_radius = (max_subpixel_distance / self.subpixel_divisions) + 2;

        // Get the grid of pixels centered on the player's pixel, sized based on the distance
        let min_i = if center_i > pixel_radius { center_i - pixel_radius } else { 0 };
        let max_i = center_i + pixel_radius;
        let min_j = if center_j > pixel_radius { center_j - pixel_radius } else { 0 };
        let max_j = std::cmp::min(center_j + pixel_radius, self.height_pixels - 1);

        // Get all subpixels in the pixel grid
        let subpixels = self.get_subpixels_in_rectangle(min_i, max_i, min_j, max_j);

        // Add the center subpixel first
        result.push((center_i, center_j, center_k, self.get_subpixel_corners(center_i, center_j, center_k)));

        // Pre-allocate with approximate capacity
        let approx_subpixels_per_pixel = self.subpixel_divisions * self.subpixel_divisions;
        let approx_total = (max_i - min_i + 1) * (max_j - min_j + 1) * approx_subpixels_per_pixel;
        result.reserve(approx_total);

        // Convert center coordinates to continuous subpixel coordinates
        let center_sub_i = center_k / self.subpixel_divisions;
        let center_sub_j = center_k % self.subpixel_divisions;
        let center_continuous_i = (center_i * self.subpixel_divisions + center_sub_i) as f64;
        let center_continuous_j = (center_j * self.subpixel_divisions + center_sub_j) as f64;

        // Use Euclidean distance for circular pattern
        for (i, j, k, corners) in subpixels {
            // Skip the center (already added)
            if i == center_i && j == center_j && k == center_k {
                continue;
            }

            // Convert current coordinates to continuous subpixel coordinates
            let sub_i = k / self.subpixel_divisions;
            let sub_j = k % self.subpixel_divisions;
            let continuous_i = (i * self.subpixel_divisions + sub_i) as f64;
            let continuous_j = (j * self.subpixel_divisions + sub_j) as f64;

            // Calculate Euclidean distance
            let dx = continuous_i - center_continuous_i;
            let dy = continuous_j - center_continuous_j;
            let euclidean_distance = (dx * dx + dy * dy).sqrt();

            // Include if within the maximum subpixel distance
            if euclidean_distance <= max_subpixel_distance as f64 {
                result.push((i, j, k, corners));
            }
        }

        result
    }

    /// Get subpixels within a rectangular region
    /// This provides a simple rectangular selection pattern
    pub fn get_subpixels_by_rectangular_distance(&self, center_i: usize, center_j: usize, center_k: usize, max_subpixel_distance: usize)
        -> Vec<(usize, usize, usize, [(f64, f64); 4])> {
        let mut result = Vec::new();
        let (longitude, latitude) = self.subpixel_to_geo(center_i, center_j, center_k);
        // Calculate how many pixels we need to include in each direction
        let pixel_radius_y = (max_subpixel_distance / self.subpixel_divisions) + 1;
        let pixel_radius_x = (max_subpixel_distance / self.get_lon_subdivisons(latitude)) + 1;



        // Get the grid of pixels centered on the player's pixel
        let min_i = if center_i > pixel_radius_x { center_i - pixel_radius_x } else { 0  };
        let max_i = (center_i + pixel_radius_x);//%self.width_pixels;// crash
        let min_j = if center_j > pixel_radius_y { center_j - pixel_radius_y } else { 0 };
        let max_j = std::cmp::min(center_j + pixel_radius_y, self.height_pixels - 1);
        let mut subpixels = self.get_subpixels_in_rectangle(min_i, max_i, min_j, max_j);

            let _min_i = center_i as i32 - pixel_radius_x as i32;
            let _max_i = center_i as i32 + pixel_radius_x as i32;
            let _min_j = center_j as i32 - pixel_radius_y as i32;
            let _max_j = center_j as i32 + pixel_radius_y as i32;
        if _min_j < 0 {
        subpixels = self.get_subpixels_in_rectangle(0, self.get_width_pixels(), 0, max_j);
        }
        if _max_j >= self.get_height_pixels() as i32 {
        subpixels = self.get_subpixels_in_rectangle(0, self.get_width_pixels(), min_j, self.get_height_pixels());
        }
            if (_min_i < 0) && (_max_i < self.get_width_pixels() as i32) {
                let mini = self.get_width_pixels() as i32 + _min_i;
                let maxi = self.get_width_pixels();
                subpixels.append(&mut self.get_subpixels_in_rectangle(mini as usize, maxi, min_j, max_j));
            }
            if (_min_i >= 0) && (_max_i >= self.get_width_pixels() as i32) {
                let mini = 0usize;
                let maxi = (_max_i as usize) % self.get_width_pixels();
                subpixels.append(&mut self.get_subpixels_in_rectangle(mini, maxi, min_j, max_j));
            }
        // Add the center subpixel first
        result.push((center_i, center_j, center_k, self.get_subpixel_corners(center_i, center_j, center_k)));

        // Pre-allocate with approximate capacity
        let approx_subpixels_per_pixel = self.subpixel_divisions * self.subpixel_divisions;
        let approx_total = (max_i - min_i + 1) * (max_j - min_j + 1) * approx_subpixels_per_pixel;
        result.reserve(approx_total);

        // Convert center coordinates to continuous subpixel coordinates
        let center_sub_i = center_k / self.subpixel_divisions;
        let center_sub_j = center_k % self.subpixel_divisions;
        let center_continuous_i = (center_i * self.subpixel_divisions + center_sub_i) as f64;
        let center_continuous_j = (center_j * self.subpixel_divisions + center_sub_j) as f64;

        let center_geo = self.subpixel_to_geo(center_i, center_j, center_k);
        // Use Chebyshev distance (max of dx, dy) for rectangular pattern
        for (i, j, k, corners) in subpixels {

            let subpixel_geo = self.subpixel_to_geo(i, j, k);
            let subpixel_world = self.geo_to_gnomonic(subpixel_geo.0, subpixel_geo.1, center_geo.0, center_geo.1);
            // Skip the center (already added)
            if i == center_i && j == center_j && k == center_k {
                continue;
            }
            let d2 = subpixel_world.0*subpixel_world.0 + subpixel_world.1*subpixel_world.1;
            let d = sqrt(d2 as f32);
            if d<= max_subpixel_distance as f32 {
                result.push((i, j, k, corners));
            }
            // Convert current coordinates to continuous subpixel coordinates
            let sub_i = k / self.subpixel_divisions;
            let sub_j = k % self.subpixel_divisions;
            let continuous_i = (i * self.subpixel_divisions + sub_i) as f64;
            let continuous_j = (j * self.subpixel_divisions + sub_j) as f64;
            //let distance_to_center = sqrt()

            // Calculate Chebyshev distance (rectangular pattern)
            let dx = (continuous_i - center_continuous_i).abs() as f32;
            let dy = (continuous_j - center_continuous_j).abs() as f32;
            let chebyshev_distance = sqrt(dx*dx+dy*dy);

            // Include if within the maximum subpixel distance
            //if chebyshev_distance  <= max_subpixel_distance as f32 {
            //    result.push((i, j, k, corners));
            //}
        }

        result
    }

    /// Get subpixels using the specified distance calculation method
    /// This provides a unified interface for different selection patterns
    pub fn get_subpixels_by_distance_method(
        &self,
        center_i: usize,
        center_j: usize,
        center_k: usize,
        max_subpixel_distance: usize,
        method: DistanceMethod
    ) -> Vec<(usize, usize, usize, [(f64, f64); 4])> {
        match method {
            DistanceMethod::Manhattan => self.get_subpixels_by_distance(center_i, center_j, center_k, max_subpixel_distance),
            DistanceMethod::Euclidean => self.get_subpixels_by_circular_distance(center_i, center_j, center_k, max_subpixel_distance),
            DistanceMethod::Chebyshev => self.get_subpixels_by_rectangular_distance(center_i, center_j, center_k, max_subpixel_distance),
        }
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
