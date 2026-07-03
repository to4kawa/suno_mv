use std::{path::Path, process::Command};

const ALLOWED_RESOLUTIONS: &[&str] = &["1280x720", "1920x1080"];
const ALLOWED_VISUALIZERS: &[&str] = &["spectrum", "showspectrum"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateRequest {
    pub url: String,
    pub base64: Option<String>,
    pub resolution: Option<String>,
    pub visualizer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateResponse {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub output_path: Option<String>,
}

pub fn extract_suno_id(url: &str) -> Option<String> {
    let marker = "/song/";
    let start = url.find(marker)? + marker.len();
    let id: String = url[start..]
        .chars()
        .take_while(|c| c.is_ascii_hexdigit() || *c == '-')
        .collect();
    if id.is_empty() { None } else { Some(id) }
}

pub fn validate_resolution(value: Option<&str>) -> Result<String, String> {
    validate_allow_list(value.unwrap_or("1280x720"), ALLOWED_RESOLUTIONS, "resolution")
}

pub fn validate_visualizer(value: Option<&str>) -> Result<String, String> {
    validate_allow_list(value.unwrap_or("spectrum"), ALLOWED_VISUALIZERS, "visualizer")
}

fn validate_allow_list(value: &str, allowed: &[&str], label: &str) -> Result<String, String> {
    if allowed.contains(&value) {
        Ok(value.to_string())
    } else {
        Err(format!("unsupported {label}: {value}"))
    }
}

pub fn build_ffmpeg_args(
    mp3_path: &Path,
    image_path: &Path,
    mp4_path: &Path,
    resolution: &str,
    visualizer: &str,
) -> Vec<String> {
    vec![
        "-y".into(),
        "-i".into(),
        mp3_path.to_string_lossy().into_owned(),
        "-loop".into(),
        "1".into(),
        "-i".into(),
        image_path.to_string_lossy().into_owned(),
        "-filter_complex".into(),
        format!("[0:a]showspectrum=s={resolution}:mode={visualizer}[spec];[1:v][spec]overlay=format=auto"),
        "-shortest".into(),
        "-c:v".into(),
        "libx264".into(),
        "-c:a".into(),
        "aac".into(),
        mp4_path.to_string_lossy().into_owned(),
    ]
}

pub fn run_ffmpeg(args: &[String]) -> Result<GenerateResponse, String> {
    let output = Command::new("ffmpeg")
        .args(args)
        .output()
        .map_err(|e| format!("failed to run ffmpeg: {e}"))?;

    Ok(GenerateResponse {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        output_path: args.last().cloned(),
    })
}

pub fn run() {
    println!("Suno MV Tauri backend checks are available. Launch with a Tauri runtime in a full desktop environment.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_suno_id() {
        assert_eq!(extract_suno_id("https://suno.com/song/abc123-def"), Some("abc123-def".into()));
        assert_eq!(extract_suno_id("https://example.com/nope"), None);
    }

    #[test]
    fn validates_allow_lists() {
        assert!(validate_resolution(Some("1920x1080")).is_ok());
        assert!(validate_resolution(Some("640x480;rm -rf /")).is_err());
        assert!(validate_visualizer(Some("spectrum")).is_ok());
        assert!(validate_visualizer(Some("bad")).is_err());
    }

    #[test]
    fn ffmpeg_args_are_not_shell_joined() {
        let args = build_ffmpeg_args(Path::new("a b.mp3"), Path::new("cover image.jpg"), Path::new("out file.mp4"), "1280x720", "spectrum");
        assert_eq!(args[0], "-y");
        assert!(args.contains(&"a b.mp3".to_string()));
        assert!(!args.join(" ").contains("\"a b.mp3\""));
    }
}
