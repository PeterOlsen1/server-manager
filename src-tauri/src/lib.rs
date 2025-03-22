use std::path::Path;
use std::{collections::HashMap, fs};
use std::process::{Stdio};
use std::io::{BufRead};
use std::sync::{Arc};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use tokio::sync::mpsc::channel;
use tokio::sync::Mutex;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::task::spawn;
use tokio::sync::mpsc::Sender;
use tokio::process::Command;


// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

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

#[tauri::command]
fn pwd() -> String {
    std::env::current_dir()
        .expect("error while getting current directory")
        .to_string_lossy()
        .to_string()
}

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

#[tauri::command]
fn read_submission_dir() -> String {
    let filename = "submission_metadata.yml";
    if !Path::new(filename).exists() {
        return String::from("No submission metadata found!");
    }

    // read the file
    let mut ret = String::new();
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
#[tauri::command]
fn handle_student_click(submission_id: String, port: i32, app: AppHandle) -> String {
    let mut submission_path = format!("./submission_{}", submission_id);
    if !Path::new(&submission_path).exists() {
        return "No submission found for this student".to_string();
    }

    // read the directory
    let entries = match fs::read_dir(&submission_path) {
        Ok(entries) => entries,
        Err(_) => return "Error reading student submission directory".to_string(),
    };

    // only get the first entry, there should be only one folder
    let entry = match entries.into_iter().next() {
        Some(Ok(dir_entry)) => dir_entry,
        Some(Err(_)) | None => return format!("Could not find student submission in directory: {}", submission_path),
    };

    if !entry.file_type().expect("Error reading submisison directory").is_dir() {
        return format!("Student submission is not a directory: {}", entry.path().to_string_lossy());
    }

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
        let file_name = file_name.to_str().unwrap();

        //run the dang server (nobody should have python or js on their top level that isn't a server)
        if file_name.ends_with(".py") || file_name.ends_with(".js") {
            let file_path = entry.path().to_string_lossy().to_string();
            dbg!(&file_path);
            
            cd(&submission_path);
            dbg!(pwd());

            //at this point we need to fork and exec the server
            //use python3 since i assume we are all grading on linux machines
            let child_result = if file_name.ends_with(".py") {
                run_python_server(file_name.to_string(), port, app)
            } else {
                run_node_server(file_name.to_string(), port, app)
            };

            let child;
            match child_result {
                Err(e) => return format!("Error starting server: {}", e),
                Ok(c) => {child = c}
            }

            cd("../..");
            dbg!(pwd());

            // Return the process ID
            return child.id().to_string();
        }
    }

    return "No server file found in student submission".to_string();
}

#[tauri::command]
fn run_python_server(fname: String, port: i32, app: AppHandle) -> Result<std::process::Child, std::io::Error> {
    //set up command to execute
    let mut command = Command::new("python3");
    command.arg(&fname);
    if port > 0 {
        command.arg(&port.to_string());
    }
    //capture stdout
    command.stdout(Stdio::piped());

    //create child process
    let mut child = match command.spawn().map_err(|e| e.to_string()) {
        Ok(c) => c,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    };

    //if this wait works, that's bad!
    match child.try_wait() {
        Ok(Some(status)) => {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Process exited!"))
        }
        Ok(None) => {
            //process still running. good for a server
        }
        Err(e) => {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "Process errored: e"))
        }
    }
    
    //get stdout from the child
    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Error capturing stdout")),
    };

    //get stdout reader
    // let reader = BufReader::new(stdout);

    // create tauri channel
    // https://v2.tauri.app/develop/calling-frontend/#channels
    let (tx, mut rx) = channel::<String>(10);
    let tx_arc = Arc::new(Mutex::new(Some(tx)));

    // spawn task to read stdout and send to channel
    let tx_clone: Arc<Mutex<Option<Sender<String>>>> = Arc::clone(&tx_arc);
    spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            dbg!(&line);
            // Lock the Mutex and send the line
            let mut tx_guard = tx_clone.lock().await;
            if let Some(tx) = tx_guard.as_ref() {
                if tx.send(line).await.is_err() {
                    break; // Stop if the receiver is dropped
                }
            }
        }
        // Drop the sender to signal that no more messages will be sent
        tx_clone.lock().await.take();
    });

    // Spawn another task to emit messages to frontend
    let app_clone = app.clone();
    spawn(async move {
        while let Some(message) = rx.recv().await {
            let _ = app_clone.emit("python-server-output", message);
        }
    });

    Ok(child)
}

#[tauri::command]
async fn run_node_server(fname: String, port: i32, app: AppHandle) -> Result<(), String> {
    // Set up command to execute
    let mut command = Command::new("node");
    command.arg(&fname);
    if port > 0 {
        command.arg(&port.to_string());
    }
    // Capture stdout
    command.stdout(std::process::Stdio::piped());

    // Create child process
    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => return Err(format!("Failed to spawn process: {}", e)),
    };

    // Get stdout from the child
    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => return Err("Error capturing stdout".to_string()),
    };

    // Create tauri channel
    let (tx, mut rx) = channel::<String>(10);
    let tx_arc = Arc::new(Mutex::new(Some(tx)));

    // Spawn task to read stdout and send to channel
    let tx_clone = Arc::clone(&tx_arc);
    spawn(async move {
        let reader = BufReader::new(stdout); // Use tokio::process::ChildStdout directly
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            dbg!(&line);
            // Lock the Mutex and send the line
            let mut tx_guard = tx_clone.lock().await;
            if let Some(tx) = tx_guard.as_ref() {
                if tx.send(line).await.is_err() {
                    break; // Stop if the receiver is dropped
                }
            }
        }
        // Drop the sender to signal that no more messages will be sent
        tx_clone.lock().await.take();
    });

    // Spawn another task to emit messages to frontend
    let app_clone = app.clone();
    spawn(async move {
        while let Some(message) = rx.recv().await {
            let _ = app_clone.emit("node-server-output", message);
        }
    });

    Ok(child)
}

#[tauri::command]
fn kill_server(pid: u32) -> String {
    let output = Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .output()
        .expect("Error killing server");

    if output.status.success() {
        return "Server killed".to_string();
    } else {
        return "Error killing server".to_string();
    }
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
