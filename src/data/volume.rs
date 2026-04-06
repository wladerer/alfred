use nalgebra::Matrix3;

/// 3D volumetric scalar field on a regular grid.
#[derive(Debug, Clone)]
pub struct VolumeGrid {
    pub lattice: Matrix3<f64>,
    pub dims: [usize; 3],
    /// Flattened 3D data in Fortran order (x fastest, then y, then z).
    pub data: Vec<f64>,
}

impl VolumeGrid {
    pub fn get(&self, ix: usize, iy: usize, iz: usize) -> f64 {
        self.data[ix + iy * self.dims[0] + iz * self.dims[0] * self.dims[1]]
    }

    pub fn min(&self) -> f64 {
        self.data.iter().cloned().fold(f64::INFINITY, f64::min)
    }

    pub fn max(&self) -> f64 {
        self.data.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    }

    pub fn mean(&self) -> f64 {
        self.data.iter().sum::<f64>() / self.data.len() as f64
    }

    pub fn std_dev(&self) -> f64 {
        let mean = self.mean();
        let var = self.data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / self.data.len() as f64;
        var.sqrt()
    }

    /// Detect a reasonable isovalue based on the data statistics.
    /// Adapts strategy for wavefunctions (has negative values) vs charge density.
    pub fn suggest_isovalue(&self) -> f64 {
        let has_negative = self.min() < 0.0;
        let std = self.std_dev();
        let mean = self.mean();

        if has_negative {
            let abs_max = self.min().abs().max(self.max().abs());
            let suggested = (2.0 * std).min(0.3 * abs_max);
            // Validate
            if suggested >= abs_max {
                if mean.abs() < std {
                    std
                } else {
                    mean.abs() + std
                }
            } else {
                suggested
            }
        } else {
            let suggested = (mean + 2.0 * std).min(0.5 * self.max());
            if suggested >= self.max() {
                mean + std
            } else {
                suggested
            }
        }
    }
}
