use ndarray::Array2;
use image::{DynamicImage, GenericImageView};

pub mod coordinates;
pub mod distance;
pub mod field;
pub mod sampling;

pub use distance::DistanceMethod;
pub use field::PixelField;

pub type Result<T> = std::result::Result<T, image::ImageError>;

/// Represents a geographic map with elevation data and coordinate conversion capabilities.
/// Handles transformation between geographic coordinates (latitude, longitude) and grid positions.
#[derive(Clone)]
pub struct Planisphere {
    /// Width of the map grid in pixels
    pub width_pixels: usize,
    /// Height of the map grid in pixels
    pub height_pixels: usize,
    /// Number of subpixel divisions for high-resolution mapping
    pub subpixel_divisions: usize,
    /// Radius of the Earth in meters, used for coordinate transformations
    pub radius: f64,
    pub mean_tile_size: f64, // Average size of a tile in meters
    /// Elevation data for each grid point
    pub(crate) elevation_grid: PixelField,
    /// Boolean mask indicating sea vs. land areas
    pub(crate) sea_mask: Array2<bool>,
    /// Optional source image containing elevation data
    pub(crate) elevation_map: Option<DynamicImage>,
    /// Red channel values normalized between 0.0 and 1.0
    pub(crate) red_channel: PixelField,
    /// Green channel values normalized between 0.0 and 1.0
    pub(crate) green_channel: PixelField,
    /// Blue channel values normalized between 0.0 and 1.0
    pub(crate) blue_channel: PixelField,
    /// Alpha channel values normalized between 0.0 and 1.0
    pub(crate) alpha_channel: PixelField,
}

impl Planisphere {
    /// Creates a new Planisphere with specified dimensions
    ///
    /// # Parameters
    /// * `width_pixels` - Number of horizontal grid points
    /// * `height_pixels` - Number of vertical grid points
    /// * `subpixel_divisions` - Number of subdivisions within each grid cell
    pub fn new(width_pixels: usize, height_pixels: usize, subpixel_divisions: usize) -> Self {
        Planisphere {
            width_pixels,
            height_pixels,
            subpixel_divisions,
            radius: 1.0,
            mean_tile_size: 0.0, // Default value, can be set later
            elevation_grid: PixelField::zeros(width_pixels, height_pixels),
            sea_mask: Array2::from_elem((width_pixels, height_pixels), false),
            elevation_map: None,
            red_channel: PixelField::zeros(width_pixels, height_pixels),
            green_channel: PixelField::zeros(width_pixels, height_pixels),
            blue_channel: PixelField::zeros(width_pixels, height_pixels),
            alpha_channel: PixelField::ones(width_pixels, height_pixels),
        }
    }

    pub fn set_radius(&mut self, radius: f64) {
        self.radius = radius;
        self.compute_mean_tile_size();
    }

    pub fn get_lon_subdivisons(&self, latitude: f64) -> usize {
        // Calculate the number of longitude subdivisions based on latitude
        (self.subpixel_divisions as f64 * latitude.to_radians().cos()).max(1.0) as usize
    }

    pub fn get_pixel_lon_subdivisions(&self, i: usize, j:usize) -> usize{
        let current_pixel_norm_lat = j as f64 / self.height_pixels as f64;
        let current_latitude = current_pixel_norm_lat * 180.0 - 90.0;
        (self.subpixel_divisions as f64 * current_latitude.to_radians().cos()).max(1.0) as usize
   }
    /// Creates a new Planisphere from an elevation map image
    ///
    /// # Parameters
    /// * `filename` - Path to the elevation map image
    /// * `subpixel_divisions` - Number of subdivisions within each grid cell
    ///
    /// # Returns
    /// * `Result<Self, image::ImageError>` - A new Planisphere with dimensions matching the image, or an error
    pub fn from_elevation_map(filename: &str, subpixel_divisions: usize) -> Result<Self> {
        let img = image::open(filename)?;
        let (width_pixels, height_pixels) = img.dimensions();
        println!("Loaded elevation map: {}x{}", width_pixels, height_pixels);
        let mut planisphere = Self::new(width_pixels as usize, height_pixels as usize, subpixel_divisions);
        planisphere.elevation_map = Some(img);

        // Initialize elevation grid and sea mask based on the image
        planisphere.process_elevation_data();
        println!("Processed elevation data for Planisphere ({}x{})", planisphere.width_pixels, planisphere.height_pixels);
        Ok(planisphere)
    }

