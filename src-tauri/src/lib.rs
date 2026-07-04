use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Manager};

const ALLOWED_RESOLUTIONS: &[&str] = &["1280x720", "1920x1080", "1080x1080"];
const ALLOWED_VISUALIZERS: &[&str] = &["none", "spectrum"];
const ALLOWED_ENCODER_PRESETS: &[&str] = &["cpu_x264", "amd_amf"];
const ALLOWED_QUALITIES: &[&str] = &["standard", "high"];
const SETTINGS_FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(default)]
pub struct AppSettings {
    pub save_folder: String,
    pub ffmpeg_auto_detect: bool,
    pub ffmpeg_path: String,
    pub encoder_preset: String,
    pub quality: String,
    pub suno_authorization: String,
    pub suno_browser_token: String,
    pub suno_device_id: String,
    pub suno_max_pages: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            save_folder: String::new(),
            ffmpeg_auto_detect: true,
            ffmpeg_path: String::new(),
            encoder_preset: "cpu_x264".to_string(),
            quality: "standard".to_string(),
            suno_authorization: String::new(),
            suno_browser_token: String::new(),
            suno_device_id: String::new(),
            suno_max_pages: 100,
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
    pub audio_url: Option<String>,
    pub metadata_image_url: Option<String>,
    pub output_filename: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SunoMetadata {
    pub id: Option<String>,
    pub title: Option<String>,
    pub display_name: Option<String>,
    pub created_at: Option<String>,
    pub audio_url: Option<String>,
    pub video_url: Option<String>,
    pub image_url: Option<String>,
    pub image_large_url: Option<String>,
    pub prompt: Option<String>,
    pub lyrics: Option<String>,
    pub style: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MetadataFetchResponse {
    pub success: bool,
    pub status: String,
    pub song_id: Option<String>,
    pub metadata: Option<SunoMetadata>,
    pub pages_checked: u32,
    pub logs: Vec<String>,
}

pub fn extract_suno_id(url: &str) -> Option<String> {
    let marker = if url.contains("/song/") {
        "/song/"
    } else {
        "/s/"
    };
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

pub fn mask_secret(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if let Some((prefix, token)) = trimmed.split_once(' ') {
        if !token.is_empty() {
            return format!("{prefix} {}", mask_token(token));
        }
    }

    mask_token(trimmed)
}

fn mask_token(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 6 {
        return "***".to_string();
    }
    let start: String = chars.iter().take(3).collect();
    let end: String = chars
        .iter()
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{start}...{end}")
}

pub fn has_suno_api_settings(settings: &AppSettings) -> bool {
    !settings.suno_authorization.trim().is_empty()
        && !settings.suno_browser_token.trim().is_empty()
        && !settings.suno_device_id.trim().is_empty()
}

pub fn select_audio_source(metadata: Option<&SunoMetadata>, fallback: &str) -> String {
    metadata
        .and_then(|metadata| metadata.audio_url.as_deref())
        .filter(|url| !url.trim().is_empty())
        .unwrap_or(fallback)
        .to_string()
}

pub fn select_cover_source(
    frontend_data: Option<&str>,
    metadata: Option<&SunoMetadata>,
    fallback: &str,
) -> String {
    if let Some(frontend_data) = frontend_data {
        if !frontend_data.trim().is_empty() {
            return frontend_data.to_string();
        }
    }

    metadata
        .and_then(|metadata| {
            metadata
                .image_large_url
                .as_deref()
                .or(metadata.image_url.as_deref())
        })
        .filter(|url| !url.trim().is_empty())
        .unwrap_or(fallback)
        .to_string()
}

pub fn find_matching_clip(clips: &[SunoMetadata], id: &str) -> Option<SunoMetadata> {
    clips
        .iter()
        .find(|clip| clip.id.as_deref() == Some(id))
        .cloned()
}

fn safe_filename(value: &str) -> String {
    let name: String = value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect();
    let trimmed = name.trim().trim_matches('.').to_string();
    if trimmed.is_empty() {
        "suno_mv".to_string()
    } else {
        trimmed.chars().take(120).collect()
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

fn download_to_file_with_context(
    agent: &ureq::Agent,
    url: &str,
    path: &Path,
    context: &str,
) -> Result<(), String> {
    let response = agent
        .get(url)
        .call()
        .map_err(|err| describe_download_error(context, url, err))?;
    let mut reader = response.into_reader();
    let mut file = fs::File::create(path).map_err(|err| err.to_string())?;
    std::io::copy(&mut reader, &mut file).map_err(|err| err.to_string())?;
    Ok(())
}

fn metadata_from_value(value: &serde_json::Value) -> SunoMetadata {
    SunoMetadata {
        id: value
            .get("id")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        title: value
            .get("title")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        display_name: value
            .get("display_name")
            .or_else(|| value.get("displayName"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        created_at: value
            .get("created_at")
            .or_else(|| value.get("createdAt"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        audio_url: value
            .get("audio_url")
            .or_else(|| value.get("audioUrl"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        video_url: value
            .get("video_url")
            .or_else(|| value.get("videoUrl"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        image_url: value
            .get("image_url")
            .or_else(|| value.get("imageUrl"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        image_large_url: value
            .get("image_large_url")
            .or_else(|| value.get("imageLargeUrl"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        prompt: value
            .get("prompt")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        lyrics: value
            .get("lyrics")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        style: value
            .get("style")
            .or_else(|| value.get("style_prompt"))
            .or_else(|| value.get("stylePrompt"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
    }
}

fn first_string_for_keys(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            for key in keys {
                if let Some(found) = map.get(*key).and_then(|value| value.as_str()) {
                    if !found.trim().is_empty() {
                        return Some(found.to_string());
                    }
                }
            }
            for child in map.values() {
                if let Some(found) = first_string_for_keys(child, keys) {
                    return Some(found);
                }
            }
            None
        }
        serde_json::Value::Array(items) => items
            .iter()
            .find_map(|item| first_string_for_keys(item, keys)),
        _ => None,
    }
}

fn metadata_from_embedded_json(value: &serde_json::Value, song_id: &str) -> SunoMetadata {
    SunoMetadata {
        id: Some(song_id.to_string()),
        title: first_string_for_keys(value, &["title", "name"]),
        display_name: first_string_for_keys(value, &["display_name", "displayName", "artist_name"]),
        created_at: first_string_for_keys(value, &["created_at", "createdAt", "dateCreated"]),
        audio_url: first_string_for_keys(value, &["audio_url", "audioUrl", "audio", "contentUrl"]),
        video_url: first_string_for_keys(value, &["video_url", "videoUrl", "video"]),
        image_url: first_string_for_keys(
            value,
            &["image_url", "imageUrl", "thumbnailUrl", "image"],
        ),
        image_large_url: first_string_for_keys(value, &["image_large_url", "imageLargeUrl"]),
        prompt: first_string_for_keys(
            value,
            &[
                "prompt",
                "gpt_description_prompt",
                "description",
                "style_prompt",
            ],
        ),
        lyrics: first_string_for_keys(value, &["lyrics", "lyric"]),
        style: first_string_for_keys(value, &["style", "style_prompt", "stylePrompt"]),
    }
}

fn merge_metadata(primary: SunoMetadata, secondary: Option<SunoMetadata>) -> SunoMetadata {
    let Some(secondary) = secondary else {
        return primary;
    };

    SunoMetadata {
        id: primary.id.or(secondary.id),
        title: primary.title.or(secondary.title),
        display_name: primary.display_name.or(secondary.display_name),
        created_at: primary.created_at.or(secondary.created_at),
        audio_url: primary.audio_url.or(secondary.audio_url),
        video_url: primary.video_url.or(secondary.video_url),
        image_url: primary.image_url.or(secondary.image_url),
        image_large_url: primary.image_large_url.or(secondary.image_large_url),
        prompt: primary.prompt.or(secondary.prompt),
        lyrics: primary.lyrics.or(secondary.lyrics),
        style: primary.style.or(secondary.style),
    }
}

fn has_any_metadata(metadata: &SunoMetadata) -> bool {
    metadata.title.is_some()
        || metadata.display_name.is_some()
        || metadata.created_at.is_some()
        || metadata.audio_url.is_some()
        || metadata.video_url.is_some()
        || metadata.image_url.is_some()
        || metadata.image_large_url.is_some()
        || metadata.prompt.is_some()
        || metadata.lyrics.is_some()
        || metadata.style.is_some()
}

fn script_contents(html: &str) -> Vec<String> {
    let mut contents = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<script") {
        rest = &rest[start..];
        let Some(open_end) = rest.find('>') else {
            break;
        };
        let after_open = &rest[open_end + 1..];
        let Some(close_start) = after_open.find("</script>") else {
            break;
        };
        contents.push(html_unescape(&after_open[..close_start]));
        rest = &after_open[close_start + "</script>".len()..];
    }
    contents
}

fn html_unescape(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn extract_page_metadata_from_html(html: &str, song_id: &str) -> Option<SunoMetadata> {
    let mut merged = SunoMetadata {
        id: Some(song_id.to_string()),
        ..SunoMetadata::default()
    };

    for script in script_contents(html) {
        let trimmed = script.trim();
        if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let metadata = metadata_from_embedded_json(&value, song_id);
            merged = merge_metadata(merged, Some(metadata));
        }
    }

    if !has_any_metadata(&merged) {
        if let Some(title) = extract_meta_content(html, "og:title").or_else(|| extract_title(html))
        {
            merged.title = Some(title);
        }
        if let Some(image) = extract_meta_content(html, "og:image") {
            merged.image_url = Some(image);
        }
        if let Some(audio) = extract_meta_content(html, "og:audio") {
            merged.audio_url = Some(audio);
        }
        if let Some(description) = extract_meta_content(html, "description") {
            merged.prompt = Some(description);
        }
    }

    if has_any_metadata(&merged) {
        Some(merged)
    } else {
        None
    }
}

fn extract_title(html: &str) -> Option<String> {
    let start = html.find("<title>")? + "<title>".len();
    let end = html[start..].find("</title>")? + start;
    Some(html_unescape(html[start..end].trim()))
}

fn extract_meta_content(html: &str, name: &str) -> Option<String> {
    for marker in [
        format!("property=\"{name}\""),
        format!("name=\"{name}\""),
        format!("property='{name}'"),
        format!("name='{name}'"),
    ] {
        let Some(marker_start) = html.find(&marker) else {
            continue;
        };
        let tag_start = html[..marker_start].rfind('<').unwrap_or(marker_start);
        let tag_end = html[marker_start..]
            .find('>')
            .map(|offset| marker_start + offset)
            .unwrap_or(html.len());
        let tag = &html[tag_start..tag_end];
        if let Some(content) = extract_attr(tag, "content") {
            if !content.trim().is_empty() {
                return Some(html_unescape(content.trim()));
            }
        }
    }
    None
}

fn extract_attr<'a>(tag: &'a str, attr: &str) -> Option<&'a str> {
    let marker = format!("{attr}=");
    let start = tag.find(&marker)? + marker.len();
    let quote = tag[start..].chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let value_start = start + quote.len_utf8();
    let value_end = tag[value_start..].find(quote)? + value_start;
    Some(&tag[value_start..value_end])
}

fn warm_up_suno_page(agent: &ureq::Agent, suno_url: &str) -> Result<String, String> {
    let response = agent
        .get(suno_url)
        .call()
        .map_err(|err| describe_download_error("Warm-up Suno page request", suno_url, err))?;
    let mut text = String::new();
    response
        .into_reader()
        .read_to_string(&mut text)
        .map_err(|err| format!("Warm-up Suno page read failed for {suno_url}: {err}"))?;
    Ok(text)
}

fn clips_from_feed_value(value: &serde_json::Value) -> Vec<SunoMetadata> {
    if let Some(items) = value.as_array() {
        return items.iter().map(metadata_from_value).collect();
    }

    for key in ["clips", "data", "items"] {
        if let Some(items) = value.get(key).and_then(|value| value.as_array()) {
            return items.iter().map(metadata_from_value).collect();
        }
    }

    Vec::new()
}

fn fetch_suno_metadata_inner(
    agent: &ureq::Agent,
    settings: &AppSettings,
    suno_url: &str,
    song_id: &str,
) -> MetadataFetchResponse {
    let max_pages = settings.suno_max_pages.max(1);
    let mut logs = vec![
        format!("id extracted: {song_id}"),
        format!("warm-up Suno page started: {suno_url}"),
    ];

    let page_metadata = match warm_up_suno_page(agent, suno_url) {
        Ok(html) => {
            logs.push("warm-up Suno page succeeded".to_string());
            logs.push("metadata source attempted: page embedded data".to_string());
            match extract_page_metadata_from_html(&html, song_id) {
                Some(metadata) => {
                    logs.push("metadata source result: page found".to_string());
                    logs.push(format!(
                        "metadata selected fields: title {}, lyrics {}, prompt {}, style {}",
                        yes_no(metadata.title.is_some()),
                        yes_no(metadata.lyrics.is_some()),
                        yes_no(metadata.prompt.is_some()),
                        yes_no(metadata.style.is_some())
                    ));
                    Some(metadata)
                }
                None => {
                    logs.push("metadata source result: page unavailable".to_string());
                    None
                }
            }
        }
        Err(err) => {
            logs.push(format!("warm-up Suno page failed: {err}"));
            logs.push("metadata source attempted: page embedded data".to_string());
            logs.push("metadata source result: page unavailable".to_string());
            None
        }
    };

    if !has_suno_api_settings(settings) {
        logs.push("metadata source attempted: feed API".to_string());
        logs.push("metadata source result: feed API skipped because settings missing".to_string());
        if let Some(metadata) = page_metadata.clone() {
            return MetadataFetchResponse {
                success: true,
                status: "Metadata found".to_string(),
                song_id: Some(song_id.to_string()),
                metadata: Some(metadata),
                pages_checked: 0,
                logs,
            };
        }
        return MetadataFetchResponse {
            success: false,
            status: "Metadata unavailable, using fallback".to_string(),
            song_id: Some(song_id.to_string()),
            metadata: None,
            pages_checked: 0,
            logs,
        };
    }

    logs.push("metadata source attempted: feed API".to_string());
    logs.push(format!(
        "authorization {}",
        mask_secret(&settings.suno_authorization)
    ));
    logs.push(format!(
        "browser-token {}",
        mask_secret(&settings.suno_browser_token)
    ));
    logs.push(format!(
        "device-id {}",
        mask_secret(&settings.suno_device_id)
    ));

    for page in 0..max_pages {
        let url = format!("https://studio-api-prod.suno.com/api/feed/?page={page}");
        logs.push(format!("metadata fetch page={page}"));
        let response = match agent
            .get(&url)
            .set("authorization", settings.suno_authorization.trim())
            .set("browser-token", settings.suno_browser_token.trim())
            .set("device-id", settings.suno_device_id.trim())
            .set("accept", "*/*")
            .set("content-type", "application/json")
            .call()
        {
            Ok(response) => response,
            Err(err) => {
                let status = describe_download_error("Metadata fetch", &url, err);
                logs.push(status.clone());
                let merged = merge_metadata(page_metadata.clone().unwrap_or_default(), None);
                return MetadataFetchResponse {
                    success: has_any_metadata(&merged),
                    status,
                    song_id: Some(song_id.to_string()),
                    metadata: if has_any_metadata(&merged) {
                        Some(merged)
                    } else {
                        None
                    },
                    pages_checked: page + 1,
                    logs,
                };
            }
        };

        let mut text = String::new();
        let value = match response
            .into_reader()
            .read_to_string(&mut text)
            .map_err(|err| err.to_string())
            .and_then(|_| {
                serde_json::from_str::<serde_json::Value>(&text).map_err(|err| err.to_string())
            }) {
            Ok(value) => value,
            Err(err) => {
                let status = format!("Metadata fetch failed for {url}: invalid JSON: {err}");
                logs.push(status.clone());
                return MetadataFetchResponse {
                    success: false,
                    status,
                    song_id: Some(song_id.to_string()),
                    metadata: None,
                    pages_checked: page + 1,
                    logs,
                };
            }
        };
        let clips = clips_from_feed_value(&value);
        logs.push(format!("metadata page={page} clips={}", clips.len()));
        if clips.is_empty() {
            let merged = merge_metadata(page_metadata.clone().unwrap_or_default(), None);
            return MetadataFetchResponse {
                success: has_any_metadata(&merged),
                status: "Metadata unavailable, using fallback".to_string(),
                song_id: Some(song_id.to_string()),
                metadata: if has_any_metadata(&merged) {
                    Some(merged)
                } else {
                    None
                },
                pages_checked: page + 1,
                logs,
            };
        }
        if let Some(metadata) = find_matching_clip(&clips, song_id) {
            logs.push(format!("metadata match page={page} id={song_id}"));
            logs.push("metadata source result: feed API found".to_string());
            let merged = merge_metadata(page_metadata.clone().unwrap_or_default(), Some(metadata));
            return MetadataFetchResponse {
                success: true,
                status: "Metadata found".to_string(),
                song_id: Some(song_id.to_string()),
                metadata: Some(merged),
                pages_checked: page + 1,
                logs,
            };
        }
    }

    logs.push(format!("metadata match not found within {max_pages} pages"));
    logs.push("metadata source result: feed API no match".to_string());
    let merged = merge_metadata(page_metadata.clone().unwrap_or_default(), None);
    MetadataFetchResponse {
        success: has_any_metadata(&merged),
        status: "Metadata unavailable, using fallback".to_string(),
        song_id: Some(song_id.to_string()),
        metadata: if has_any_metadata(&merged) {
            Some(merged)
        } else {
            None
        },
        pages_checked: max_pages,
        logs,
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn warm_up_direct_downloads(agent: &ureq::Agent, suno_url: &str) -> Result<(), String> {
    warm_up_suno_page(agent, suno_url).map(|_| ())
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
    let filter_complex = build_video_filter_complex(resolution, visualizer, video_format);
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

pub fn build_video_filter_complex(
    resolution: &str,
    visualizer: &str,
    video_format: &str,
) -> String {
    let final_video_filter = format!("scale=trunc(iw/2)*2:trunc(ih/2)*2,format={video_format}[v]");
    if visualizer == "spectrum" {
        format!(
            "[0:a]showspectrum=s={resolution}:mode=combined[spec];[1:v][spec]overlay=format=auto,{final_video_filter}"
        )
    } else {
        format!("[1:v]{final_video_filter}")
    }
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
fn fetch_suno_metadata(app: AppHandle, url: String) -> MetadataFetchResponse {
    let settings = match read_settings(&app) {
        Ok(settings) => settings,
        Err(err) => {
            return MetadataFetchResponse {
                success: false,
                status: format!("Settings load failed: {err}"),
                song_id: None,
                metadata: None,
                pages_checked: 0,
                logs: vec![format!("Settings load failed: {err}")],
            };
        }
    };
    let song_id = match extract_suno_id(&url) {
        Some(song_id) => song_id,
        None => {
            return MetadataFetchResponse {
                success: false,
                status: "Suno URL id could not be extracted.".to_string(),
                song_id: None,
                metadata: None,
                pages_checked: 0,
                logs: vec!["id extraction failed".to_string()],
            };
        }
    };
    let agent = ureq::AgentBuilder::new().build();
    fetch_suno_metadata_inner(&agent, &settings, &url, &song_id)
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
    let visualizer = request.visualizer.unwrap_or_else(|| "none".to_string());
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

    let filename = request
        .output_filename
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(safe_filename)
        .unwrap_or_else(|| id.clone());
    let mp3_path = base_dir.join(format!("{filename}.mp3"));
    let cover_path = base_dir.join(format!("{filename}.jpeg"));
    let output_path = base_dir.join(format!("{filename}.mp4"));

    let http_agent = ureq::AgentBuilder::new().build();
    let fallback_mp3_url = format!("https://cdn1.suno.ai/{id}.mp3");
    let fallback_cover_url = format!("https://cdn2.suno.ai/{id}.jpeg");
    let mp3_url = select_audio_source(None, &fallback_mp3_url);
    let cover_url = select_cover_source(
        request.base64.as_deref(),
        Some(&SunoMetadata {
            image_url: request.metadata_image_url.clone(),
            image_large_url: request.metadata_image_url.clone(),
            ..SunoMetadata::default()
        }),
        &fallback_cover_url,
    );
    let mp3_url = request
        .audio_url
        .as_deref()
        .filter(|url| !url.trim().is_empty())
        .unwrap_or(&mp3_url)
        .to_string();
    warm_up_direct_downloads(&http_agent, &request.url)?;

    download_to_file_with_context(&http_agent, &mp3_url, &mp3_path, "Audio download")?;
    if let Some(base64) = request.base64 {
        write_base64_image(&base64, &cover_path)?;
    } else {
        download_to_file_with_context(&http_agent, &cover_url, &cover_path, "Cover fetch")?;
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
        ..AppSettings::default()
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
            open_output_folder,
            fetch_suno_metadata
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    const DEFAULT_INSPECT_URL: &str = "https://suno.com/song/daf62c51-6b0a-4dd2-bfa7-99795df4720c";

    #[derive(Debug, Serialize)]
    struct InspectMetaTag {
        name_or_property: String,
        content: String,
    }

    #[derive(Debug, Serialize)]
    struct NextPayloadMatch {
        target: String,
        index: usize,
        snippet: String,
    }

    #[derive(Debug, Serialize)]
    struct NextPayloadSummary {
        index: usize,
        char_length: usize,
        summary: String,
        matches: Vec<NextPayloadMatch>,
        decoded_json_fragment_count: usize,
    }

    #[derive(Debug, Serialize)]
    struct SunoMetadataInspectionReport {
        url: String,
        id: String,
        http_status: u16,
        html_length: usize,
        has_next_data: bool,
        has_next_f_payload: bool,
        meta_tags: Vec<InspectMetaTag>,
        json_script_count: usize,
        next_payload_count: usize,
        next_payloads: Vec<NextPayloadSummary>,
        recursive_keys: BTreeMap<String, Vec<String>>,
        candidate_values: BTreeMap<String, Vec<String>>,
    }

    fn sanitize_inspection_text(value: &str) -> String {
        let mut sanitized = value.to_string();
        for marker in [
            "authorization",
            "browser-token",
            "device-id",
            "cookie",
            "set-cookie",
        ] {
            let lower = sanitized.to_lowercase();
            let mut search_start = 0;
            while let Some(found) = lower[search_start..].find(marker) {
                let start = search_start + found;
                let value_start = sanitized[start..]
                    .find([':', '=', '"'])
                    .map(|offset| start + offset + 1)
                    .unwrap_or(start + marker.len());
                let value_end = sanitized[value_start..]
                    .find([',', ';', '\n', '\r', '"'])
                    .map(|offset| value_start + offset)
                    .unwrap_or_else(|| sanitized.len().min(value_start + 80));
                sanitized.replace_range(value_start..value_end, " ***");
                search_start = (value_start + 4).min(sanitized.len());
            }
        }
        sanitized.chars().take(500).collect()
    }

    fn inspect_meta_tags(html: &str) -> Vec<InspectMetaTag> {
        let mut tags = Vec::new();
        let mut rest = html;
        while let Some(start) = rest.find("<meta") {
            rest = &rest[start..];
            let Some(end) = rest.find('>') else {
                break;
            };
            let tag = &rest[..end + 1];
            let name_or_property = extract_attr(tag, "property")
                .or_else(|| extract_attr(tag, "name"))
                .map(html_unescape);
            let content = extract_attr(tag, "content").map(html_unescape);
            if let (Some(name_or_property), Some(content)) = (name_or_property, content) {
                tags.push(InspectMetaTag {
                    name_or_property: sanitize_inspection_text(&name_or_property),
                    content: sanitize_inspection_text(&content),
                });
            }
            rest = &rest[end + 1..];
        }
        tags
    }

    fn json_looking_script_values(html: &str) -> Vec<serde_json::Value> {
        script_contents(html)
            .into_iter()
            .filter_map(|script| {
                let trimmed = script.trim();
                if trimmed.starts_with('{') || trimmed.starts_with('[') {
                    serde_json::from_str::<serde_json::Value>(trimmed).ok()
                } else {
                    None
                }
            })
            .collect()
    }

    fn extract_next_f_payloads(html: &str) -> Vec<String> {
        let marker = "self.__next_f.push(";
        let mut payloads = Vec::new();
        let mut rest = html;
        while let Some(start) = rest.find(marker) {
            let payload_start = start + marker.len();
            let chars: Vec<(usize, char)> = rest[payload_start..].char_indices().collect();
            let mut depth = 1usize;
            let mut in_string: Option<char> = None;
            let mut escaped = false;
            let mut end_offset = None;
            for (offset, ch) in chars {
                if let Some(quote) = in_string {
                    if escaped {
                        escaped = false;
                    } else if ch == '\\' {
                        escaped = true;
                    } else if ch == quote {
                        in_string = None;
                    }
                    continue;
                }
                match ch {
                    '"' | '\'' | '`' => in_string = Some(ch),
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end_offset = Some(offset);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if let Some(end_offset) = end_offset {
                payloads.push(html_unescape(
                    &rest[payload_start..payload_start + end_offset],
                ));
                rest = &rest[payload_start + end_offset + 1..];
            } else {
                break;
            }
        }
        payloads
    }

    fn snippet_around(value: &str, needle: &str) -> Option<String> {
        let lower = value.to_lowercase();
        let needle = needle.to_lowercase();
        let start = lower.find(&needle)?;
        let snippet_start = start.saturating_sub(120);
        let snippet_end = (start + needle.len() + 180).min(value.len());
        Some(sanitize_inspection_text(&value[snippet_start..snippet_end]))
    }

    fn json_fragments_from_payload(payload: &str) -> Vec<serde_json::Value> {
        let mut values = Vec::new();
        for (open, close) in [('[', ']'), ('{', '}')] {
            let mut search_start = 0;
            while let Some(relative_start) = payload[search_start..].find(open) {
                let start = search_start + relative_start;
                if let Some(end) = find_balanced_json_end(&payload[start..], open, close) {
                    let fragment = &payload[start..start + end];
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(fragment) {
                        values.push(value);
                    }
                    search_start = start + end;
                } else {
                    break;
                }
            }
        }
        let nested = values
            .iter()
            .flat_map(json_fragments_from_json_strings)
            .collect::<Vec<_>>();
        values.extend(nested);
        values
    }

    fn json_fragments_from_json_strings(value: &serde_json::Value) -> Vec<serde_json::Value> {
        let mut values = Vec::new();
        match value {
            serde_json::Value::String(text) => {
                let trimmed = text.trim();
                if trimmed.starts_with('{') || trimmed.starts_with('[') {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
                        values.push(value);
                    }
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    values.extend(json_fragments_from_json_strings(item));
                }
            }
            serde_json::Value::Object(map) => {
                for item in map.values() {
                    values.extend(json_fragments_from_json_strings(item));
                }
            }
            _ => {}
        }
        values
    }

    fn find_balanced_json_end(value: &str, open: char, close: char) -> Option<usize> {
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escaped = false;
        for (offset, ch) in value.char_indices() {
            if in_string {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    in_string = false;
                }
                continue;
            }
            match ch {
                '"' => in_string = true,
                ch if ch == open => depth += 1,
                ch if ch == close => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(offset + ch.len_utf8());
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn inspect_next_payloads(
        html: &str,
        targets: &[&str],
        recursive_keys: &mut BTreeMap<String, BTreeSet<String>>,
        candidate_values: &mut BTreeMap<String, BTreeSet<String>>,
    ) -> Vec<NextPayloadSummary> {
        extract_next_f_payloads(html)
            .into_iter()
            .enumerate()
            .map(|(index, payload)| {
                let matches = targets
                    .iter()
                    .filter_map(|target| {
                        snippet_around(&payload, target).map(|snippet| NextPayloadMatch {
                            target: (*target).to_string(),
                            index,
                            snippet,
                        })
                    })
                    .collect::<Vec<_>>();
                let json_fragments = json_fragments_from_payload(&payload);
                for value in &json_fragments {
                    collect_recursive_key_paths(
                        value,
                        &format!("next_payload[{index}]"),
                        targets,
                        recursive_keys,
                        candidate_values,
                    );
                }
                NextPayloadSummary {
                    index,
                    char_length: payload.len(),
                    summary: sanitize_inspection_text(&payload),
                    matches,
                    decoded_json_fragment_count: json_fragments.len(),
                }
            })
            .collect()
    }

    fn collect_recursive_key_paths(
        value: &serde_json::Value,
        path: &str,
        targets: &[&str],
        keys: &mut BTreeMap<String, BTreeSet<String>>,
        values: &mut BTreeMap<String, BTreeSet<String>>,
    ) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, child) in map {
                    let child_path = if path.is_empty() {
                        key.to_string()
                    } else {
                        format!("{path}.{key}")
                    };
                    for target in targets {
                        if key.eq_ignore_ascii_case(target) || key.to_lowercase().contains(target) {
                            keys.entry((*target).to_string())
                                .or_default()
                                .insert(child_path.clone());
                            if let Some(value) = child.as_str() {
                                if !value.trim().is_empty() {
                                    values
                                        .entry((*target).to_string())
                                        .or_default()
                                        .insert(sanitize_inspection_text(value));
                                }
                            }
                        }
                    }
                    collect_recursive_key_paths(child, &child_path, targets, keys, values);
                }
            }
            serde_json::Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    let child_path = format!("{path}[{index}]");
                    collect_recursive_key_paths(child, &child_path, targets, keys, values);
                }
            }
            serde_json::Value::String(value) => {
                let lower = value.to_lowercase();
                for target in targets {
                    if lower.contains(target) {
                        keys.entry((*target).to_string())
                            .or_default()
                            .insert(path.to_string());
                        values
                            .entry((*target).to_string())
                            .or_default()
                            .insert(sanitize_inspection_text(value));
                    }
                }
            }
            _ => {}
        }
    }

    fn inspect_suno_metadata_html(
        url: &str,
        id: &str,
        http_status: u16,
        html: &str,
    ) -> SunoMetadataInspectionReport {
        let targets = [
            "lyrics",
            "prompt",
            "style",
            "tags",
            "title",
            "audio_url",
            "image_url",
            "daf62c51",
        ];
        let json_values = json_looking_script_values(html);
        let mut recursive_keys: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut candidate_values: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for value in &json_values {
            collect_recursive_key_paths(
                value,
                "",
                &targets,
                &mut recursive_keys,
                &mut candidate_values,
            );
        }
        let next_payloads =
            inspect_next_payloads(html, &targets, &mut recursive_keys, &mut candidate_values);

        for meta in inspect_meta_tags(html) {
            let lower_key = meta.name_or_property.to_lowercase();
            for target in targets {
                if lower_key.contains(target)
                    || (target == "image_url" && lower_key.contains("image"))
                    || (target == "title" && lower_key.contains("title"))
                {
                    candidate_values
                        .entry(target.to_string())
                        .or_default()
                        .insert(meta.content.clone());
                    recursive_keys
                        .entry(target.to_string())
                        .or_default()
                        .insert(format!("meta:{}", meta.name_or_property));
                }
            }
        }

        let recursive_keys = recursive_keys
            .into_iter()
            .map(|(key, paths)| (key, paths.into_iter().collect()))
            .collect();
        let candidate_values = candidate_values
            .into_iter()
            .map(|(key, values)| (key, values.into_iter().collect()))
            .collect();

        SunoMetadataInspectionReport {
            url: url.to_string(),
            id: id.to_string(),
            http_status,
            html_length: html.len(),
            has_next_data: html.contains("__NEXT_DATA__"),
            has_next_f_payload: html.contains("self.__next_f.push"),
            meta_tags: inspect_meta_tags(html),
            json_script_count: json_values.len(),
            next_payload_count: next_payloads.len(),
            next_payloads,
            recursive_keys,
            candidate_values,
        }
    }

    #[test]
    fn extracts_suno_id() {
        assert_eq!(
            extract_suno_id("https://suno.com/song/123e4567-e89b-12d3-a456-426614174000"),
            Some("123e4567-e89b-12d3-a456-426614174000".to_string())
        );
        assert_eq!(
            extract_suno_id("https://suno.com/s/123e4567-e89b-12d3-a456-426614174000"),
            Some("123e4567-e89b-12d3-a456-426614174000".to_string())
        );
        assert_eq!(
            extract_suno_id("https://www.suno.com/s/123e4567-e89b-12d3-a456-426614174000?foo=bar"),
            Some("123e4567-e89b-12d3-a456-426614174000".to_string())
        );
        assert_eq!(extract_suno_id("https://suno.com/"), None);
    }

    #[test]
    #[ignore]
    fn inspect_suno_metadata_sources_ignored() {
        let url =
            env::var("SUNO_MV_INSPECT_URL").unwrap_or_else(|_| DEFAULT_INSPECT_URL.to_string());
        let id = extract_suno_id(&url).expect("SUNO_MV_INSPECT_URL should contain a Suno id");
        let agent = ureq::AgentBuilder::new().build();
        let response = agent.get(&url).call().expect("failed to fetch Suno page");
        let http_status = response.status();
        let mut html = String::new();
        response
            .into_reader()
            .read_to_string(&mut html)
            .expect("failed to read Suno page response");
        let report = inspect_suno_metadata_html(&url, &id, http_status, &html);
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("src-tauri should have a parent directory");
        let debug_dir = repo_root.join("debug");
        fs::create_dir_all(&debug_dir).expect("failed to create debug dir");
        let report_path = debug_dir.join(format!("suno_metadata_inspection_{id}.json"));
        let report_json =
            serde_json::to_string_pretty(&report).expect("failed to serialize inspection report");
        fs::write(&report_path, report_json).expect("failed to write inspection report");

        println!("wrote {}", report_path.display());
        println!(
            "title candidates: {}",
            report
                .candidate_values
                .get("title")
                .map(|values| values.len())
                .unwrap_or(0)
        );
        println!(
            "lyrics candidates: {}",
            report
                .candidate_values
                .get("lyrics")
                .map(|values| values.len())
                .unwrap_or(0)
        );
        println!(
            "prompt candidates: {}",
            report
                .candidate_values
                .get("prompt")
                .map(|values| values.len())
                .unwrap_or(0)
        );
        println!(
            "style candidates: {}",
            report
                .candidate_values
                .get("style")
                .map(|values| values.len())
                .unwrap_or(0)
        );
        println!(
            "image candidates: {}",
            report
                .candidate_values
                .get("image_url")
                .map(|values| values.len())
                .unwrap_or(0)
        );
        println!(
            "audio candidates: {}",
            report
                .candidate_values
                .get("audio_url")
                .map(|values| values.len())
                .unwrap_or(0)
        );
    }

    #[test]
    fn inspects_metadata_report_from_html_without_network() {
        let html = r#"
            <html><head>
                <meta property="og:title" content="Meta title">
                <meta property="og:image" content="https://cdn.test/image.jpeg">
                <script id="__NEXT_DATA__" type="application/json">
                {"props":{"clip":{"title":"JSON title","lyrics":"words","prompt":"make it bright","style":"pop","audio_url":"https://cdn.test/audio.mp3","image_url":"https://cdn.test/cover.jpeg","tags":["tag-a"]}}}
                </script>
                <script>self.__next_f.push(["payload"])</script>
                <script>self.__next_f.push([1,"{\"title\":\"RSC title\",\"prompt\":\"RSC prompt\",\"audio_url\":\"https://cdn.test/rsc.mp3\",\"id\":\"daf62c51\"}"])</script>
            </head></html>
        "#;

        let report = inspect_suno_metadata_html("https://suno.com/s/id", "id", 200, html);

        assert_eq!(report.http_status, 200);
        assert!(report.has_next_data);
        assert!(report.has_next_f_payload);
        assert_eq!(report.json_script_count, 1);
        assert_eq!(report.next_payload_count, 2);
        assert!(report.next_payloads.iter().any(|payload| payload
            .matches
            .iter()
            .any(|found| found.target == "daf62c51")));
        assert!(report
            .candidate_values
            .get("lyrics")
            .is_some_and(|values| values.iter().any(|value| value == "words")));
        assert!(report
            .candidate_values
            .get("prompt")
            .is_some_and(|values| values.iter().any(|value| value == "make it bright")));
        assert!(report
            .candidate_values
            .get("style")
            .is_some_and(|values| values.iter().any(|value| value == "pop")));
    }

    #[test]
    fn masks_sensitive_values() {
        assert_eq!(mask_secret("Bearer abcdefghij"), "Bearer abc...hij");
        assert_eq!(mask_secret("abcdefghi"), "abc...ghi");
        assert_eq!(mask_secret("abc"), "***");
        assert_eq!(mask_secret(""), "");
    }

    #[test]
    fn detects_suno_api_settings() {
        assert!(!has_suno_api_settings(&AppSettings::default()));
        assert!(has_suno_api_settings(&AppSettings {
            suno_authorization: "Bearer token".to_string(),
            suno_browser_token: "browser".to_string(),
            suno_device_id: "device".to_string(),
            ..AppSettings::default()
        }));
    }

    #[test]
    fn finds_matching_clip_by_id() {
        let clips = vec![
            SunoMetadata {
                id: Some("a".to_string()),
                title: Some("First".to_string()),
                ..SunoMetadata::default()
            },
            SunoMetadata {
                id: Some("b".to_string()),
                title: Some("Second".to_string()),
                ..SunoMetadata::default()
            },
        ];

        assert_eq!(
            find_matching_clip(&clips, "b").and_then(|clip| clip.title),
            Some("Second".to_string())
        );
        assert_eq!(find_matching_clip(&clips, "c"), None);
    }

    #[test]
    fn empty_feed_result_stops_pagination_path() {
        let value = serde_json::json!([]);
        assert!(clips_from_feed_value(&value).is_empty());
    }

    #[test]
    fn selects_metadata_audio_when_available() {
        let metadata = SunoMetadata {
            audio_url: Some("https://example.test/audio.mp3".to_string()),
            ..SunoMetadata::default()
        };

        assert_eq!(
            select_audio_source(Some(&metadata), "https://cdn1.suno.ai/id.mp3"),
            "https://example.test/audio.mp3"
        );
        assert_eq!(
            select_audio_source(None, "https://cdn1.suno.ai/id.mp3"),
            "https://cdn1.suno.ai/id.mp3"
        );
    }

    #[test]
    fn extracts_page_metadata_from_embedded_json() {
        let html = r#"
            <html><head>
            <script type="application/json">
            {
                "props": {
                    "clip": {
                        "title": "Page title",
                        "audio_url": "https://cdn.suno.test/audio.mp3",
                        "image_large_url": "https://cdn.suno.test/large.jpeg",
                        "lyrics": "la la",
                        "style_prompt": "cinematic pop"
                    }
                }
            }
            </script>
            </head></html>
        "#;

        let metadata = extract_page_metadata_from_html(html, "song-id").expect("metadata");

        assert_eq!(metadata.id, Some("song-id".to_string()));
        assert_eq!(metadata.title, Some("Page title".to_string()));
        assert_eq!(
            metadata.audio_url,
            Some("https://cdn.suno.test/audio.mp3".to_string())
        );
        assert_eq!(
            metadata.image_large_url,
            Some("https://cdn.suno.test/large.jpeg".to_string())
        );
        assert_eq!(metadata.lyrics, Some("la la".to_string()));
        assert_eq!(metadata.style, Some("cinematic pop".to_string()));
    }

    #[test]
    fn page_metadata_has_priority_when_merged_with_feed_metadata() {
        let page = SunoMetadata {
            audio_url: Some("https://page.test/audio.mp3".to_string()),
            image_large_url: Some("https://page.test/image.jpeg".to_string()),
            ..SunoMetadata::default()
        };
        let feed = SunoMetadata {
            audio_url: Some("https://feed.test/audio.mp3".to_string()),
            image_large_url: Some("https://feed.test/image.jpeg".to_string()),
            title: Some("Feed title".to_string()),
            ..SunoMetadata::default()
        };

        let merged = merge_metadata(page, Some(feed));

        assert_eq!(
            merged.audio_url,
            Some("https://page.test/audio.mp3".to_string())
        );
        assert_eq!(
            merged.image_large_url,
            Some("https://page.test/image.jpeg".to_string())
        );
        assert_eq!(merged.title, Some("Feed title".to_string()));
    }

    #[test]
    fn preserves_frontend_cover_priority() {
        let metadata = SunoMetadata {
            image_url: Some("https://example.test/image.jpeg".to_string()),
            image_large_url: Some("https://example.test/large.jpeg".to_string()),
            ..SunoMetadata::default()
        };

        assert_eq!(
            select_cover_source(
                Some("data:image/png;base64,abc"),
                Some(&metadata),
                "https://cdn2.suno.ai/id.jpeg"
            ),
            "data:image/png;base64,abc"
        );
        assert_eq!(
            select_cover_source(None, Some(&metadata), "https://cdn2.suno.ai/id.jpeg"),
            "https://example.test/large.jpeg"
        );
        assert_eq!(
            select_cover_source(None, None, "https://cdn2.suno.ai/id.jpeg"),
            "https://cdn2.suno.ai/id.jpeg"
        );
    }

    #[test]
    fn validates_allowlists() {
        assert!(ensure_allowed("1280x720", ALLOWED_RESOLUTIONS, "resolution").is_ok());
        assert!(ensure_allowed("640x480", ALLOWED_RESOLUTIONS, "resolution").is_err());
        assert!(ensure_allowed("none", ALLOWED_VISUALIZERS, "visualizer").is_ok());
        assert!(ensure_allowed("spectrum", ALLOWED_VISUALIZERS, "visualizer").is_ok());
        assert!(ensure_allowed("combined", ALLOWED_VISUALIZERS, "visualizer").is_err());
    }

    #[test]
    fn builds_ffmpeg_as_args_not_shell() {
        let args = build_ffmpeg_args(
            Path::new("/tmp/audio file.mp3"),
            Path::new("/tmp/cover image.jpeg"),
            Path::new("/tmp/out file.mp4"),
            "1280x720",
            "none",
            "cpu_x264",
            "standard",
        );
        assert!(args.contains(&"/tmp/audio file.mp3".to_string()));
        assert!(args.contains(&"/tmp/cover image.jpeg".to_string()));
        assert!(!args.iter().any(|arg| arg.contains("showspectrum")));
        assert!(!args.iter().any(|arg| arg.contains("overlay")));
        assert!(args
            .iter()
            .any(|arg| arg.contains("[1:v]scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p[v]")));
        assert!(args.windows(2).any(|pair| pair == ["-map", "[v]"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "0:a"]));
    }

    #[test]
    fn builds_spectrum_filter_when_requested() {
        let args = build_ffmpeg_args(
            Path::new("/tmp/audio.mp3"),
            Path::new("/tmp/cover.jpeg"),
            Path::new("/tmp/out.mp4"),
            "1280x720",
            "spectrum",
            "cpu_x264",
            "standard",
        );

        assert!(args
            .iter()
            .any(|arg| arg.contains("showspectrum=s=1280x720:mode=combined")));
        assert!(args.iter().any(|arg| arg.contains("overlay=format=auto")));
        assert!(args
            .iter()
            .any(|arg| arg.contains("scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p[v]")));
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
            "none",
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
            "none",
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
