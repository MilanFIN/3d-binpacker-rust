pub trait Container: Clone + Send + Sync {
    fn volume(&self) -> f64;
    fn max_weight(&self) -> f32;
    fn current_weight(&self) -> f32;
    fn w(&self) -> f32;
    fn h(&self) -> f32;
    fn d(&self) -> f32;
}