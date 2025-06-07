#[derive(Debug, Clone)]
pub struct Color {
    pub id: u32,
    pub name: String,
    pub hexadecimal: String,
}

impl Color {
    pub fn new(id: u32, name: &str, hexadecimal: &str) -> Color {
        Self {
            id,
            name: name.to_string(),
            hexadecimal: hexadecimal.to_string(),
        }
    }
}
