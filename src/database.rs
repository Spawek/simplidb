use std::path::PathBuf;

#[derive(Debug)]
pub struct Database {
    pub datastores: Vec<Datastore>,
}

#[derive(Debug)]
pub struct Datastore {
    pub name: String,
    pub path: PathBuf,
    pub columns: Vec<Column>, // TODO: change to read data on-demand
}

#[derive(Debug, Clone)] // TODO: remove Copy/Clone
pub struct Column {
    pub name: String,
    pub data: Vec<String>,
}