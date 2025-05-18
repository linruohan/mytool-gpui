mod list;

pub struct ProjectData {
    pub name: String,
    pub description: String,
    pub icon: String,
    pub color: String,
    pub story: String,
}

pub struct ProjectStory {
    pub name: String,
    pub description: String,
    pub story: String,
}

impl ProjectStory {
    pub fn new(name: String, description: String, story: String) -> Self {
        Self {
            name,
            description,
            story,
        }
    }
}
