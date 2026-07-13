use crate::launcher::{
    command_exists, command_stdout, run_short_command, spawn_detached_with_fallbacks,
};
use serde::Serialize;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize)]
pub struct SystemStatus {
    pub ssid: Option<String>,
    pub internet_available: bool,
    pub volume: u8,
    pub audio_available: bool,
}

pub fn get_system_status() -> SystemStatus {
    let ssid = active_wifi_ssid();
    let internet_available = ssid.is_some() && internet_is_available();
    let volume = current_volume();

    SystemStatus {
        ssid,
        internet_available,
        volume: volume.unwrap_or(0),
        audio_available: volume.is_some(),
    }
}

pub fn set_system_volume(volume: u8) -> Result<(), String> {
    let volume = volume.min(100);

    if command_exists("wpctl") {
        run_short_command(
            "wpctl",
            &["set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{volume}%")],
        )?;
        return run_short_command(
            "wpctl",
            &[
                "set-mute",
                "@DEFAULT_AUDIO_SINK@",
                if volume == 0 { "1" } else { "0" },
            ],
        );
    }

    if command_exists("pactl") {
        run_short_command(
            "pactl",
            &["set-sink-volume", "@DEFAULT_SINK@", &format!("{volume}%")],
        )?;
        return run_short_command(
            "pactl",
            &[
                "set-sink-mute",
                "@DEFAULT_SINK@",
                if volume == 0 { "1" } else { "0" },
            ],
        );
    }

    if command_exists("amixer") {
        let volume_arg = format!("{volume}%");
        let mute_arg = if volume == 0 { "mute" } else { "unmute" };
        if run_short_command("amixer", &["-q", "sset", "Master", &volume_arg, mute_arg]).is_ok() {
            return Ok(());
        }
        return run_short_command("amixer", &["-q", "sset", "PCM", &volume_arg, mute_arg]);
    }

    Err("no supported system volume tool found; expected wpctl, pactl, or amixer".to_string())
}

pub fn open_network_settings() -> Result<(), String> {
    spawn_detached_with_fallbacks(&[
        vec!["nm-connection-editor".to_string()],
        vec!["connman-gtk".to_string()],
        vec!["wicd-client".to_string()],
        vec![
            "x-terminal-emulator".to_string(),
            "-e".to_string(),
            "nmtui".to_string(),
        ],
        vec!["lxterminal".to_string(), "-e".to_string(), "nmtui".to_string()],
    ])
}

fn active_wifi_ssid() -> Option<String> {
    if command_exists("nmcli") {
        if let Ok(output) = command_stdout("nmcli", &["-t", "-f", "ACTIVE,SSID", "dev", "wifi"]) {
            for line in output.lines() {
                if let Some(ssid) = line.strip_prefix("yes:") {
                    let ssid = unescape_nmcli_value(ssid.trim());
                    if !ssid.is_empty() {
                        return Some(ssid);
                    }
                }
            }
        }
    }

    if command_exists("iwgetid") {
        if let Ok(ssid) = command_stdout("iwgetid", &["-r"]) {
            let ssid = ssid.trim();
            if !ssid.is_empty() {
                return Some(ssid.to_string());
            }
        }
    }

    None
}

fn internet_is_available() -> bool {
    if command_exists("nmcli") {
        if let Ok(connectivity) =
            command_stdout("nmcli", &["-t", "networking", "connectivity", "check"])
        {
            match connectivity.trim().to_ascii_lowercase().as_str() {
                "full" => return true,
                "limited" | "portal" | "none" => return false,
                _ => {}
            }
        }
    }

    if command_exists("ping") {
        return Command::new("ping")
            .args(["-c", "1", "-W", "1", "1.1.1.1"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
    }

    false
}

fn current_volume() -> Option<u8> {
    if command_exists("wpctl") {
        if let Ok(output) = command_stdout("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"]) {
            if let Some(value) = parse_wpctl_volume(&output) {
                return Some(value);
            }
        }
    }

    if command_exists("pactl") {
        if let Ok(output) = command_stdout("pactl", &["get-sink-volume", "@DEFAULT_SINK@"]) {
            if let Some(value) = parse_first_percent(&output) {
                return Some(value);
            }
        }
    }

    if command_exists("amixer") {
        for control in ["Master", "PCM"] {
            if let Ok(output) = command_stdout("amixer", &["get", control]) {
                if let Some(value) = parse_first_percent(&output) {
                    return Some(value);
                }
            }
        }
    }

    None
}

fn parse_wpctl_volume(output: &str) -> Option<u8> {
    let value = output
        .split_whitespace()
        .find_map(|token| token.parse::<f64>().ok())?;
    Some((value * 100.0).round().clamp(0.0, 100.0) as u8)
}

fn parse_first_percent(output: &str) -> Option<u8> {
    for (index, ch) in output.char_indices() {
        if ch != '%' {
            continue;
        }
        let prefix = &output[..index];
        let digits_reversed: String = prefix
            .chars()
            .rev()
            .take_while(|ch| ch.is_ascii_digit())
            .collect();
        if digits_reversed.is_empty() {
            continue;
        }
        let digits: String = digits_reversed.chars().rev().collect();
        if let Ok(value) = digits.parse::<u8>() {
            return Some(value.min(100));
        }
    }
    None
}

fn unescape_nmcli_value(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            result.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else {
            result.push(ch);
        }
    }
    if escaped {
        result.push('\\');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wpctl_volume() {
        assert_eq!(parse_wpctl_volume("Volume: 0.55"), Some(55));
        assert_eq!(parse_wpctl_volume("Volume: 1.00 [MUTED]"), Some(100));
    }

    #[test]
    fn parses_percentage_output() {
        assert_eq!(parse_first_percent("front-left: 32768 / 50% / -18.06 dB"), Some(50));
        assert_eq!(parse_first_percent("Mono: Playback 74 [74%]"), Some(74));
    }

    #[test]
    fn unescapes_nmcli_ssid() {
        assert_eq!(unescape_nmcli_value("Green\\:Ice"), "Green:Ice");
    }
}
