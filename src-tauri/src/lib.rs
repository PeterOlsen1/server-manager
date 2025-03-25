use std::path::Path;
use std::{collections::HashMap, fs};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

mod servers;
use servers::{
    handle_server_run, kill_server
};


// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// define structs for reading YAML
#[derive(Debug, Serialize, Deserialize)]
struct Submission {
    submitters: Vec<Submitter>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Submitter {
    name: String,
    sid: String,
    email: String,
}

///
/// Return the current working directory
/// 
#[tauri::command]
fn pwd() -> String {
    std::env::current_dir()
        .expect("error while getting current directory")
        .to_string_lossy()
        .to_string()
}

///
/// List all files in the current directory.
/// Separate each file with a newline.
/// 
#[tauri::command]
fn ls() -> String {
    let mut result = String::new();
    for entry in std::fs::read_dir(".").expect("error while reading directory") {
        let entry = entry.expect("error while reading directory entry");
        result.push_str(&entry.file_name().to_string_lossy());
        result.push_str("\n");
    }

    result
}

///
/// List all directories in the current directory.
/// Each directory is separated by a newline.
/// 
#[tauri::command]
fn ls_directories() -> String {
    let mut result = String::new();
    for entry in std::fs::read_dir(".").expect("error while reading directory") {
        let entry = entry.expect("error while reading directory entry");
        if entry.file_type().expect("error while getting file type").is_dir() {
            result.push_str(&entry.file_name().to_string_lossy());
            result.push_str("\n");
        }
    }

    result
}

///
/// Change the current directory to the
/// one passed in.
/// 
/// There is minimal chance of error, except for
/// the current_dir() which shoudldn't fail.
///
#[tauri::command]
fn cd(name: &str) -> String {
    if name.len() == 0 {
        return "error: no directory provided".to_string();
    }

    // try to change directory
    match std::env::set_current_dir(name) {
        Err(e) => {
            return format!("error while changing directory: {}", e);
        }
        _ => {}
    };

    std::env::current_dir()
        .expect("error while getting current directory")
        .to_string_lossy()
        .to_string()
}

///
/// If we are currently in a directory with a submission_metadata.yml file,
/// read the file and return the contents as a JSON string.
/// 
#[tauri::command]
fn read_submission_dir() -> String {
    let filename = "submission_metadata.yml";
    if !Path::new(filename).exists() {
        return String::from("No submission metadata found!");
    }

    // read the file
    let ret: String;
    let content_result = fs::read_to_string(filename);
    let content: String;
    match content_result {
        Err(e) => return format!("Error reading submission metadata: {}", e),
        Ok(c) => {
            content = c;
        }
    }

    //this solution is a little janky. normal YAML files don't have a leading ":", so remove it here
    let fixed_content = content.replace(" :", " ");
    let submissions_result: Result<HashMap<String, Submission>, serde_yaml::Error> = serde_yaml::from_str(&fixed_content);
    let submissions: HashMap<String, Submission>;
    match submissions_result {
        Err(e) => return format!("Error parsing submission metadata: {}", e),
        Ok(s) => {
            submissions = s;
        }
    }

    // parse the result into a JSON string
    let json_result = serde_json::to_string_pretty(&submissions);
    match json_result {
        Err(e) => return format!("Error converting submission metadata to JSON: {}", e),
        Ok(j) => {
            ret = j;
        }
    }

    return ret;
}

/// Handle clicking on a student directory.
/// 
/// This should boot up a server for the given student ID
/// and return the process ID so that we can kill it later.
/// 
/// This could probably be done recursively to find
/// the server file, but this method just searches 2 levels of
/// directories. any more thanm that would be very weird
/// 
#[tauri::command]
async fn handle_student_click(submission_id: String, port: i32, app: AppHandle) -> String {
    let mut submission_path = format!("./submission_{}", submission_id);
    if !Path::new(&submission_path).exists() {
        return "No submission found for this student".to_string();
    }

    // read the directory
    let entries = match fs::read_dir(&submission_path) {
        Ok(entries) => entries,
        Err(_) => return "Error reading student submission directory".to_string(),
    };

    //read top level directory
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        //read in one extra directory level
        if entry.file_type().expect("Error reading submisison directory").is_dir() {            
            // read the directory inside of the submission directory
            submission_path = entry.path().to_string_lossy().to_string();
            let entries = match fs::read_dir(&submission_path) {
                Ok(entries) => entries,
                Err(_) => return "Error reading student submission directory".to_string(),
            };

            //read the entries. we want to find python or js file
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                // get the file name, just continue if it doesn't work, no need to throw error
                let file_name = entry.file_name();
                if file_name.is_empty() {
                    continue;
                }
                let file_name = file_name.to_str().unwrap_or_else(|| "file_name_error");

                //run the dang server (nobody should have python or js on their top level that isn't a server)
                if file_name.ends_with(".py") || file_name.ends_with(".js") && file_name.to_lowercase().contains("server") {
                    cd(&submission_path);

                    let process_id = handle_server_run(file_name.to_string(), port, app).await;

                    cd("../..");

                    // Return the process ID
                    return process_id;
                }
            } // end inner for
        }

        // get the file name, just continue if it doesn't work, no need to throw error
        let file_name = entry.file_name();
        if file_name.is_empty() {
            continue;
        }
        let file_name = file_name.to_str().unwrap();

        //run the dang server (nobody should have python or js on their top level that isn't a server)
        if (file_name.ends_with(".py") || file_name.ends_with(".js")) && file_name.to_lowercase().contains("server") {
            cd(&submission_path);

            let process_id = handle_server_run(file_name.to_string(), port, app).await;

            cd("..");

            // Return the process ID
            return process_id;
        }
    } // end outer for

    return "No server file found in student submission".to_string();
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            pwd, ls, cd, ls_directories, read_submission_dir, handle_student_click,
            kill_server
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
