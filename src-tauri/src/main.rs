use base64::Engine;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{path::PathBuf, process::Stdio, sync::Arc};
use tauri::{Emitter, Manager};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::Mutex,
};

#[derive(Default)]
struct AppState {
    ffmpeg: Arc<Mutex<Option<Child>>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ClipMetadata {
    clip_id: String,
    title: Option<String>,
    audio_url: Option<String>,
    image_url: Option<String>,
    video_url: Option<String>,
    raw: Value,
}

#[derive(Debug, Deserialize)]
struct FetchRequest {
    url: String,
    cookie: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GenerateRequest {
    url: String,
    cookie: Option<String>,
    image_data_url: Option<String>,
    resolution: String,
    visualizer: String,
    output_dir: Option<String>,
}

#[tauri::command]
fn extract_clip_id(url: String) -> Result<String, String> {
    extract_id(&url).ok_or_else(|| "Suno URLからclip_idを抽出できません".into())
}

#[tauri::command]
async fn fetch_clip(req: FetchRequest) -> Result<ClipMetadata, String> {
    let clip_id =
        extract_id(&req.url).ok_or_else(|| "Suno URLからclip_idを抽出できません".to_string())?;
    fetch_clip_metadata(&clip_id, req.cookie.as_deref()).await
}

#[tauri::command]
async fn generate_mp4(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    req: GenerateRequest,
) -> Result<String, String> {
    let clip = fetch_clip_metadata(
        &extract_id(&req.url).ok_or("Suno URLからclip_idを抽出できません")?,
        req.cookie.as_deref(),
    )
    .await?;
    let audio_url = clip
        .audio_url
        .clone()
        .ok_or("audio_urlを取得できませんでした")?;
    let image_url = clip.image_url.clone();
    let out_dir = req
        .output_dir
        .map(PathBuf::from)
        .unwrap_or(app.path().app_data_dir().map_err(|e| e.to_string())?);
    tokio::fs::create_dir_all(&out_dir)
        .await
        .map_err(|e| e.to_string())?;
    let audio_path = out_dir.join(format!("{}.mp3", clip.clip_id));
    let image_path = out_dir.join(format!("{}_cover", clip.clip_id));
    let output_path = out_dir.join(format!("{}.mp4", clip.clip_id));

    app.emit("mv-log", format!("clip_id: {}", clip.clip_id))
        .ok();
    download_to(&audio_url, &audio_path, req.cookie.as_deref()).await?;
    if let Some(data_url) = req.image_data_url {
        write_data_url(&data_url, &image_path).await?;
    } else {
        download_to(
            &image_url.ok_or("image_urlを取得できませんでした")?,
            &image_path,
            req.cookie.as_deref(),
        )
        .await?;
    }

    let filter = visualizer_filter(&req.resolution, &req.visualizer);
    let mut child = Command::new("ffmpeg")
        .args(["-y", "-loop", "1", "-i"])
        .arg(&image_path)
        .arg("-i")
        .arg(&audio_path)
        .args([
            "-filter_complex",
            &filter,
            "-map",
            "[v]",
            "-map",
            "1:a",
            "-shortest",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-movflags",
            "+faststart",
        ])
        .arg(&output_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ffmpeg起動失敗: {e}"))?;
    let stderr = child.stderr.take();
    let stdout = child.stdout.take();
    *state.ffmpeg.lock().await = Some(child);
    for pipe in [stdout, stderr].into_iter().flatten() {
        let app2 = app.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(pipe).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                app2.emit("mv-log", line).ok();
            }
        });
    }
    let mut guard = state.ffmpeg.lock().await;
    let status = guard
        .as_mut()
        .unwrap()
        .wait()
        .await
        .map_err(|e| e.to_string())?;
    *guard = None;
    if status.success() {
        Ok(output_path.to_string_lossy().to_string())
    } else {
        Err(format!("ffmpeg failed: {status}"))
    }
}

