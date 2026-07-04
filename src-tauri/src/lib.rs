use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Manager};

const ALLOWED_RESOLUTIONS: &[&str] = &["1280x720", "1920x1080", "1080x1080"];
const ALLOWED_VISUALIZERS: &[&str] = &["combined", "separate", "single"];
const ALLOWED_ENCODER_PRESETS: &[&str] = &["cpu_x264", "amd_amf"];
const ALLOWED_QUALITIES: &[&str] = &["standard", "high"];
const SETTINGS_FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct AppSettings {
    pub save_folder: String,
    pub ffmpeg_auto_detect: bool,
    pub ffmpeg_path: String,
    pub encoder_preset: String,
    pub quality: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            save_folder: String::new(),
            ffmpeg_auto_detect: true,
            ffmpeg_path: String::new(),
            encoder_preset: "cpu_x264".to_string(),
            quality: "standard".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRequest {
    pub url: String,
    pub base64: Option<String>,
    pub resolution: Option<String>,
    pub visualizer: Option<String>,
    pub output_dir: Option<String>,
    pub ffmpeg_auto_detect: Option<bool>,
    pub ffmpeg_path: Option<String>,
    pub encoder_preset: Option<String>,
    pub quality: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GenerateResponse {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub output_path: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegTestResponse {
    pub success: bool,
    pub status: String,
    pub version: Option<String>,
    pub path: Option<String>,
    pub details: String,
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

fn describe_download_error(context: &str, url: &str, err: ureq::Error) -> String {
    match err {
        ureq::Error::Status(code, response) => format!(
            "{context} failed for {url}: HTTP {code} {}",
            response.status_text()
        ),
        other => format!("{context} failed for {url}: {other}"),
    }
}

fn download_to_file(agent: &ureq::Agent, url: &str, path: &Path) -> Result<(), String> {
    let response = agent
        .get(url)
        .call()
        .map_err(|err| describe_download_error("Download", url, err))?;
    let mut reader = response.into_reader();
    let mut file = fs::File::create(path).map_err(|err| err.to_string())?;
    std::io::copy(&mut reader, &mut file).map_err(|err| err.to_string())?;
    Ok(())
}

fn warm_up_direct_downloads(
    agent: &ureq::Agent,
    suno_url: &str,
    cover_url: Option<&str>,
) -> Result<(), String> {
    agent
        .get(suno_url)
        .call()
        .map(|_| ())
        .map_err(|err| describe_download_error("Warm-up Suno page request", suno_url, err))?;

    if let Some(cover_url) = cover_url {
        agent
            .get(cover_url)
            .set("Range", "bytes=0-0")
            .call()
            .map(|_| ())
            .map_err(|err| describe_download_error("Warm-up cover request", cover_url, err))?;
    }

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
    encoder_preset: &str,
    quality: &str,
) -> Vec<String> {
    let video_format = if encoder_preset == "amd_amf" {
        "nv12"
    } else {
        "yuv420p"
    };
    let filter_complex = format!(
        "[0:a]showspectrum=s={resolution}:mode={visualizer}[spec];[1:v][spec]overlay=format=auto,scale=trunc(iw/2)*2:trunc(ih/2)*2,format={video_format}[v]"
    );
    let mut args = vec![
        "-y".into(),
        "-i".into(),
        mp3_path.to_string_lossy().into_owned(),
        "-loop".into(),
        "1".into(),
        "-i".into(),
        cover_path.to_string_lossy().into_owned(),
        "-filter_complex".into(),
        filter_complex,
        "-map".into(),
        "[v]".into(),
        "-map".into(),
        "0:a".into(),
        "-shortest".into(),
    ];

    match encoder_preset {
        "amd_amf" => {
            args.extend([
                "-c:v".into(),
                "h264_amf".into(),
                "-quality".into(),
                if quality == "high" {
                    "quality"
                } else {
                    "balanced"
                }
                .into(),
                "-usage".into(),
                "transcoding".into(),
            ]);
        }
        _ => {
            args.extend([
                "-c:v".into(),
                "libx264".into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
            ]);
            if quality == "high" {
                args.extend(["-crf".into(), "18".into(), "-preset".into(), "slow".into()]);
            }
        }
    }

    args.extend([
        "-c:a".into(),
        "aac".into(),
        "-movflags".into(),
        "+faststart".into(),
        output_path.to_string_lossy().into_owned(),
    ]);
    args
}

#[derive(Debug)]
struct FfmpegCandidate {
    label: String,
    path: PathBuf,
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|err| err.to_string())?
        .join(SETTINGS_FILE_NAME))
}

fn default_output_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|err| err.to_string())?
        .join("output"))
}

