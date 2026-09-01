use crate::common::bin::Bin;
use crate::common::sphere_spec::Sphere;
use crate::postprocessor::postprocessor_interface::Postprocessor;

pub struct SphereLastBinCleanupProcessor;

impl SphereLastBinCleanupProcessor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SphereLastBinCleanupProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Postprocessor<Sphere, Bin> for SphereLastBinCleanupProcessor {
    fn process(&self, _solution: &mut [Vec<Sphere>], _bin: &Bin) {
        // TODO: Implement sphere last bin cleanup logic
        // E.g., drop sphere centers down towards bottom surface accounting for radius
    }
}