    pub fn compute_mean_tile_size(&mut self) {
        let center_i = self.get_width_pixels() / 2;
        let center_j = self.get_height_pixels() / 2;

        // Calculate mean tile size for distance calculation
        let (lon1, lat1) = self.subpixel_to_geo(center_i, center_j, 0);
        let (lon2, lat2) = self.subpixel_to_geo(center_i, center_j, 1);
        let (world1_x, world1_y) = geo_to_gnomonic_helper(lon1, lat1, 0.0, 0.0, &self);
        let (world2_x, world2_y) = geo_to_gnomonic_helper(lon2, lat2, 0.0, 0.0, &self);
        self.mean_tile_size = ((world2_x - world1_x).abs() + (world2_y - world1_y).abs()) as f64;
    }

    /// Get the width in pixels
    pub fn get_width_pixels(&self) -> usize {
        self.width_pixels
    }

    /// Get the height in pixels
    pub fn get_height_pixels(&self) -> usize {
        self.height_pixels
    }

    /// Get the number of subpixel divisions
    pub fn get_subpixel_divisions(&self) -> usize {
        self.subpixel_divisions
    }

    /// Get the red channel data
    pub fn get_red_channel(&self) -> &PixelField {
        &self.red_channel
    }

    /// Get the green channel data
    pub fn get_green_channel(&self) -> &PixelField {
        &self.green_channel
    }

    /// Get the blue channel data
    pub fn get_blue_channel(&self) -> &PixelField {
        &self.blue_channel
    }

    /// Get the alpha channel data
    pub fn get_alpha_channel(&self) -> &PixelField {
        &self.alpha_channel
    }



    /// Gets the coordinates of a neighboring grid point with appropriate wrapping at map edges
    ///
    /// # Parameters
    /// * `x` - Current x coordinate
    /// * `y` - Current y coordinate
    /// * `dx` - X offset (-1, 0, or 1)
    /// * `dy` - Y offset (-1, 0, or 1)
    ///
    /// # Returns
    /// Coordinates of the neighbor, accounting for map edge wrapping
    pub(crate) fn get_neighbour(&self, x: usize, y: usize, dx: i32, dy: i32) -> (i32, i32) {
        let mut coords = (x as i32 + dx, y as i32 + dy);

        // Handle wrapping at top/bottom edges (latitude wrapping)
        // When we go over the poles, we flip to the opposite side of the map
        if coords.1 >= self.height_pixels as i32 {
            let overflow = coords.1 - self.height_pixels as i32 + 1;
            coords.1 = self.height_pixels as i32 - overflow;
            // Shift longitude 180 degrees when crossing poles
            coords.0 = coords.0 + self.width_pixels as i32 / 2;
        }
        if coords.1 < 0 {
            coords.1 = -coords.1;
            // Shift longitude 180 degrees when crossing poles
            coords.0 = coords.0 + self.width_pixels as i32 / 2;
        }

        // Handle wrapping at left/right edges (longitude wrapping)
        if coords.0 >= self.width_pixels as i32 {
            coords.0 = coords.0 - self.width_pixels as i32;
        }
        if coords.0 < 0 {
            coords.0 = self.width_pixels as i32 + coords.0;
        }

        coords
    }