fn read_settings(app: &AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(app)?;
    if !path.is_file() {
        return Ok(AppSettings::default());
    }
    let text = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let mut settings: AppSettings = serde_json::from_str(&text).map_err(|err| err.to_string())?;
    if !ALLOWED_ENCODER_PRESETS.contains(&settings.encoder_preset.as_str()) {
        settings.encoder_preset = AppSettings::default().encoder_preset;
    }
    if !ALLOWED_QUALITIES.contains(&settings.quality.as_str()) {
        settings.quality = AppSettings::default().quality;
    }
    Ok(settings)
}

fn write_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    ensure_allowed(
        &settings.encoder_preset,
        ALLOWED_ENCODER_PRESETS,
        "encoder preset",
    )?;
    ensure_allowed(&settings.quality, ALLOWED_QUALITIES, "quality")?;
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|err| err.to_string())?;
    fs::write(path, text).map_err(|err| err.to_string())
}

fn auto_detect_ffmpeg_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "(Get-Command ffmpeg -ErrorAction SilentlyContinue).Source",
            ])
            .output()
            .ok()?;
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

fn ffmpeg_candidates(settings: &AppSettings) -> Vec<FfmpegCandidate> {
    let mut candidates = Vec::new();

    if !settings.ffmpeg_auto_detect && !settings.ffmpeg_path.trim().is_empty() {
        candidates.push(FfmpegCandidate {
            label: "user-configured FFmpeg path".to_string(),
            path: PathBuf::from(settings.ffmpeg_path.trim()),
        });
    }

    if settings.ffmpeg_auto_detect {
        if let Some(path) = auto_detect_ffmpeg_path() {
            candidates.push(FfmpegCandidate {
                label: "auto-detected FFmpeg path".to_string(),
                path,
            });
        }
    }

    if let Ok(path) = env::var("SUNO_MV_FFMPEG_PATH") {
        if !path.trim().is_empty() {
            candidates.push(FfmpegCandidate {
                label: "SUNO_MV_FFMPEG_PATH".to_string(),
                path: PathBuf::from(path),
            });
        }
    }

    let project_local = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
        .join("tools")
        .join("ffmpeg")
        .join("bin")
        .join("ffmpeg.exe");
    if project_local.is_file() {
        candidates.push(FfmpegCandidate {
            label: "project-local tools/ffmpeg/bin/ffmpeg.exe".to_string(),
            path: project_local,
        });
    }

    candidates.push(FfmpegCandidate {
        label: "ffmpeg from PATH".to_string(),
        path: PathBuf::from("ffmpeg"),
    });

    candidates
}

fn run_ffmpeg_with_settings(
    args: &[String],
    settings: &AppSettings,
) -> Result<(String, String), String> {
    let candidates = ffmpeg_candidates(settings);
    let mut tried = Vec::new();

    for candidate in &candidates {
        match Command::new(&candidate.path).args(args).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                return if output.status.success() {
                    Ok((stdout, stderr))
                } else {
                    Err(format!(
                        "{} ({}) exited with status {}: {}",
                        candidate.label,
                        candidate.path.display(),
                        output.status,
                        stderr
                    ))
                };
            }
            Err(err) => tried.push(format!(
                "{} ({}): {}",
                candidate.label,
                candidate.path.display(),
                err
            )),
        }
    }

    Err(format!(
        "FFmpeg executable was not found or could not be started. Tried: {}",
        tried.join("; ")
    ))
}

pub fn run_ffmpeg(args: &[String]) -> Result<(String, String), String> {
    run_ffmpeg_with_settings(args, &AppSettings::default())
}

#[tauri::command]
fn load_settings(app: AppHandle) -> Result<AppSettings, String> {
    read_settings(&app)
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    write_settings(&app, &settings)
}

#[tauri::command]
fn detect_ffmpeg(app: AppHandle) -> FfmpegTestResponse {
    let mut settings = read_settings(&app).unwrap_or_default();
    settings.ffmpeg_auto_detect = true;
    let args = vec!["-version".to_string()];
    test_ffmpeg_inner(&settings, &args)
}

