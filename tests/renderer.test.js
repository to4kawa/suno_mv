const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');

test('renderer uses Tauri invoke instead of Electron preload API', () => {
  const renderer = fs.readFileSync('src/renderer.js', 'utf8');
  assert.match(renderer, /__TAURI__\.core/);
  assert.doesNotMatch(renderer, /electronAPI/);
});
