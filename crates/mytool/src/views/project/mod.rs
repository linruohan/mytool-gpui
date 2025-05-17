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