#[tauri::command]
fn test_ffmpeg(app: AppHandle, settings: AppSettings) -> FfmpegTestResponse {
    let _ = write_settings(&app, &settings);
    let args = vec!["-version".to_string()];
    test_ffmpeg_inner(&settings, &args)
}

fn test_ffmpeg_inner(settings: &AppSettings, args: &[String]) -> FfmpegTestResponse {
    let candidates = ffmpeg_candidates(settings);
    let mut tried = Vec::new();

    for candidate in &candidates {
        match Command::new(&candidate.path).args(args).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let version = stdout.lines().next().map(|line| line.to_string());
                if output.status.success() {
                    return FfmpegTestResponse {
                        success: true,
                        status: "OK".to_string(),
                        version,
                        path: Some(candidate.path.to_string_lossy().into_owned()),
                        details: format!("{} ({})", candidate.label, candidate.path.display()),
                    };
                }
                tried.push(format!(
                    "{} ({}): exited with status {}: {}",
                    candidate.label,
                    candidate.path.display(),
                    output.status,
                    stderr
                ));
            }
            Err(err) => tried.push(format!(
                "{} ({}): {}",
                candidate.label,
                candidate.path.display(),
                err
            )),
        }
    }

    FfmpegTestResponse {
        success: false,
        status: "Not found".to_string(),
        version: None,
        path: None,
        details: format!("Tried: {}", tried.join("; ")),
    }
}

