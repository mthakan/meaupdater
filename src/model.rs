// src/model.rs

#[derive(Debug, PartialEq, Clone)]
pub enum UpdateType {
    Security,
    Software,
    Kernel,
}

#[derive(Debug, PartialEq, Clone)]
pub struct PackageUpdate {
    pub name: String,
    pub current_version: String,
    pub new_version: String,
    pub update_type: UpdateType,
    pub size: String, // Paket boyutu
}