    /// Gets the coordinates of a neighboring subpixel with appropriate wrapping at map and pixel edges
    ///
    /// This function handles both subpixel movement within a pixel and pixel boundary crossing,
    /// properly accounting for the planisphere's periodicity (wrapping at edges), and ensuring
    /// smooth transitions between pixels with different longitude subdivisions.
    ///
    /// # Parameters
    /// * `i` - Current pixel x coordinate
    /// * `j` - Current pixel y coordinate
    /// * `k` - Current subpixel index
    /// * `di` - Subpixel x offset (can be positive or negative)
    /// * `dj` - Subpixel y offset (can be positive or negative)
    ///
    /// # Returns
    /// A tuple (i, j, k) representing the coordinates of the neighbor subpixel
    pub fn get_neighbour_subpixel(&self, i: usize, j: usize, k: usize, di: i32, dj: i32) -> (usize, usize, usize) {
        // Extract subpixel coordinates
        let sub_i = k / self.subpixel_divisions;
        let sub_j = k % self.subpixel_divisions;

        // Get longitude subdivisions for the current pixel based on latitude
        let current_pixel_norm_lat = j as f64 / self.height_pixels as f64;
        let current_latitude = current_pixel_norm_lat * 180.0 - 90.0;
        let current_lon_subdivisions = (self.subpixel_divisions as f64 * current_latitude.to_radians().cos()).max(1.0) as usize;

        // Calculate new subpixel coordinates including overflow
        let new_sub_i = sub_i as i32 + di;
        let new_sub_j = sub_j as i32 + dj;

        // Calculate pixel displacement from subpixel overflow
        let mut pixel_di = 0;
        let mut pixel_dj = 0;
        let mut final_sub_i = new_sub_i;
        let mut final_sub_j = new_sub_j;

        // Handle pixel boundary crossing for i-direction
        if new_sub_i >= current_lon_subdivisions as i32 {
            pixel_di = 1; // Moving east
            final_sub_i = 0; // Always the leftmost column of the next pixel
        } else if new_sub_i < 0 {
            pixel_di = -1; // Moving west
            // For western movement, we need the rightmost column of the previous pixel
            // Get the longitude subdivisions of the western pixel
            let _west_i = if i == 0 { self.width_pixels - 1 } else { i - 1 };
            let west_lon_subdivisions = current_lon_subdivisions; // Same latitude, same subdivisions
            final_sub_i = (west_lon_subdivisions - 1) as i32; // Rightmost column
        }

        // Handle pixel boundary crossing for j-direction
        if new_sub_j >= self.subpixel_divisions as i32 {
            pixel_dj = 1; // Moving south
            final_sub_j = 0; // Top row of the southern pixel
        } else if new_sub_j < 0 {
            pixel_dj = -1; // Moving north
            final_sub_j = (self.subpixel_divisions - 1) as i32; // Bottom row of the northern pixel
        }

        // If we're not crossing a pixel boundary, just return the new subpixel index
        if pixel_di == 0 && pixel_dj == 0 {
            let new_k = (final_sub_i as usize) * self.subpixel_divisions + (final_sub_j as usize);
            return (i, j, new_k);
        }

        // Get neighboring pixel with proper wrapping at map edges
        let (wrapped_i, wrapped_j) = self.get_neighbour(i, j, pixel_di, pixel_dj);

        // Special handling for north/south transitions since longitude subdivisions may change
        if pixel_dj != 0 && pixel_di == 0 {
            // Get longitude subdivisions for the target pixel
            let target_pixel_norm_lat = wrapped_j as f64 / self.height_pixels as f64;
            let target_latitude = target_pixel_norm_lat * 180.0 - 90.0;
            let target_lon_subdivisions = (self.subpixel_divisions as f64 * target_latitude.to_radians().cos()).max(1.0) as usize;

            // Adjust the sub_i value to maintain relative position
            let target_sub_i = (sub_i * target_lon_subdivisions) / current_lon_subdivisions;
            final_sub_i = target_sub_i as i32;
        }

        // Combine into final subpixel index
        let new_k = (final_sub_i as usize) * self.subpixel_divisions + (final_sub_j as usize);

        (wrapped_i as usize, wrapped_j as usize, new_k)
    }

    /// Computes the latitude and longitude of the four corners of a subpixel
    ///
    /// # Parameters
    /// * `i` - Pixel x coordinate
    /// * `j` - Pixel y coordinate
    /// * `k` - Subpixel index
    ///
    /// # Returns
    /// An array of four (longitude, latitude) pairs representing the corners of the subpixel:
    /// [top-left, top-right, bottom-right, bottom-left] in clockwise order for proper polygon drawing
    pub fn get_subpixel_corners(&self, i: usize, j: usize, k: usize) -> [(f64, f64); 4] {
        // For visualization purposes, use the subpixel boundaries function
        // which handles the dateline crossing issue more consistently
        let sub_i = k / self.subpixel_divisions;
        let sub_j = k % self.subpixel_divisions;

        // Get the boundaries directly - this handles dateline crossing better for visualization
        let (left, right, top, bottom) = self.get_subpixel_boundaries(i, j, sub_i, sub_j);

        // Return the corners in CLOCKWISE order for proper polygon drawing
        [
            (left, top),       // Top-left
            (right, top),      // Top-right
            (right, bottom),   // Bottom-right
            (left, bottom)     // Bottom-left
        ]
    }

