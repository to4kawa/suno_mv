const assert = require('node:assert/strict');
const fs = require('node:fs');
const test = require('node:test');

test('renderer invokes the registered Tauri generate_mp4 command', () => {
  const source = fs.readFileSync('public/renderer.js', 'utf8');
  assert.match(source, /invoke\("generate_mp4"/);
});