#[tauri::command]
async fn cancel_generate(state: tauri::State<'_, AppState>) -> Result<(), String> {
    if let Some(child) = state.ffmpeg.lock().await.as_mut() {
        child.kill().await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn extract_id(url: &str) -> Option<String> {
    Regex::new(r"(?i)(?:song|clip)/([0-9a-f-]{20,})")
        .ok()?
        .captures(url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

async fn fetch_clip_metadata(clip_id: &str, cookie: Option<&str>) -> Result<ClipMetadata, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 SunoMV/0.1")
        .build()
        .map_err(|e| e.to_string())?;
    let cookie = cookie
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .or_else(|| std::env::var("SUNO_COOKIE").ok());
    let urls = [
        format!("https://studio-api.suno.ai/api/feed/?ids={clip_id}"),
        format!("https://studio-api.suno.ai/api/feed/v2?ids={clip_id}"),
        format!("https://app.suno.ai/api/feed/?ids={clip_id}"),
        format!("https://api.suno.ai/api/clip/{clip_id}"),
    ];
    let mut last = String::new();
    for url in urls {
        let mut r = client.get(&url).header("Accept", "application/json");
        if let Some(c) = &cookie {
            r = r.header("Cookie", c);
        }
        match r.send().await {
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                if status.is_success() {
                    if let Ok(v) = serde_json::from_str::<Value>(&text) {
                        if let Some(meta) = normalize_clip(clip_id, v) {
                            return Ok(meta);
                        }
                    }
                }
                last = format!("{url}: {status} {text}");
            }
            Err(e) => last = format!("{url}: {e}"),
        }
    }
    Err(format!(
        "メタ情報取得に失敗しました。SUNO_COOKIEを確認してください。最後の応答: {last}"
    ))
}

fn normalize_clip(clip_id: &str, v: Value) -> Option<ClipMetadata> {
    let item = if let Some(a) = v.as_array() {
        a.first()?.clone()
    } else if let Some(a) = v.get("clips").and_then(|x| x.as_array()) {
        a.first()?.clone()
    } else {
        v.clone()
    };
    Some(ClipMetadata {
        clip_id: clip_id.into(),
        title: item
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_string),
        audio_url: find_url(&item, &["audio_url", "audioUrl"]),
        image_url: find_url(&item, &["image_url", "imageUrl", "image_large_url"])
            .or_else(|| Some(format!("https://cdn2.suno.ai/image_large_{clip_id}.jpeg"))),
        video_url: find_url(&item, &["video_url", "videoUrl"]),
        raw: item,
    })
}
fn find_url(v: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| v.get(k).and_then(Value::as_str).map(str::to_string))
        .or_else(|| {
            v.get("metadata").and_then(|m| {
                keys.iter()
                    .find_map(|k| m.get(k).and_then(Value::as_str).map(str::to_string))
            })
        })
}
async fn download_to(url: &str, path: &PathBuf, cookie: Option<&str>) -> Result<(), String> {
    let client = reqwest::Client::new();
    let mut r = client.get(url);
    if let Some(c) = cookie {
        r = r.header("Cookie", c);
    }
    let bytes = r
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    tokio::fs::write(path, bytes)
        .await
        .map_err(|e| e.to_string())
}
async fn write_data_url(data_url: &str, path: &PathBuf) -> Result<(), String> {
    let b64 = data_url.split_once(',').ok_or("画像Data URLが不正です")?.1;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| e.to_string())?;
    tokio::fs::write(path, bytes)
        .await
        .map_err(|e| e.to_string())
}
fn visualizer_filter(res: &str, kind: &str) -> String {
    let safe_res = if Regex::new(r"^\d+x\d+$").unwrap().is_match(res) {
        res
    } else {
        "1280x720"
    };
    match kind { "waveform" => format!("[0:v]scale={safe_res}:force_original_aspect_ratio=increase,crop={safe_res},setsar=1[bg];[1:a]showwaves=s={safe_res}:mode=line:colors=White@0.8[vis];[bg][vis]overlay=format=auto[v]"), _ => format!("[0:v]scale={safe_res}:force_original_aspect_ratio=increase,crop={safe_res},setsar=1[bg];[1:a]showspectrum=s={safe_res}:mode=combined:color=intensity[vis];[bg][vis]blend=all_mode=screen:all_opacity=0.65[v]") }
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            extract_clip_id,
            fetch_clip,
            generate_mp4,
            cancel_generate
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri app");
}