    /// Computes the latitude and longitude of the four corners of a pixel
    ///
    /// # Parameters
    /// * `i` - Pixel x coordinate
    /// * `j` - Pixel y coordinate
    ///
    /// # Returns
    /// An array of four (longitude, latitude) pairs representing the corners of the pixel:
    /// [top-left, top-right, bottom-right, bottom-left] in clockwise order for proper polygon drawing
    pub fn get_pixel_corners(&self, i: usize, j: usize) -> [(f64, f64); 4] {
        // For better consistency with visualization, use the pixel boundaries directly
        let (left, right, top, bottom) = self.get_pixel_boundaries(i, j);

        // Return the corners in CLOCKWISE order for proper polygon drawing
        [
            (left, top),       // Top-left
            (right, top),      // Top-right
            (right, bottom),   // Bottom-right
            (left, bottom)     // Bottom-left
        ]
    }

    /// Returns all subpixels in a rectangular region with their corner coordinates
    ///
    /// # Parameters
    /// * `min_i` - Minimum horizontal grid index
    /// * `max_i` - Maximum horizontal grid index
    /// * `min_j` - Minimum vertical grid index
    /// * `max_j` - Maximum vertical grid index
    ///
    /// # Returns
    /// A vector of tuples containing (i, j, k, corners) where corners is a 4-tuple of (lon, lat) pairs
    /// representing the four corners of each subpixel in the order: top-left, top-right, bottom-left, bottom-right
    pub fn get_subpixels_in_rectangle(&self, min_i: usize, max_i: usize, min_j: usize, max_j: usize)
        -> Vec<(usize, usize, usize, [(f64, f64); 4])> {
        let mut result = Vec::new();

        // Pre-allocate with approximate capacity
        let approx_subpixels_per_pixel = self.subpixel_divisions * self.subpixel_divisions;
        let approx_total = (max_i - min_i + 1) * (max_j - min_j + 1) * approx_subpixels_per_pixel;
        result.reserve(approx_total);

        // Process each pixel in the range
        for mut i in min_i..=max_i {
            if i >= self.width_pixels {
                i = i % self.width_pixels; // Wrap around at map edge
            }
            for j in min_j..=max_j {
                // Get the correct number of subpixels based on latitude
                let pixel_norm_lat = j as f64 / self.height_pixels as f64;
                let latitude_at_pixel = pixel_norm_lat * 180.0 - 90.0;
                let lon_subdivisions = (self.subpixel_divisions as f64 * latitude_at_pixel.to_radians().cos()).max(1.0) as usize;

                // Process each subpixel in the pixel
                for sub_i in 0..lon_subdivisions {
                    for sub_j in 0..self.subpixel_divisions {
                        // Calculate the k value
                        let k = sub_i * self.subpixel_divisions + sub_j;

                        // Get the corners of this subpixel
                        let corners = self.get_subpixel_corners(i, j, k);

                        // Add to the result
                        result.push((i, j, k, corners));
                    }
                }
            }
        }

        result
    }

    /// Calculates pixel geographic boundaries directly
    ///
    /// # Parameters
    /// * `i` - Horizontal pixel index
    /// * `j` - Vertical pixel index
    ///
    /// # Returns
    /// (left, right, top, bottom) geographic coordinates of the pixel boundaries
    pub fn get_pixel_boundaries(&self, i: usize, j: usize) -> (f64, f64, f64, f64) {
        let pixel_width = 360.0 / self.width_pixels as f64;
        let pixel_height = 180.0 / self.height_pixels as f64;

        let pixel_left = -180.0 + i as f64 * pixel_width;
        let pixel_right = pixel_left + pixel_width;
        let pixel_top = -90.0 + j as f64 * pixel_height;
        let pixel_bottom = pixel_top + pixel_height;

        // For visualization purposes, we won't cross the dateline
        // This ensures that rectangle edges don't cross each other in the visualization
        // Handle possible wrap-around at ±180°
        let (adjusted_left, adjusted_right) = if (pixel_left <= -180.0 && pixel_right >= -180.0) ||
                                              (pixel_left <= 180.0 && pixel_right >= 180.0) {
            // Keep coordinates in the same phase
            if i < self.width_pixels / 2 {
                // Western hemisphere - use negative coordinates
                let left = if pixel_left >= 0.0 { pixel_left - 360.0 } else { pixel_left };
                let right = if pixel_right > 0.0 { pixel_right - 360.0 } else { pixel_right };
                (left, right)
            } else {
                // Eastern hemisphere - use positive coordinates
                let left = if pixel_left < 0.0 { pixel_left + 360.0 } else { pixel_left };
                let right = if pixel_right <= 0.0 { pixel_right + 360.0 } else { pixel_right };
                (left, right)
            }
        } else {
            (pixel_left, pixel_right)
        };

        (adjusted_left, adjusted_right, pixel_top, pixel_bottom)
    }

