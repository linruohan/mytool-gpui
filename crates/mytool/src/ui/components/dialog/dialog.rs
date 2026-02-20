#[derive(Clone)]
pub struct DialogConfig {
    pub title: String,
    pub overlay: bool,
    pub keyboard: bool,
    pub overlay_closable: bool,
    pub cancel_label: String,
}

impl Default for DialogConfig {
    fn default() -> Self {
        Self {
            title: "Dialog".to_string(),
            overlay: true,
            keyboard: true,
            overlay_closable: true,
            cancel_label: "Cancel".to_string(),
        }
    }
}

impl DialogConfig {
    pub fn new(title: &str) -> Self {
        Self { title: title.to_string(), ..Default::default() }
    }

    pub fn overlay(mut self, overlay: bool) -> Self {
        self.overlay = overlay;
        self
    }
}
