use super::RoutingAlgorithm;
use crate::instance::ContractInstance;
use crate::types::LoadBalancerError;
use std::sync::Arc;

/// Routes to the instance with the lowest current load score
pub struct LeastLoadedAlgorithm;

impl LeastLoadedAlgorithm {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LeastLoadedAlgorithm {
    fn default() -> Self {
        Self::new()
    }
}

impl RoutingAlgorithm for LeastLoadedAlgorithm {
    fn select(
        &self,
        instances: &[Arc<ContractInstance>],
    ) -> Result<Arc<ContractInstance>, LoadBalancerError> {
        instances
            .iter()
            .filter(|i| i.is_available())
            .min_by(|a, b| {
                a.load_score()
                    .partial_cmp(&b.load_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(Arc::clone)
            .ok_or(LoadBalancerError::NoHealthyInstances)
    }

    fn name(&self) -> &'static str {
        "least_loaded"
    }
}
