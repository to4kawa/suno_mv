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
