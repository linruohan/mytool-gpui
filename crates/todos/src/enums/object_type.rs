pub enum ObjectType {
    PROJECT,
    SECTION,
    ITEM,
    LABEL,
    TASK,
    TaskList,
    FILTER,
}
impl ObjectType {
    pub fn get_header(&self) -> &str {
        match self {
            ObjectType::PROJECT => "Projects",
            ObjectType::SECTION => "Sections",
            ObjectType::ITEM => "Tasks",
            ObjectType::LABEL => "Labels",
            ObjectType::TASK => "Filters",
            ObjectType::TaskList => "Tasks",
            ObjectType::FILTER => "Lists",
        }
    }
}