    /// Returns subpixel geographic boundaries directly
    ///
    /// # Parameters
    /// * `i` - Horizontal pixel index
    /// * `j` - Vertical pixel index
    /// * `sub_i` - Horizontal subpixel index
    /// * `sub_j` - Vertical subpixel index
    ///
    /// # Returns
    /// (left, right, top, bottom) geographic coordinates of the subpixel boundaries
    pub fn get_subpixel_boundaries(&self, i: usize, j: usize, sub_i: usize, sub_j: usize) -> (f64, f64, f64, f64) {
        // Get pixel boundaries
        let (pixel_left, pixel_right, pixel_top, pixel_bottom) = self.get_pixel_boundaries(i, j);

        // Calculate latitude-dependent longitude subdivisions
        let pixel_norm_lat = j as f64 / self.height_pixels as f64;
        let latitude_at_pixel = pixel_norm_lat * 180.0 - 90.0;
        let lon_subdivisions = (self.subpixel_divisions as f64 * latitude_at_pixel.to_radians().cos()).max(1.0) as usize;

        // Calculate subpixel size
        let pixel_width = pixel_right - pixel_left;
        let pixel_height = pixel_bottom - pixel_top;
        let sub_width = pixel_width / lon_subdivisions as f64;
        let sub_height = pixel_height / self.subpixel_divisions as f64;

        // Calculate subpixel boundaries
        let sub_left = pixel_left + sub_i as f64 * sub_width;
        let sub_right = sub_left + sub_width;
        let sub_top = pixel_top + sub_j as f64 * sub_height;
        let sub_bottom = sub_top + sub_height;

        // For visualization consistency, subpixels should have coordinates in the
        // same hemisphere as their parent pixels
        let is_western_hemisphere = i < self.width_pixels / 2;

        // Handle wrap-around
        let (adjusted_left, adjusted_right) = if (sub_left <= -180.0 && sub_right >= -180.0) ||
                                         (sub_left <= 180.0 && sub_right >= 180.0) {
            // Crossing the dateline - keep consistent with parent pixel
            if is_western_hemisphere {
                // Western hemisphere - use negative coordinates
                let left = if sub_left >= 0.0 { sub_left - 360.0 } else { sub_left };
                let right = if sub_right > 0.0 { sub_right - 360.0 } else { sub_right };
                (left, right)
            } else {
                // Eastern hemisphere - use positive coordinates
                let left = if sub_left < 0.0 { sub_left + 360.0 } else { sub_left };
                let right = if sub_right <= 0.0 { sub_right + 360.0 } else { sub_right };
                (left, right)
            }
        } else {
            // Not crossing dateline, but ensure consistency with parent pixel
            if is_western_hemisphere && sub_left > 0.0 && sub_right > 0.0 {
                (sub_left - 360.0, sub_right - 360.0)
            } else if !is_western_hemisphere && sub_left < 0.0 && sub_right < 0.0 {
                (sub_left + 360.0, sub_right + 360.0)
            } else {
                (sub_left, sub_right)
            }
        };

        (adjusted_left, adjusted_right, sub_top, sub_bottom)
    }
}

