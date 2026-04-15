use super::Planisphere;

impl Planisphere {
    /// Converts a grid position (including subpixel) to geographic coordinates
    ///
    /// # Parameters
    /// * `i` - Horizontal grid index (pixel)
    /// * `j` - Vertical grid index (pixel)
    /// * `k` - Subpixel index within the grid cell
    ///
    /// # Returns
    /// A tuple of (longitude, latitude) in degrees
    pub fn subpixel_to_geo(&self, i: usize, j: usize, k: usize) -> (f64, f64) {
        // Normalize grid indices to [0.0, 1.0) range
        let norm_long = i as f64 / self.width_pixels as f64;
        let norm_lat = j as f64 / self.height_pixels as f64;

        // Convert to geographic coordinates for the grid cell corner
        let longitude_corner = norm_long * 360.0 - 180.0;
        let latitude_corner = norm_lat * 180.0 - 90.0;

        // Calculate longitude subdivisions, accounting for latitude-dependent distortion
        // (more subdivisions near equator, fewer near poles)
        let lon_subdivisions = (self.subpixel_divisions as f64 * latitude_corner.to_radians().cos()).max(1.0) as usize;

        // Extract the sub-coordinates from the k value
        let sub_lon = k / self.subpixel_divisions;
        let sub_lat = k % self.subpixel_divisions;

        // Calculate the next cell coordinates for interpolation
        let next_lon = (i + 1) as f64 / self.width_pixels as f64;
        let next_lat = (j + 1) as f64 / self.height_pixels as f64;
        let next_corner_lon = next_lon * 360.0 - 180.0;
        let next_corner_lat = next_lat * 180.0 - 90.0;

        // Interpolate to final geographic coordinates
        let longitude = longitude_corner + (sub_lon as f64 / lon_subdivisions as f64) * (next_corner_lon - longitude_corner) as f64;
        let latitude = latitude_corner + (sub_lat as f64 / self.subpixel_divisions as f64) * (next_corner_lat - latitude_corner) as f64;

        (longitude, latitude)
    }

    /// Converts geographic coordinates to a grid position (including subpixel)
    ///
    /// # Parameters
    /// * `longitude` - Longitude in degrees (-180 to 180)
    /// * `latitude` - Latitude in degrees (-90 to 90)
    ///
    /// # Returns
    /// A tuple of (i, j, k) representing (horizontal_pixel, vertical_pixel, subpixel_index)
    pub fn geo_to_subpixel(&self, longitude: f64, latitude: f64) -> (usize, usize, usize) {
        // Normalize geographic coordinates to [0.0, 1.0) range
        let norm_long = (longitude + 180.0) / 360.0;
        let norm_lat = (latitude + 90.0) / 180.0;

        // Calculate grid cell indices
        let i = (norm_long * self.width_pixels as f64) as usize % self.width_pixels;
        let j = (norm_lat * self.height_pixels as f64) as usize % self.height_pixels;

        // Calculate subpixel count for longitude, accounting for latitude (cosine effect)
        let lon_subdivisions = (self.subpixel_divisions as f64 * latitude.to_radians().cos()).max(1.0) as usize;

        // Calculate subpixel position
        let sub_i = i % lon_subdivisions;
        let sub_j = j % self.subpixel_divisions;

        // Combine into final subpixel index
        let k = sub_i * self.subpixel_divisions + sub_j;

        (i, j, k)
    }

    /// Converts geographic coordinates to gnomonic projection
    ///
    /// # Parameters
    /// * `lon` - Longitude in degrees
    /// * `lat` - Latitude in degrees
    /// * `center_lon` - Center longitude for projection (degrees)
    /// * `center_lat` - Center latitude for projection (degrees)
    /// * `planet_radius` - Radius of the planet (in arbitrary units, defaults to 1.0)
    ///
    /// # Returns
    /// (x, y) coordinates in the gnomonic projection
    pub fn geo_to_gnomonic(&self, lon: f64, lat: f64, center_lon: f64, center_lat: f64) -> (f64, f64) {
        // Convert to radians
        let lon_rad = lon.to_radians();
        let lat_rad = lat.to_radians();
        let center_lon_rad = center_lon.to_radians();
        let center_lat_rad = center_lat.to_radians();

        // Calculate the cosine of the angular distance
        let cos_c = lat_rad.sin() * center_lat_rad.sin() +
                    lat_rad.cos() * center_lat_rad.cos() * (lon_rad - center_lon_rad).cos();

        // Safety - ensure cos_c is not zero (or too close to zero)
        let cos_c = cos_c.max(0.01);

        // Calculate the gnomonic projection coordinates
        // For gnomonic projection on a sphere, the formula includes the planet radius
        let x = self.radius * lat_rad.cos() * (lon_rad - center_lon_rad).sin() / cos_c;
        let y = self.radius * (lat_rad.sin() * center_lat_rad.cos() -
                 lat_rad.cos() * center_lat_rad.sin() * (lon_rad - center_lon_rad).cos()) / cos_c;

        // The rendering scale can be adjusted by changing the planet_radius parameter
        (x, y)
    }
}
