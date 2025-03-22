use std::process::{Stdio, ChildStdout};

use tauri::{AppHandle, Emitter};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::task::spawn;
use tokio::process::{Command, Child};

pub fn handle_server_run(file_name: String, port: i32, app: AppHandle) -> String {
    //at this point we need to fork and exec the server
    //use python3 since i assume we are all grading on linux machines
    let child_result = if file_name.ends_with(".py") {
        run_python_server(file_name.to_string(), port)
    } else {
        Command::new("node")
            .arg(&file_name)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())
    };

    let mut child;
    match child_result {
        Err(e) => return format!("Error starting server: {}", e),
        Ok(c) => {child = c}
    }

    // get the process ID before the child is moved into the async block
    let process_id = match child.id() {
        Some(pid) => pid.to_string(),
        None => "Error getting process ID".to_string(),
    };

    //get stdout and stderr from the child
    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => return String::from("Error capturing stdout"),
    };
    let stderr = match child.stderr.take() {
        Some(s) => s,
        None => return String::from("Error capturing stderr"),
    };

    let app_clone = app.clone();
    spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            dbg!(&line); // Debug statement to trace stdout lines
            let _ = app_clone.emit("server-output", line);
        }
    });

    let app_clone = app.clone();
    spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            dbg!(&line); // Debug statement to trace stderr lines
            let _ = app_clone.emit("server-error", line);
        }
    });

    // Detach the process
    spawn(async move {
        let _ = child.wait().await; // Let process run in background
    });

    process_id
}


///
/// Run a python server
pub fn run_python_server(fname: String, port: i32) -> Result<Child, String> {
    //set up command to execute
    let mut command = Command::new("python3");
    command.arg("-u");
    command.arg(&fname);
    if port > 0 {
        command.arg(&port.to_string());
    }

    //capture stdout and stderr
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    //create child process
    let mut child = match command.spawn().map_err(|e| e.to_string()) {
        Ok(c) => c,
        Err(e) => return Err(String::from("Error creating child process")),
    };

    Ok(child)
}

#[tauri::command]
pub async fn kill_server(pid: u32) -> String {
    let output = Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .output()
        .await
        .expect("Error killing server");

    if output.status.success() {
        return "Server killed".to_string();
    } else {
        return "Error killing server".to_string();
    }
}