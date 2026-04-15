use ndarray::Array2;
use std::ops::{Index, IndexMut};

/// A scalar field defined on the Planisphere's 2D pixel grid.
///
/// Every geographic raster layer (RGBA image channels, elevation, future derived
/// fields, …) is stored as a `PixelField` so they all share a common interface
/// and can be extended uniformly without touching the rest of the code.
///
/// The `[[i, j]]` indexing syntax works identically to a raw `Array2<f64>` thanks
/// to the `Index` / `IndexMut` implementations below.
#[derive(Clone)]
pub struct PixelField {
    pub(crate) data: Array2<f64>,
}

impl PixelField {
    /// All cells initialised to 0.0.
    pub fn zeros(width: usize, height: usize) -> Self {
        Self { data: Array2::zeros((width, height)) }
    }

    /// All cells initialised to 1.0.
    pub fn ones(width: usize, height: usize) -> Self {
        Self { data: Array2::ones((width, height)) }
    }

    /// All cells initialised to `value`.
    pub fn fill(width: usize, height: usize, value: f64) -> Self {
        Self { data: Array2::from_elem((width, height), value) }
    }
}

impl Index<[usize; 2]> for PixelField {
    type Output = f64;
    #[inline]
    fn index(&self, idx: [usize; 2]) -> &f64 {
        &self.data[idx]
    }
}

impl IndexMut<[usize; 2]> for PixelField {
    #[inline]
    fn index_mut(&mut self, idx: [usize; 2]) -> &mut f64 {
        &mut self.data[idx]
    }
}
