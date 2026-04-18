use super::{Planisphere, PixelField};
use image::GenericImageView;
use ndarray::Array2;

impl Planisphere {
    /// Processes the loaded elevation image to populate elevation_grid and sea_mask
    ///
    /// This should be called after loading an elevation map
    pub(super) fn process_elevation_data(&mut self) {
        if let Some(ref img) = self.elevation_map {
            // Reset grid sizes to match the image if needed
            let (width, height) = img.dimensions();
            if width as usize != self.width_pixels || height as usize != self.height_pixels {
                self.width_pixels = width as usize;
                self.height_pixels = height as usize;
                self.elevation_grid = PixelField::zeros(self.width_pixels, self.height_pixels);
                self.sea_mask = Array2::from_elem((self.width_pixels, self.height_pixels), false);
                self.red_channel = PixelField::zeros(self.width_pixels, self.height_pixels);
                self.green_channel = PixelField::zeros(self.width_pixels, self.height_pixels);
                self.blue_channel = PixelField::zeros(self.width_pixels, self.height_pixels);
                self.alpha_channel = PixelField::ones(self.width_pixels, self.height_pixels);
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
    pub fn load_elevation_map(&mut self, filename: &str) -> super::Result<()> {
        let img = image::open(filename)?;

        // Update dimensions to match the image
        let (width, height) = img.dimensions();
        self.width_pixels = width as usize;
        self.height_pixels = height as usize;

        // Reset data structures
        self.elevation_grid = PixelField::zeros(self.width_pixels, self.height_pixels);
        self.sea_mask = Array2::from_elem((self.width_pixels, self.height_pixels), false);
        self.red_channel = PixelField::zeros(self.width_pixels, self.height_pixels);
        self.green_channel = PixelField::zeros(self.width_pixels, self.height_pixels);
        self.blue_channel = PixelField::zeros(self.width_pixels, self.height_pixels);
        self.alpha_channel = PixelField::ones(self.width_pixels, self.height_pixels);

        // Store the image
        self.elevation_map = Some(img);

        // Process the image data
        self.process_elevation_data();

        Ok(())
    }

    /// Get RGBA values at specific pixel coordinates
    ///
    /// # Parameters
    /// * `i` - Horizontal pixel index
    /// * `j` - Vertical pixel index
    ///
    /// # Returns
    /// A tuple of (red, green, blue, alpha) values normalized between 0.0 and 1.0
    pub fn get_rgba_at_pixel(&self, i: i32, j: i32) -> (f64, f64, f64, f64) {
        let mut iout = i ;
        let mut jout = j ;
        let width = self.width_pixels as i32;
        let height = self.height_pixels as i32;

        if iout >= width {iout = iout -width-1;}
        if iout<0 {iout = width + iout;}
        if jout >= height { jout = height - (jout - height)-1; } 
        if jout < 0 { jout = -jout; } 


        (
            self.red_channel[[iout as usize, jout as usize]],
            self.green_channel[[iout as usize, jout as usize]],
            self.blue_channel[[iout as usize, jout as usize]],
            self.alpha_channel[[iout as usize, jout as usize]]
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
    pub fn get_rgba_at_subpixel(&self, i: i32, j: i32, k: usize) -> (f64, f64, f64, f64) {
        let rgba_sw = self.get_rgba_at_pixel(i,     j);
        let rgba_se = self.get_rgba_at_pixel(i + 1, j);
        let rgba_nw = self.get_rgba_at_pixel(i,     j + 1);
        let rgba_ne = self.get_rgba_at_pixel(i + 1, j + 1);

        let lon_divs = self.get_pixel_lon_subdivisions(i as usize, j as usize);
        let sub_i = k / self.get_subpixel_divisions();
        let sub_j = k % self.get_subpixel_divisions();

        // Fractional position of the subpixel centre within the parent pixel [0, 1)
        let tx = (sub_i as f64 + 0.5) / lon_divs as f64;
        let ty = (sub_j as f64 + 0.5) / self.get_subpixel_divisions() as f64;

        // Bilinear interpolation across the four neighbouring pixel centres
        let lerp = |sw: f64, se: f64, nw: f64, ne: f64| -> f64 {
            (1.0 - tx) * (1.0 - ty) * sw
                + tx   * (1.0 - ty) * se
                + (1.0 - tx) * ty   * nw
                + tx         * ty   * ne
        };

        (
            lerp(rgba_sw.0, rgba_se.0, rgba_nw.0, rgba_ne.0),
            lerp(rgba_sw.1, rgba_se.1, rgba_nw.1, rgba_ne.1),
            lerp(rgba_sw.2, rgba_se.2, rgba_nw.2, rgba_ne.2),
            lerp(rgba_sw.3, rgba_se.3, rgba_nw.3, rgba_ne.3),
        )
    }

    /// Derive a normalized altitude value (0.0–1.0) for a subpixel position.
    ///
    /// Returns a single value at the subpixel centre, suitable for texture selection.
    /// For per-corner mesh heights use [`get_alti_at_subpixel_corners`] instead.
    pub fn get_alti_at_subpixel(&self, i: i32, j: i32, k: usize) -> f32 {
        let (r, g, b, a) = self.get_rgba_at_subpixel(i, j, k);
        rgba_to_alti(r, g, b, a)
    }

    /// Returns the altitude at each of the 4 corners of a subpixel,
    /// in the same order as [`get_subpixel_corners`]: [top-left, top-right, bottom-right, bottom-left].
    ///
    /// Each corner is at a distinct fractional pixel-grid position, so the terrain
    /// mesh will have proper height variation instead of flat subpixel quads.
    pub fn get_altitude_at_subpixel_corners(&self, i: i32, j: i32, k: usize) -> [f32; 4] {
        let sub_i = k / self.subpixel_divisions;
        let sub_j = k % self.subpixel_divisions;
        let lon_divs = self.get_pixel_lon_subdivisions(i as usize, j as usize);

        let fi_left  = i as f64 +  sub_i      as f64 / lon_divs as f64;
        let fi_right = i as f64 + (sub_i + 1) as f64 / lon_divs as f64;
        // "top" and "bottom" match the naming in get_subpixel_corners:
        // top = south edge (sub_j), bottom = north edge (sub_j + 1)
        let fj_top    = j as f64 +  sub_j      as f64 / self.subpixel_divisions as f64;
        let fj_bottom = j as f64 + (sub_j + 1) as f64 / self.subpixel_divisions as f64;

        [
            self.alti_at_pixel_coords(fi_left,  fj_top),    // top-left
            self.alti_at_pixel_coords(fi_right, fj_top),    // top-right
            self.alti_at_pixel_coords(fi_right, fj_bottom), // bottom-right
            self.alti_at_pixel_coords(fi_left,  fj_bottom), // bottom-left
        ]
    }

    /// Bilinear interpolation of altitude at a continuous pixel-grid position `(fi, fj)`.
    fn alti_at_pixel_coords(&self, fi: f64, fj: f64) -> f32 {
        let i0 = fi.floor() as i32;
        let j0 = fj.floor() as i32;
        let tx = fi - fi.floor();
        let ty = fj - fj.floor();

        let sw = self.get_rgba_at_pixel(i0,     j0);
        let se = self.get_rgba_at_pixel(i0 + 1, j0);
        let nw = self.get_rgba_at_pixel(i0,     j0 + 1);
        let ne = self.get_rgba_at_pixel(i0 + 1, j0 + 1);

        let lerp = |sw: f64, se: f64, nw: f64, ne: f64| -> f64 {
            (1.0 - tx) * (1.0 - ty) * sw
                + tx   * (1.0 - ty) * se
                + (1.0 - tx) * ty   * nw
                + tx         * ty   * ne
        };

        rgba_to_alti(
            lerp(sw.0, se.0, nw.0, ne.0),
            lerp(sw.1, se.1, nw.1, ne.1),
            lerp(sw.2, se.2, nw.2, ne.2),
            lerp(sw.3, se.3, nw.3, ne.3),
        )
    }
}

/// Convert RGBA channel values to a normalized altitude scalar (0.0–1.0).
///
/// The alpha channel (inverted) acts as a global weight so that fully-transparent
/// pixels contribute zero altitude.  Red, green, and blue are blended with fixed
/// perceptual weights before the alpha scale is applied.
///
/// The result is normalized so that the maximum possible weighted sum equals 1.0.
pub fn rgba_to_alti(red: f64, green: f64, blue: f64, alpha: f64) -> f32 {
    const COEF_RED:   f64 = 0.5;
    const COEF_GREEN: f64 = 0.4;
    const COEF_BLUE:  f64 = 0.1;
    const COEF_ALPHA: f64 = 1.0;

    let inv_alpha  = 1.0 - alpha;
    let numerator  = COEF_ALPHA * inv_alpha * (COEF_RED * red + COEF_GREEN * green + COEF_BLUE * blue);
    let denominator = COEF_ALPHA * (COEF_RED + COEF_GREEN + COEF_BLUE);
    (numerator / denominator) as f32
}
