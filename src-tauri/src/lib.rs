use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Manager};

const ALLOWED_RESOLUTIONS: &[&str] = &["1280x720", "1920x1080", "1080x1080"];
const ALLOWED_VISUALIZERS: &[&str] = &["combined", "separate", "single"];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRequest {
    pub url: String,
    pub base64: Option<String>,
    pub resolution: Option<String>,
    pub visualizer: Option<String>,
    pub output_dir: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
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
        .take_while(|ch| ch.is_ascii_hexdigit() || *ch == '-')
        .collect();

    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

fn ensure_allowed(value: &str, allowed: &[&str], field_name: &str) -> Result<(), String> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(format!(
            "Invalid {field_name}: {value}. Allowed values: {}",
            allowed.join(", ")
        ))
    }
}

fn safe_id(id: &str) -> String {
    id.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect()
}

fn download_to_file(url: &str, path: &Path) -> Result<(), String> {
    let response = ureq::get(url).call().map_err(|err| err.to_string())?;
    let mut reader = response.into_reader();
    let mut file = fs::File::create(path).map_err(|err| err.to_string())?;
    std::io::copy(&mut reader, &mut file).map_err(|err| err.to_string())?;
    Ok(())
}

fn write_base64_image(data_url: &str, path: &Path) -> Result<(), String> {
    let payload = data_url
        .strip_prefix("data:image/")
        .and_then(|rest| rest.split_once(','))
        .map(|(_, data)| data)
        .ok_or_else(|| "画像データがありません".to_string())?;
    let decoded = BASE64_STANDARD
        .decode(payload)
        .map_err(|err| err.to_string())?;
    let mut file = fs::File::create(path).map_err(|err| err.to_string())?;
    file.write_all(&decoded).map_err(|err| err.to_string())
}

pub fn build_ffmpeg_args(
    mp3_path: &Path,
    cover_path: &Path,
    output_path: &Path,
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
        cover_path.to_string_lossy().into_owned(),
        "-filter_complex".into(),
        format!("[0:a]showspectrum=s={resolution}:mode={visualizer}[spec];[1:v][spec]overlay=format=auto"),
        "-shortest".into(),
        "-c:v".into(),
        "libx264".into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
        "-c:a".into(),
        "aac".into(),
        "-movflags".into(),
        "+faststart".into(),
        output_path.to_string_lossy().into_owned(),
    ]
}

pub fn run_ffmpeg(args: &[String]) -> Result<(String, String), String> {
    let output = Command::new("ffmpeg")
        .args(args)
        .output()
        .map_err(|err| err.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok((stdout, stderr))
    } else {
        Err(stderr)
    }
}

#[tauri::command]
fn generate_mp4(app: AppHandle, request: GenerateRequest) -> GenerateResponse {
    match generate_mp4_inner(app, request) {
        Ok(response) => response,
        Err(stderr) => GenerateResponse {
            success: false,
            stdout: String::new(),
            stderr,
            output_path: None,
        },
    }
}

fn generate_mp4_inner(
    app: AppHandle,
    request: GenerateRequest,
) -> Result<GenerateResponse, String> {
    let id =
        safe_id(&extract_suno_id(&request.url).ok_or_else(|| "Suno曲のURLが無効です".to_string())?);
    let resolution = request.resolution.unwrap_or_else(|| "1280x720".to_string());
    let visualizer = request.visualizer.unwrap_or_else(|| "combined".to_string());
    ensure_allowed(&resolution, ALLOWED_RESOLUTIONS, "resolution")?;
    ensure_allowed(&visualizer, ALLOWED_VISUALIZERS, "visualizer")?;

    let base_dir = match request.output_dir {
        Some(output_dir) => PathBuf::from(output_dir),
        None => app
            .path()
            .app_data_dir()
            .map_err(|err| err.to_string())?
            .join("output"),
    };
    fs::create_dir_all(&base_dir).map_err(|err| err.to_string())?;

    let mp3_path = base_dir.join(format!("{id}.mp3"));
    let cover_path = base_dir.join(format!("{id}.jpeg"));
    let output_path = base_dir.join(format!("{id}.mp4"));

    download_to_file(&format!("https://cdn1.suno.ai/{id}.mp3"), &mp3_path)?;
    if let Some(base64) = request.base64 {
        write_base64_image(&base64, &cover_path)?;
    } else {
        download_to_file(
            &format!("https://cdn2.suno.ai/image_large_{id}.jpeg"),
            &cover_path,
        )?;
    }

    let args = build_ffmpeg_args(
        &mp3_path,
        &cover_path,
        &output_path,
        &resolution,
        &visualizer,
    );
    match run_ffmpeg(&args) {
        Ok((stdout, stderr)) => Ok(GenerateResponse {
            success: true,
            stdout,
            stderr,
            output_path: Some(output_path.to_string_lossy().into_owned()),
        }),
        Err(stderr) => Err(stderr),
    }
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![generate_mp4])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_suno_id() {
        assert_eq!(
            extract_suno_id("https://suno.com/song/123e4567-e89b-12d3-a456-426614174000"),
            Some("123e4567-e89b-12d3-a456-426614174000".to_string())
        );
        assert_eq!(extract_suno_id("https://suno.com/"), None);
    }

    #[test]
    fn validates_allowlists() {
        assert!(ensure_allowed("1280x720", ALLOWED_RESOLUTIONS, "resolution").is_ok());
        assert!(ensure_allowed("640x480", ALLOWED_RESOLUTIONS, "resolution").is_err());
        assert!(ensure_allowed("combined", ALLOWED_VISUALIZERS, "visualizer").is_ok());
        assert!(ensure_allowed("spectrum", ALLOWED_VISUALIZERS, "visualizer").is_err());
    }

    #[test]
    fn builds_ffmpeg_as_args_not_shell() {
        let args = build_ffmpeg_args(
            Path::new("/tmp/audio file.mp3"),
            Path::new("/tmp/cover image.jpeg"),
            Path::new("/tmp/out file.mp4"),
            "1280x720",
            "combined",
        );
        assert!(args.contains(&"/tmp/audio file.mp3".to_string()));
        assert!(args.contains(&"/tmp/cover image.jpeg".to_string()));
        assert!(args
            .iter()
            .any(|arg| arg.contains("showspectrum=s=1280x720:mode=combined")));
    }
}