#[tauri::command]
fn open_output_folder(app: AppHandle, save_folder: Option<String>) -> Result<String, String> {
    let output_dir = match save_folder {
        Some(folder) if !folder.trim().is_empty() => PathBuf::from(folder),
        _ => default_output_dir(&app)?,
    };
    fs::create_dir_all(&output_dir).map_err(|err| err.to_string())?;

    #[cfg(windows)]
    Command::new("explorer")
        .arg(&output_dir)
        .spawn()
        .map_err(|err| err.to_string())?;

    #[cfg(not(windows))]
    Command::new("xdg-open")
        .arg(&output_dir)
        .spawn()
        .map_err(|err| err.to_string())?;

    Ok(output_dir.to_string_lossy().into_owned())
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
    let saved_settings = read_settings(&app).unwrap_or_default();
    let ffmpeg_auto_detect = request
        .ffmpeg_auto_detect
        .unwrap_or(saved_settings.ffmpeg_auto_detect);
    let ffmpeg_path = request.ffmpeg_path.unwrap_or(saved_settings.ffmpeg_path);
    let encoder_preset = request
        .encoder_preset
        .unwrap_or(saved_settings.encoder_preset);
    let quality = request.quality.unwrap_or(saved_settings.quality);
    ensure_allowed(&resolution, ALLOWED_RESOLUTIONS, "resolution")?;
    ensure_allowed(&visualizer, ALLOWED_VISUALIZERS, "visualizer")?;
    ensure_allowed(&encoder_preset, ALLOWED_ENCODER_PRESETS, "encoder preset")?;
    ensure_allowed(&quality, ALLOWED_QUALITIES, "quality")?;

    let base_dir = match request.output_dir {
        Some(output_dir) if !output_dir.trim().is_empty() => PathBuf::from(output_dir),
        _ => default_output_dir(&app)?,
    };
    fs::create_dir_all(&base_dir).map_err(|err| err.to_string())?;

    let mp3_path = base_dir.join(format!("{id}.mp3"));
    let cover_path = base_dir.join(format!("{id}.jpeg"));
    let output_path = base_dir.join(format!("{id}.mp4"));

    let http_agent = ureq::AgentBuilder::new().build();
    let mp3_url = format!("https://cdn1.suno.ai/{id}.mp3");
    let cover_url = format!("https://cdn2.suno.ai/{id}.jpeg");
    let has_frontend_cover = request.base64.is_some();
    warm_up_direct_downloads(
        &http_agent,
        &request.url,
        if has_frontend_cover {
            None
        } else {
            Some(&cover_url)
        },
    )?;

    download_to_file(&http_agent, &mp3_url, &mp3_path)?;
    if let Some(base64) = request.base64 {
        write_base64_image(&base64, &cover_path)?;
    } else {
        download_to_file(&http_agent, &cover_url, &cover_path)?;
    }

    let args = build_ffmpeg_args(
        &mp3_path,
        &cover_path,
        &output_path,
        &resolution,
        &visualizer,
        &encoder_preset,
        &quality,
    );
    let ffmpeg_settings = AppSettings {
        save_folder: base_dir.to_string_lossy().into_owned(),
        ffmpeg_auto_detect,
        ffmpeg_path,
        encoder_preset,
        quality,
    };
    match run_ffmpeg_with_settings(&args, &ffmpeg_settings) {
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
        .invoke_handler(tauri::generate_handler![
            generate_mp4,
            load_settings,
            save_settings,
            detect_ffmpeg,
            test_ffmpeg,
            open_output_folder
        ])
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
            "cpu_x264",
            "standard",
        );
        assert!(args.contains(&"/tmp/audio file.mp3".to_string()));
        assert!(args.contains(&"/tmp/cover image.jpeg".to_string()));
        assert!(args
            .iter()
            .any(|arg| arg.contains("showspectrum=s=1280x720:mode=combined")));
        assert!(args
            .iter()
            .any(|arg| arg.contains("scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p[v]")));
        assert!(args.windows(2).any(|pair| pair == ["-map", "[v]"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "0:a"]));
    }

    #[test]
    fn includes_path_ffmpeg_candidate() {
        let candidates = ffmpeg_candidates(&AppSettings::default());
        assert!(candidates.iter().any(|candidate| {
            candidate.label == "ffmpeg from PATH" && candidate.path == PathBuf::from("ffmpeg")
        }));
    }

    #[test]
    fn prefers_env_ffmpeg_candidate() {
        let previous = env::var("SUNO_MV_FFMPEG_PATH").ok();
        env::set_var("SUNO_MV_FFMPEG_PATH", "C:\\ffmpeg\\bin\\ffmpeg.exe");

        let settings = AppSettings {
            ffmpeg_auto_detect: false,
            ..AppSettings::default()
        };
        let candidates = ffmpeg_candidates(&settings);

        match previous {
            Some(value) => env::set_var("SUNO_MV_FFMPEG_PATH", value),
            None => env::remove_var("SUNO_MV_FFMPEG_PATH"),
        }

        let first = candidates.first().expect("expected ffmpeg candidates");
        assert_eq!(first.label, "SUNO_MV_FFMPEG_PATH");
        assert_eq!(first.path, PathBuf::from("C:\\ffmpeg\\bin\\ffmpeg.exe"));
    }

    #[test]
    fn prefers_user_configured_ffmpeg_candidate() {
        let settings = AppSettings {
            ffmpeg_auto_detect: false,
            ffmpeg_path: "D:\\tools\\ffmpeg.exe".to_string(),
            ..AppSettings::default()
        };

        let candidates = ffmpeg_candidates(&settings);

        let first = candidates.first().expect("expected ffmpeg candidates");
        assert_eq!(first.label, "user-configured FFmpeg path");
        assert_eq!(first.path, PathBuf::from("D:\\tools\\ffmpeg.exe"));
    }

    #[test]
    fn builds_amd_amf_args_without_cpu_fallback() {
        let args = build_ffmpeg_args(
            Path::new("/tmp/audio.mp3"),
            Path::new("/tmp/cover.jpeg"),
            Path::new("/tmp/out.mp4"),
            "1280x720",
            "combined",
            "amd_amf",
            "standard",
        );

        assert!(args.windows(2).any(|pair| pair == ["-c:v", "h264_amf"]));
        assert!(args
            .iter()
            .any(|arg| arg.contains("scale=trunc(iw/2)*2:trunc(ih/2)*2,format=nv12[v]")));
        assert!(!args.iter().any(|arg| arg == "libx264"));
    }

    #[test]
    fn builds_amd_amf_high_with_transcoding_usage() {
        let args = build_ffmpeg_args(
            Path::new("/tmp/audio.mp3"),
            Path::new("/tmp/cover.jpeg"),
            Path::new("/tmp/out.mp4"),
            "1280x720",
            "combined",
            "amd_amf",
            "high",
        );

        assert!(args
            .windows(2)
            .any(|pair| pair == ["-usage", "transcoding"]));
        assert!(args.windows(2).any(|pair| pair == ["-quality", "quality"]));
        assert!(!args.windows(2).any(|pair| pair == ["-usage", "quality"]));
    }
}
