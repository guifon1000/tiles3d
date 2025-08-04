use ndarray::Array2;
use image::{DynamicImage, GenericImageView};

pub type Result<T> = std::result::Result<T, image::ImageError>;

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

/// Represents a geographic map with elevation data and coordinate conversion capabilities.
/// Handles transformation between geographic coordinates (latitude, longitude) and grid positions.
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
    elevation_grid: Array2<f64>,
    /// Boolean mask indicating sea vs. land areas
    sea_mask: Array2<bool>,
    /// Optional source image containing elevation data
    elevation_map: Option<DynamicImage>,
    /// Red channel values normalized between 0.0 and 1.0
    red_channel: Array2<f64>,
    /// Green channel values normalized between 0.0 and 1.0
    green_channel: Array2<f64>,
    /// Blue channel values normalized between 0.0 and 1.0
    blue_channel: Array2<f64>,
    /// Alpha channel values normalized between 0.0 and 1.0
    alpha_channel: Array2<f64>,
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
            elevation_grid: Array2::<f64>::zeros((width_pixels, height_pixels)),
            sea_mask: Array2::from_elem((width_pixels, height_pixels), false),
            elevation_map: None,
            red_channel: Array2::<f64>::zeros((width_pixels, height_pixels)),
            green_channel: Array2::<f64>::zeros((width_pixels, height_pixels)),
            blue_channel: Array2::<f64>::zeros((width_pixels, height_pixels)),
            alpha_channel: Array2::<f64>::ones((width_pixels, height_pixels)),
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
        eprintln!("Loaded elevation map: {}x{}", width_pixels, height_pixels);
        let mut planisphere = Self::new(width_pixels as usize, height_pixels as usize, subpixel_divisions);
        eprintln!("Creating Planisphere with dimensions: {}x{}", planisphere.width_pixels, planisphere.height_pixels);
        planisphere.elevation_map = Some(img);
        
        // Initialize elevation grid and sea mask based on the image
        planisphere.process_elevation_data();
        eprintln!("Processed elevation data for Planisphere");
        Ok(planisphere)
    }
    
    /// Processes the loaded elevation image to populate elevation_grid and sea_mask
    ///
    /// This should be called after loading an elevation map
    fn process_elevation_data(&mut self) {
        if let Some(ref img) = self.elevation_map {
            // Reset grid sizes to match the image if needed
            let (width, height) = img.dimensions();
            if width as usize != self.width_pixels || height as usize != self.height_pixels {
                self.width_pixels = width as usize;
                self.height_pixels = height as usize;
                self.elevation_grid = Array2::<f64>::zeros((self.width_pixels, self.height_pixels));
                self.sea_mask = Array2::from_elem((self.width_pixels, self.height_pixels), false);
                self.red_channel = Array2::<f64>::zeros((self.width_pixels, self.height_pixels));
                self.green_channel = Array2::<f64>::zeros((self.width_pixels, self.height_pixels));
                self.blue_channel = Array2::<f64>::zeros((self.width_pixels, self.height_pixels));
                self.alpha_channel = Array2::<f64>::ones((self.width_pixels, self.height_pixels));
            }
            
            // === DUAL IMAGE PROCESSING FOR TERRAIN SYSTEM ===
            // The source image (sphere_texture.png) serves two purposes:
            // 1. ELEVATION DATA: Grayscale values determine terrain height
            // 2. TEXTURE DATA: RGBA color values determine which textures to apply
            
            // Convert image to grayscale for elevation/height information
            let gray_img = img.to_luma8();
            
            // Convert image to RGBA for texture selection color data
            // Each RGBA pixel will drive terrain texture selection via select_texture_from_rgba()
            let rgba_img = img.to_rgba8();
            
            // === PROCESS EACH PIXEL FOR BOTH ELEVATION AND TEXTURE DATA ===
            // Fill the elevation grid, sea mask, and RGBA color channels simultaneously
            for y in 0..self.height_pixels {
                for x in 0..self.width_pixels {
                    // === COORDINATE SYSTEM CONVERSION ===
                    // Convert from standard image coordinates (top-left origin) to geographic coordinates (bottom-left origin)
                    // Geographic convention: (0,0) is bottom-left (South Pole, West longitude)
                    // Image convention: (0,0) is top-left
                    // So planisphere y=0 (South Pole) reads from image bottom (height-1-0)
                    // and planisphere y=height-1 (North Pole) reads from image top (height-1-(height-1) = 0)
                    let image_y = (self.height_pixels - 1 - y) as u32;
                    
                    // Extract elevation data from grayscale value
                    let pixel_value = gray_img.get_pixel(x as u32, image_y).0[0] as f64;
                    
                    // Extract RGBA color data for texture selection
                    let rgba_pixel = rgba_img.get_pixel(x as u32, image_y).0;
                    
                    // === ELEVATION PROCESSING ===
                    // Normalize elevation from 0-255 pixel values to 0.0-1.0 range
                    let normalized_elevation = pixel_value / 255.0;
                    self.elevation_grid[[x, y]] = normalized_elevation;
                    
                    // Create sea/land classification for various game systems
                    // Threshold of 0.3 means pixels darker than ~76 (out of 255) are considered water
                    self.sea_mask[[x, y]] = normalized_elevation < 0.3;
                    
                    // === TEXTURE DATA PROCESSING ===
                    // Store RGBA color values that will be used by select_texture_from_rgba()
                    // These values are normalized to 0.0-1.0 range for consistent processing
                    // Each channel can encode different terrain information:
                    
                    // RED channel: Currently used for primary texture selection
                    self.red_channel[[x, y]] = rgba_pixel[0] as f64 / 255.0;
                    
                    // GREEN channel: Available for secondary terrain classification (unused)
                    self.green_channel[[x, y]] = rgba_pixel[1] as f64 / 255.0;
                    
                    // BLUE channel: Available for tertiary terrain classification (unused)
                    self.blue_channel[[x, y]] = rgba_pixel[2] as f64 / 255.0;
                    
                    // ALPHA channel: Available for special effects/blending (unused)
                    self.alpha_channel[[x, y]] = rgba_pixel[3] as f64 / 255.0;
                }
            }
        }
    }

    /// Enhanced load_elevation_map method that also processes the data immediately
    ///
    /// # Parameters
    /// * `filename` - Path to the elevation map image
    /// 
    /// # Returns
    /// * `Result<(), image::ImageError>` - Success or error loading the image
    pub fn load_elevation_map(&mut self, filename: &str) -> Result<()> {
        let img = image::open(filename)?;
        
        // Update dimensions to match the image
        let (width, height) = img.dimensions();
        self.width_pixels = width as usize;
        self.height_pixels = height as usize;
        
        // Reset data structures
        self.elevation_grid = Array2::<f64>::zeros((self.width_pixels, self.height_pixels));
        self.sea_mask = Array2::from_elem((self.width_pixels, self.height_pixels), false);
        self.red_channel = Array2::<f64>::zeros((self.width_pixels, self.height_pixels));
        self.green_channel = Array2::<f64>::zeros((self.width_pixels, self.height_pixels));
        self.blue_channel = Array2::<f64>::zeros((self.width_pixels, self.height_pixels));
        self.alpha_channel = Array2::<f64>::ones((self.width_pixels, self.height_pixels));
        
        // Store the image
        self.elevation_map = Some(img);
        
        // Process the image data
        self.process_elevation_data();

        Ok(())
        
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
    pub fn get_red_channel(&self) -> &Array2<f64> {
        &self.red_channel
    }
    
    /// Get the green channel data
    pub fn get_green_channel(&self) -> &Array2<f64> {
        &self.green_channel
    }
    
    /// Get the blue channel data
    pub fn get_blue_channel(&self) -> &Array2<f64> {
        &self.blue_channel
    }
    
    /// Get the alpha channel data
    pub fn get_alpha_channel(&self) -> &Array2<f64> {
        &self.alpha_channel
    }
    
    /// Get RGBA values at specific pixel coordinates
    ///
    /// # Parameters
    /// * `i` - Horizontal pixel index
    /// * `j` - Vertical pixel index
    ///
    /// # Returns
    /// A tuple of (red, green, blue, alpha) values normalized between 0.0 and 1.0
    pub fn get_rgba_at_pixel(&self, i: usize, j: usize) -> (f64, f64, f64, f64) {
        if i >= self.width_pixels || j >= self.height_pixels {
            // Return default values for out-of-bounds coordinates
            return (0.0, 0.0, 0.0, 1.0);
        }
        
        (
            self.red_channel[[i, j]],
            self.green_channel[[i, j]],
            self.blue_channel[[i, j]],
            self.alpha_channel[[i, j]]
        )
    }
    
    /// Get RGBA values at specific subpixel coordinates
    /// Since subpixels within a pixel share the same color data, this returns the parent pixel's RGBA values
    ///
    /// # Parameters
    /// * `i` - Horizontal pixel index
    /// * `j` - Vertical pixel index
    /// * `k` - Subpixel index (unused for color data, but kept for consistency)
    ///
    /// # Returns
    /// A tuple of (red, green, blue, alpha) values normalized between 0.0 and 1.0
    pub fn get_rgba_at_subpixel(&self, i: usize, j: usize, _k: usize) -> (f64, f64, f64, f64) {
        self.get_rgba_at_pixel(i, j)
    }

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
    fn get_neighbour(&self, x: usize, y: usize, dx: i32, dy: i32) -> (i32, i32) {
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
                eprint!("Warning: i index out of bounds: {} >= {}\n", i, self.width_pixels);
                i= i%self.width_pixels; // Avoid out-of-bounds access
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
    pub fn get_subpixels_by_rectangular_distanceXXXXX(&self, center_i: usize, center_j: usize, center_k: usize, max_subpixel_distance: usize) 
        -> Vec<(usize, usize, usize, [(f64, f64); 4])> {
        let mut result = Vec::new();
        
        // Calculate how many pixels we need to include in each direction
        let pixel_radius = (max_subpixel_distance / self.subpixel_divisions) + 1;
        
        // Get the grid of pixels centered on the player's pixel
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
        
        // Use Chebyshev distance (max of dx, dy) for rectangular pattern
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
            
            // Calculate Chebyshev distance (rectangular pattern)
            let dx = (continuous_i - center_continuous_i).abs();
            let dy = (continuous_j - center_continuous_j).abs();
            let chebyshev_distance = dx.max(dy);
            
            // Include if within the maximum subpixel distance
            //if chebyshev_distance <= max_subpixel_distance as f64 {
                result.push((i, j, k, corners));
            //}
        }
        
        result
    }
    


pub fn get_subpixels_by_rectangular_distance(&self, center_i: usize, center_j: usize, center_k: usize, max_subpixel_distance: usize) 
        -> Vec<(usize, usize, usize, [(f64, f64); 4])> {
        let mut result = Vec::new();
        let (longitude, latitude) = self.subpixel_to_geo(center_i, center_j, center_k);
        // Calculate how many pixels we need to include in each direction
        let pixel_radius_y = (max_subpixel_distance / self.subpixel_divisions) + 1;
        let pixel_radius_x = (max_subpixel_distance / self.get_lon_subdivisons(latitude)) + 1;
        
        // Get the grid of pixels centered on the player's pixel
        let min_i = if center_i > pixel_radius_x { center_i - pixel_radius_x } else { 0 };
        let max_i = center_i + pixel_radius_x;
        let min_j = if center_j > pixel_radius_y { center_j - pixel_radius_y } else { 0 };
        let max_j = std::cmp::min(center_j + pixel_radius_y, self.height_pixels - 1);
        
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
        
        // Use Chebyshev distance (max of dx, dy) for rectangular pattern
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
            
            // Calculate Chebyshev distance (rectangular pattern)
            let dx = (continuous_i - center_continuous_i).abs();
            let dy = (continuous_j - center_continuous_j).abs();
            let chebyshev_distance = dx.max(dy);
            
            // Include if within the maximum subpixel distance
            //if chebyshev_distance <= max_subpixel_distance as f64 {
                result.push((i, j, k, corners));
            //}
        }
        
        result
    }











    /// Get subpixels using the specified distance calculation method
    /// This provides a unified interface for different selection patterns
    pub fn  get_subpixels_by_distance_method(
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