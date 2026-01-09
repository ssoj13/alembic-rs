//! Multi-dimensional array support.
//!
//! Dimensions describe the shape of multi-dimensional array data.

use smallvec::SmallVec;

/// Dimensions of a multi-dimensional array.
/// 
/// Used to describe the shape of array samples that have more than
/// one dimension (e.g., 2D textures, 3D volumes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dimensions {
    /// Size of each dimension. Empty means scalar (rank 0).
    dims: SmallVec<[usize; 4]>,
}

impl Dimensions {
    /// Create scalar dimensions (rank 0).
    pub fn scalar() -> Self {
        Self { dims: SmallVec::new() }
    }
    
    /// Create 1D dimensions.
    pub fn d1(size: usize) -> Self {
        Self { dims: smallvec::smallvec![size] }
    }
    
    /// Create 2D dimensions.
    pub fn d2(width: usize, height: usize) -> Self {
        Self { dims: smallvec::smallvec![width, height] }
    }
    
    /// Create 3D dimensions.
    pub fn d3(width: usize, height: usize, depth: usize) -> Self {
        Self { dims: smallvec::smallvec![width, height, depth] }
    }
    
    /// Create from a slice of sizes.
    pub fn from_slice(sizes: &[usize]) -> Self {
        Self { dims: SmallVec::from_slice(sizes) }
    }
    
    /// Get the rank (number of dimensions).
    /// 
    /// - Rank 0: scalar
    /// - Rank 1: 1D array
    /// - Rank 2: 2D array (e.g., image)
    /// - Rank 3: 3D array (e.g., volume)
    #[inline]
    pub fn rank(&self) -> usize {
        self.dims.len()
    }
    
    /// Get the size of a specific dimension.
    /// 
    /// Returns None if the dimension index is out of range.
    pub fn size(&self, dim: usize) -> Option<usize> {
        self.dims.get(dim).copied()
    }
    
    /// Get all dimension sizes as a slice.
    pub fn sizes(&self) -> &[usize] {
        &self.dims
    }
    
    /// Get the total number of elements (product of all dimensions).
    pub fn num_points(&self) -> usize {
        if self.dims.is_empty() {
            1 // Scalar
        } else {
            self.dims.iter().product()
        }
    }
    
    /// Check if this represents a scalar (rank 0).
    #[inline]
    pub fn is_scalar(&self) -> bool {
        self.dims.is_empty()
    }
    
    /// Set the size of a dimension, extending if necessary.
    pub fn set_size(&mut self, dim: usize, size: usize) {
        while self.dims.len() <= dim {
            self.dims.push(1);
        }
        self.dims[dim] = size;
    }
    
    /// Add a new dimension at the end.
    pub fn push(&mut self, size: usize) {
        self.dims.push(size);
    }
    
    /// Set the rank, trimming or extending as needed.
    pub fn set_rank(&mut self, rank: usize) {
        self.dims.resize(rank, 1);
    }
}

impl From<usize> for Dimensions {
    fn from(size: usize) -> Self {
        Self::d1(size)
    }
}

impl From<(usize, usize)> for Dimensions {
    fn from((w, h): (usize, usize)) -> Self {
        Self::d2(w, h)
    }
}

impl From<(usize, usize, usize)> for Dimensions {
    fn from((w, h, d): (usize, usize, usize)) -> Self {
        Self::d3(w, h, d)
    }
}

impl From<Vec<usize>> for Dimensions {
    fn from(v: Vec<usize>) -> Self {
        Self { dims: SmallVec::from_vec(v) }
    }
}

impl std::fmt::Display for Dimensions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.dims.is_empty() {
            write!(f, "[]")
        } else {
            write!(f, "[")?;
            for (i, s) in self.dims.iter().enumerate() {
                if i > 0 {
                    write!(f, " x ")?;
                }
                write!(f, "{}", s)?;
            }
            write!(f, "]")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scalar() {
        let d = Dimensions::scalar();
        assert_eq!(d.rank(), 0);
        assert!(d.is_scalar());
        assert_eq!(d.num_points(), 1);
    }
    
    #[test]
    fn test_1d() {
        let d = Dimensions::d1(10);
        assert_eq!(d.rank(), 1);
        assert_eq!(d.size(0), Some(10));
        assert_eq!(d.num_points(), 10);
    }
    
    #[test]
    fn test_2d() {
        let d = Dimensions::d2(640, 480);
        assert_eq!(d.rank(), 2);
        assert_eq!(d.size(0), Some(640));
        assert_eq!(d.size(1), Some(480));
        assert_eq!(d.num_points(), 640 * 480);
        assert_eq!(format!("{}", d), "[640 x 480]");
    }
    
    #[test]
    fn test_3d() {
        let d = Dimensions::d3(64, 64, 64);
        assert_eq!(d.rank(), 3);
        assert_eq!(d.num_points(), 64 * 64 * 64);
    }
    
    #[test]
    fn test_from_conversions() {
        let d1: Dimensions = 100.into();
        assert_eq!(d1.rank(), 1);
        
        let d2: Dimensions = (800, 600).into();
        assert_eq!(d2.rank(), 2);
        
        let d3: Dimensions = (10, 20, 30).into();
        assert_eq!(d3.rank(), 3);
    }
    
    #[test]
    fn test_mutate() {
        let mut d = Dimensions::scalar();
        d.push(10);
        d.push(20);
        assert_eq!(d.rank(), 2);
        assert_eq!(d.sizes(), &[10, 20]);
        
        d.set_size(0, 5);
        assert_eq!(d.size(0), Some(5));
        
        d.set_rank(4);
        assert_eq!(d.rank(), 4);
        assert_eq!(d.sizes(), &[5, 20, 1, 1]);
    }
}
