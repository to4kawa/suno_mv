const assert = require('node:assert/strict');
const fs = require('node:fs');
const test = require('node:test');

test('renderer invokes the registered Tauri generate_mp4 command', () => {
  const source = fs.readFileSync('public/renderer.js', 'utf8');
  assert.match(source, /invoke\("generate_mp4"/);
});

test('renderer passes resolved frontend cover data to generate_mp4', () => {
  const source = fs.readFileSync('public/renderer.js', 'utf8');
  assert.match(source, /const base64 = await resolveCoverBase64\(\)/);
  assert.match(source, /base64,/);
});

test('renderer stores dropped images as data urls for generation', () => {
  const source = fs.readFileSync('public/renderer.js', 'utf8');
  assert.match(source, /reader\.readAsDataURL\(file\)/);
  assert.match(source, /previewImg\.setAttribute\("data-base64", base64\)/);
});

test('renderer supports Suno short URL ids and metadata fetch command', () => {
  const source = fs.readFileSync('public/renderer.js', 'utf8');
  assert.match(source, /suno\\\.com\\\/\(\?:song\|s\)\\\//);
  assert.match(source, /invoke\("fetch_suno_metadata"/);
});

test('renderer keeps frontend cover priority over metadata cover', () => {
  const source = fs.readFileSync('public/renderer.js', 'utf8');
  assert.match(source, /const selectCoverSource = \(frontendData, metadata, fallback\) => \{/);
  assert.match(source, /if \(frontendData\) return frontendData;/);
});

test('renderer passes metadata sources as optional generation fallbacks', () => {
  const source = fs.readFileSync('public/renderer.js', 'utf8');
  assert.match(source, /const metadataForRequest = currentSongId === id \? currentMetadata : null/);
  assert.match(source, /audioUrl: metadataForRequest\?\.audioUrl \|\| null/);
  assert.match(source, /metadataImageUrl: metadataForRequest\?\.imageLargeUrl \|\| metadataForRequest\?\.imageUrl \|\| null/);
  assert.match(source, /outputFilename: metadataForRequest\?\.title \|\| null/);
});

test('renderer attempts metadata command even without Suno API settings', () => {
  const source = fs.readFileSync('public/renderer.js', 'utf8');
  assert.doesNotMatch(source, /metadata fetch skipped: Suno API settings are not configured/);
  assert.match(source, /const result = await invoke\("fetch_suno_metadata", \{ url \}\)/);
});

test('renderer logs do not call cover CDN request warm-up', () => {
  const renderer = fs.readFileSync('public/renderer.js', 'utf8');
  const rust = fs.readFileSync('src-tauri/src/lib.rs', 'utf8');
  assert.doesNotMatch(renderer, /Warm-up cover request|warm-up cover/i);
  assert.doesNotMatch(rust, /Warm-up cover request|warm-up cover/i);
});