/// Helper function for gnomonic projection
/// Converts geographic coordinates to world coordinates using gnomonic projection
pub fn geo_to_gnomonic_helper(lon: f64, lat: f64, center_lon: f64, center_lat: f64, planisphere: &Planisphere) -> (f64, f64) {
    let lon_rad = lon.to_radians();
    let lat_rad = lat.to_radians();
    let center_lon_rad = center_lon.to_radians();
    let center_lat_rad = center_lat.to_radians();

    let cos_c = lat_rad.sin() * center_lat_rad.sin() +
                lat_rad.cos() * center_lat_rad.cos() * (lon_rad - center_lon_rad).cos();

    let cos_c = cos_c.max(0.01); // Prevent division by zero

    let x = planisphere.radius * lat_rad.cos() * (lon_rad - center_lon_rad).sin() / cos_c;
    let y = planisphere.radius * (lat_rad.sin() * center_lat_rad.cos() -
             lat_rad.cos() * center_lat_rad.sin() * (lon_rad - center_lon_rad).cos()) / cos_c;

    (x, y)
}

/// Improved inverse gnomonic projection - converts world coordinates back to geographic coordinates
/// This version has better numerical stability and error handling
///
/// This function is the mathematical inverse of geo_to_gnomonic_helper, converting from
/// flat world coordinates back to spherical geographic coordinates (longitude, latitude).
///
/// **Parameters:**
/// - x, y: World coordinates in the flat projection plane
/// - center_lon, center_lat: Geographic center of the projection in degrees
/// - planet_radius: Radius of the planet for scaling calculations
///
/// **Returns:**
/// - (longitude, latitude) in degrees, or (NaN, NaN) if conversion fails
pub fn gnomonic_to_geo_helper(x: f64, y: f64, center_lon: f64, center_lat: f64, planet_radius: f64) -> (f64, f64) {
    // Convert degrees to radians for trigonometric calculations
    let center_lon_rad = center_lon.to_radians();
    let center_lat_rad = center_lat.to_radians();

    // Handle the special case where we're exactly at the projection center
    // Distance from center point (0,0) in world coordinates
    let distance_from_center = (x * x + y * y).sqrt();
    if distance_from_center < 1e-10 {
        return (center_lon, center_lat); // Return center coordinates directly
    }

    // Normalize coordinates by planet radius to get dimensionless values
    let x_norm = x / planet_radius;
    let y_norm = y / planet_radius;
    let rho = (x_norm * x_norm + y_norm * y_norm).sqrt(); // Distance from center in normalized space

    // Avoid numerical issues for very large distances (beyond projection validity)
    if rho > 10.0 {
        return (f64::NAN, f64::NAN); // Return invalid coordinates
    }

    // Calculate angular distance from projection center
    let c = rho.atan(); // Angular distance in radians
    let cos_c = c.cos();
    let sin_c = c.sin();

    // Calculate latitude using inverse gnomonic projection formulas
    // This is the mathematical inverse of the forward projection
    let lat_numerator = cos_c * center_lat_rad.sin() + (y_norm * sin_c * center_lat_rad.cos()) / rho;

    // Clamp to valid range [-1, 1] for asin to avoid NaN from floating point errors
    let lat_numerator_clamped = lat_numerator.clamp(-1.0, 1.0);
    let lat_rad = lat_numerator_clamped.asin(); // Convert back to latitude in radians

    // Calculate longitude with special case handling for numerical stability
    let lon_rad = if center_lat_rad.cos().abs() < 1e-10 {
        // Handle polar projection case (center latitude near ±90°)
        // At poles, longitude calculation becomes undefined, so use center longitude
        center_lon_rad
    } else {
        // Normal case: calculate longitude using inverse projection formula
        let denominator = rho * center_lat_rad.cos() * cos_c - y_norm * center_lat_rad.sin() * sin_c;
        if denominator.abs() < 1e-10 {
            // Handle case where denominator approaches zero (numerical instability)
            center_lon_rad
        } else {
            // Standard longitude calculation for gnomonic projection inverse
            center_lon_rad + (x_norm * sin_c / denominator).atan()
        }
    };

    // Convert back to degrees for return value
    let lon_degrees = lon_rad.to_degrees();
    let lat_degrees = lat_rad.to_degrees();

    // Final validity check to ensure coordinates are within valid geographic bounds
    if lon_degrees.is_finite() && lat_degrees.is_finite() &&
       lat_degrees >= -90.0 && lat_degrees <= 90.0 &&
       lon_degrees >= -180.0 && lon_degrees <= 180.0 {
        (lon_degrees, lat_degrees) // Return valid coordinates
    } else {
        (f64::NAN, f64::NAN) // Return invalid coordinates if out of bounds
    }
}
