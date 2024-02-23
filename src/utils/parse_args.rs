use serde::{Serialize, Deserialize};
use serde_json::{Value};
use std::fs::File;
use std::error::Error;
use std::io::{self, BufReader};
use rand::{thread_rng, seq::SliceRandom};
use std::{iter,fs, env};
use std::path::{Path, PathBuf};
use project_root::get_project_root;

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiInfo{
    pub user: String,
    pub token: String 
}

fn get_file_path(path_elements: Vec<&str>) -> String{
    let mut path = get_project_root().unwrap();
    path.extend(path_elements.iter());
    path.to_string_lossy().to_string()
}

pub fn get_api_tokens() -> Result<ApiInfo, Box<dyn Error>>{
    let api_config_path = get_file_path(vec!["configs","api.json"]);

    // Read the contents of the file
    let file = File::open(api_config_path.clone()).expect(&format!("Unable to open file at path {api_config_path}"));

    // Parse the JSON into a Config struct
    let api_config: ApiInfo = serde_json::from_reader(&file)?;
    Ok(api_config)
}

pub fn get_lorem_ipsum() -> String{
    let path = get_file_path(vec!["assets","lorem_ipsum.txt"]);
    // Read the contents of the file
    let mut content = fs::read_to_string(path.clone()).expect(&format!("Unable to open file at path {}",path));
    let snippets = content.split(".").collect::<Vec<&str>>();
    snippets.choose(&mut rand::thread_rng()).unwrap().to_string()
}