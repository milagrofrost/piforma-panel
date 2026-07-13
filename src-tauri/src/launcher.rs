use std::{
    env, fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn clean_exec_command(exec: &str, name: &str) -> String {
    let mut command = exec.to_string();
    for token in ["%u", "%U", "%f", "%F", "%i", "%k"] {
        command = command.replace(token, "");
    }
    command = command.replace("%c", &shell_quote(name));
    remove_remaining_field_codes(&command).trim().to_string()
}

fn remove_remaining_field_codes(command: &str) -> String {
    let mut cleaned = String::new();
    let mut chars = command.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            cleaned.push(ch);
            continue;
        }
        match chars.peek() {
            Some('%') => {
                chars.next();
                cleaned.push('%');
            }
            Some(_) => {
                chars.next();
            }
            None => {}
        }
    }
    cleaned
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub fn resolve_desktop_dir(home: &str) -> Result<PathBuf, String> {
    if command_exists("xdg-user-dir") {
        if let Ok(path) = command_stdout("xdg-user-dir", &["DESKTOP"]) {
            let path = PathBuf::from(path);
            if path.is_dir() {
                return Ok(path);
            }
        }
    }
    let path = PathBuf::from(home).join("Desktop");
    fs::create_dir_all(&path)
        .map_err(|err| format!("failed to create {}: {err}", path.display()))?;
    Ok(path)
}

pub fn command_stdout(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|err| format!("failed to run {program}: {err}"))?;
    if !output.status.success() {
        return Err(format!("{program} exited with status {}", output.status));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|err| format!("{program} returned non-UTF-8 output: {err}"))
}

pub fn run_short_command(program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|err| format!("failed to run {program}: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with status {status}"))
    }
}

pub fn spawn_short_with_fallbacks(commands: &[Vec<String>]) -> Result<(), String> {
    let mut errors = Vec::new();
    for command in commands {
        if command.is_empty() {
            continue;
        }
        match Command::new(&command[0])
            .args(&command[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(_) => return Ok(()),
            Err(err) => errors.push(format!("{}: {err}", command.join(" "))),
        }
    }
    Err(errors.join("; "))
}

pub fn run_short_with_fallbacks(commands: &[Vec<String>]) -> Result<(), String> {
    let mut errors = Vec::new();
    for command in commands {
        if command.is_empty() {
            continue;
        }
        match Command::new(&command[0])
            .args(&command[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => errors.push(format!("{} exited with status {status}", command.join(" "))),
            Err(err) => errors.push(format!("{}: {err}", command.join(" "))),
        }
    }
    Err(errors.join("; "))
}

pub fn spawn_detached_with_fallbacks(commands: &[Vec<String>]) -> Result<(), String> {
    let mut errors = Vec::new();
    for command in commands {
        if command.is_empty() {
            continue;
        }
        if !command_exists(&command[0]) {
            errors.push(format!("{} not found", command[0]));
            continue;
        }
        match spawn_detached(&command[0], &command[1..]) {
            Ok(()) => return Ok(()),
            Err(err) => errors.push(err),
        }
    }
    Err(errors.join("; "))
}

pub fn spawn_detached_shell(command: &str) -> Result<(), String> {
    spawn_detached(
        "sh",
        &[
            "-c".to_string(),
            "exec sh -c \"$1\"".to_string(),
            "piforma-shell".to_string(),
            command.to_string(),
        ],
    )
}

pub fn spawn_detached(program: &str, args: &[String]) -> Result<(), String> {
    if command_exists("systemd-run") {
        let unit = unique_launch_unit_name();
        let mut command_args = vec![
            "--user".to_string(),
            "--scope".to_string(),
            "--collect".to_string(),
            "--quiet".to_string(),
            format!("--unit={unit}"),
        ];
        for key in [
            "DISPLAY",
            "XAUTHORITY",
            "WAYLAND_DISPLAY",
            "XDG_RUNTIME_DIR",
            "DBUS_SESSION_BUS_ADDRESS",
        ] {
            if let Ok(value) = env::var(key) {
                command_args.push(format!("--setenv={key}={value}"));
            }
        }
        command_args.push("--".to_string());
        command_args.push(program.to_string());
        command_args.extend(args.iter().cloned());
        match Command::new("systemd-run")
            .args(&command_args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(_) => return Ok(()),
            Err(err) => eprintln!("systemd-run detached launch failed: {err}"),
        }
    }

    if command_exists("setsid") {
        let mut command = Command::new("setsid");
        command.arg("-f").arg("--").arg(program).args(args);
        match command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(_) => return Ok(()),
            Err(err) => eprintln!("setsid detached launch failed: {err}"),
        }
    }

    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("failed to launch {program}: {err}"))
}

fn unique_launch_unit_name() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("piforma-launch-{}-{timestamp}", std::process::id())
}

pub fn command_exists(name: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path).any(|dir| dir.join(name).is_file())
}
