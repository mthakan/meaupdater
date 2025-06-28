// src/model.rs

#[derive(Debug, PartialEq)]
pub enum UpdateType {
    Security,
    Software,
}

#[derive(Debug, PartialEq)]
pub struct PackageUpdate {
    pub name: String,
    pub current_version: String,
    pub new_version: String,
    pub update_type: UpdateType,
}
