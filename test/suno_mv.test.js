const assert = require('assert');
const path = require('path');
const {
  extractSunoId,
  buildAssetUrls,
  buildOutputPaths,
  buildFfmpegCommand,
} = require('../src/suno_mv');

function test(name, fn) {
  try {
    fn();
    console.log(`PASS ${name}`);
  } catch (error) {
    console.error(`FAIL ${name}`);
    console.error(error.stack || error.message);
    process.exitCode = 1;
  }
}

test('extractSunoId accepts a standard Suno song URL', () => {
  assert.strictEqual(
    extractSunoId('https://suno.com/song/123e4567-e89b-12d3-a456-426614174000'),
    '123e4567-e89b-12d3-a456-426614174000',
  );
});

test('extractSunoId rejects non-song URLs', () => {
  assert.strictEqual(extractSunoId('https://suno.com/create'), null);
  assert.strictEqual(extractSunoId('not a url'), null);
});

test('buildAssetUrls sanitizes the ID before constructing CDN URLs', () => {
  assert.deepStrictEqual(buildAssetUrls('abc-123_../evil'), {
    safeId: 'abc-123_evil',
    mp3Url: 'https://cdn1.suno.ai/abc-123_evil.mp3',
    defaultImageUrl: 'https://cdn2.suno.ai/image_large_abc-123_evil.jpeg',
  });
});

test('buildOutputPaths keeps generated files inside the output directory', () => {
  const outputPath = path.join('tmp', 'out');
  assert.deepStrictEqual(buildOutputPaths(outputPath, 'abc-123'), {
    mp3Path: path.join(outputPath, 'abc-123.mp3'),
    imgPath: path.join(outputPath, 'abc-123.jpeg'),
    mp4Path: path.join(outputPath, 'abc-123.mp4'),
  });
});

test('buildFfmpegCommand includes resolution, visualizer, and quoted paths', () => {
  const command = buildFfmpegCommand({
    mp3Path: 'tmp out/song.mp3',
    imgPath: 'tmp out/song.jpeg',
    mp4Path: 'tmp out/song.mp4',
    resolution: '1920x1080',
    visualizer: 'combined',
  });

  assert(command.includes('showspectrum=s=1920x1080:mode=combined'));
  assert(command.includes('"tmp out/song.mp3"'));
  assert(command.includes('"tmp out/song.jpeg"'));
  assert(command.includes('"tmp out/song.mp4"'));
});
