const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (id) => document.getElementById(id);
const log = (line) => { $('log').textContent += `${line}\n`; $('log').scrollTop = $('log').scrollHeight; };
let currentClip = null;
let customImage = null;

listen('mv-log', (event) => log(event.payload));

async function readImage(file) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result);
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}

async function setCustomImage(file) {
  if (!file?.type?.startsWith('image/')) return alert('画像ファイルを選択してください');
  customImage = await readImage(file);
  $('preview-img').src = customImage;
}

$('file-input').addEventListener('change', (e) => setCustomImage(e.target.files[0]));
$('drop-area').addEventListener('dragover', (e) => { e.preventDefault(); });
$('drop-area').addEventListener('drop', (e) => { e.preventDefault(); setCustomImage(e.dataTransfer.files[0]); });

$('fetch-btn').addEventListener('click', async () => {
  $('log').textContent = '';
  customImage = null;
  try {
    currentClip = await invoke('fetch_clip', { req: { url: $('url-input').value.trim(), cookie: $('cookie-input').value.trim() || null } });
    $('preview-img').src = currentClip.image_url || '';
    $('meta').innerHTML = `<p><b>clip_id:</b> ${currentClip.clip_id}</p><p><b>title:</b> ${currentClip.title || '(untitled)'}</p><p><b>audio_url:</b> ${currentClip.audio_url || '(missing)'}</p><p><b>image_url:</b> ${currentClip.image_url || '(missing)'}</p>`;
    log('metadata fetched');
  } catch (e) { log(`ERROR: ${e}`); alert(e); }
});

$('generate-btn').addEventListener('click', async () => {
  $('log').textContent = '';
  try {
    const path = await invoke('generate_mp4', { req: { url: $('url-input').value.trim(), cookie: $('cookie-input').value.trim() || null, image_data_url: customImage, resolution: $('resolution').value, visualizer: $('visualizer').value, output_dir: $('output-dir').value.trim() || null } });
    log(`完了: ${path}`);
    alert(`完了: ${path}`);
  } catch (e) { log(`ERROR: ${e}`); alert(e); }
});

$('cancel-btn').addEventListener('click', async () => { await invoke('cancel_generate'); log('キャンセル要求を送信しました'); });
