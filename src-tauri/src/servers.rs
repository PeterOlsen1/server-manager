use std::process::Stdio;
// use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::task::spawn;
use tokio::process::{Command, Child};

///
/// Handle starting a server
/// 
/// This function can handle either running python or node servers.
///
/// If the file name ends with ".py", a python server will be started.
/// If the file name ends with ".js", a node server will be started.
///
/// If the port number is > 0, the server will be started with the extra
/// "port" argument, which is basically just a number at the end of the
/// rest of the arguments.
/// 
/// Upon a successful start, event emitters in separate processes will
/// be spawned to:
/// * listen to stdout and emit to _server-output_
/// * listen to stderr and emit to _server-error_
/// 
/// These listeners will automatically die after the server is killed.
/// 
pub async fn handle_server_run(file_name: String, port: i32, app: AppHandle) -> String {
    //at this point we need to fork and exec the server
    //use python3 since i assume we are all grading on linux machines
    let child_result = if file_name.ends_with(".py") {
        run_python_server(file_name.to_string(), port)
    } else {
        run_js_server(file_name.to_string(), port).await
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

    //spawn tasks to listen to stdout and stderr
    let app_clone = app.clone();
    spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app_clone.emit("server-output", line);
        }
    });

    let app_clone = app.clone();
    spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app_clone.emit("server-error", line);
        }
    });

    process_id
}


///
/// Run a python server
/// 
/// This function runs the following command:
/// python3 -u <fname> <port?>
/// 
/// Upon error, an error string will be returned in the result
/// to indicate what went wrong.
/// 
/// On success, the child process will be returned.
/// 
pub fn run_python_server(fname: String, port: i32) -> Result<Child, String> {
    //set up command to execute
    let mut command = if cfg!(target_os = "windows") {
        Command::new("python") // Use "python" on Windows
    } else {
        Command::new("python3") // Use "python3" on Linux/macOS
    };
    command.arg("-u");
    command.arg(&fname);
    if port > 0 {
        command.arg(&port.to_string());
    }

    //capture stdout and stderr
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    //create child process
    let child = match command.spawn().map_err(|e| e.to_string()) {
        Ok(c) => c,
        Err(_) => return Err(String::from("Error creating child process")),
    };

    Ok(child)
}

///
/// Run a javascript server
/// 
/// This function runs the following commands:
/// npm i
/// node <fname> <port?>
/// 
/// Upon error, an error string will be returned in the result
/// to indicate what went wrong.
/// 
/// On success, the child process will be returned.
/// 
pub async fn run_js_server(fname: String, port: i32) -> Result<Child, String> {
    //install node modules
    let npm_i = Command::new("npm")
        .arg("i")
        .output();

    match npm_i.await {
        Ok(_) => (),
        Err(e) => return Err(format!("Error installing node modules: {}", e)),
    };
    
    //set up command to execute
    let mut command = Command::new("node");
    command.arg(&fname);
    if port > 0 {
        command.arg(&port.to_string());
    }

    //capture stdout and stderr
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    //create child process
    let child = match command.spawn().map_err(|e| e.to_string()) {
        Ok(c) => c,
        Err(_) => return Err(String::from("Error creating child process")),
    };

    Ok(child)
}

/// 
/// Kill a server running with a given process id, 
/// send kill -9 to ensure that is shuts down
/// 
/// Servers for homework 4 may have difficulties
/// shutting down because of the libraries that are
/// used, make sure to wait a few seconds after shutting
/// down a homework 4 server before starting a new one
/// 
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