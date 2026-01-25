pub struct DialogConfig {
    pub title: String,
    pub overlay: bool,
    pub keyboard: bool,
    pub overlay_closable: bool,
    pub save_label: String,
    pub cancel_label: String,
}

impl Default for DialogConfig {
    fn default() -> Self {
        Self {
            title: "Dialog".to_string(),
            overlay: true,
            keyboard: true,
            overlay_closable: true,
            save_label: "Save".to_string(),
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

    pub fn keyboard(mut self, keyboard: bool) -> Self {
        self.keyboard = keyboard;
        self
    }

    pub fn overlay_closable(mut self, overlay_closable: bool) -> Self {
        self.overlay_closable = overlay_closable;
        self
    }

    pub fn edit_mode(mut self) -> Self {
        self.save_label = "Save".to_string();
        self
    }

    pub fn add_mode(mut self) -> Self {
        self.save_label = "Add".to_string();
        self
    }
}
