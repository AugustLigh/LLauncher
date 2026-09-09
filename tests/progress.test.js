import test from 'node:test';
import assert from 'node:assert/strict';
import { progressDetails } from '../src/utils/progress.js';

test('VFS verification advances without byte totals', () => {
  const p = progressDetails({ stage: 'verifying', files_done: 32, total_files: 64, bytes_total: 0, bytes_downloaded: 0 });
  assert.equal(p.percent, 50);
  assert.equal(p.byFiles, true);
  assert.equal(p.currentFile, 32);
});

test('completed VFS downloads do not show file N+1 of N', () => {
  const p = progressDetails({ stage: 'downloading', files_done: 4, total_files: 4, bytes_total: 100, bytes_downloaded: 100 });
  assert.equal(p.currentFile, 4);
  assert.equal(p.percent, 100);
});

test('pack downloads retain zero-based current file semantics', () => {
  const p = progressDetails({ stage: 'downloading', file_index: 0, total_files: 4, bytes_total: 100, bytes_downloaded: 25 });
  assert.equal(p.currentFile, 1);
  assert.equal(p.percent, 25);
  assert.equal(p.byFiles, false);
});

test('extraction uses processed bytes and the backend percentage', () => {
  const p = progressDetails({ stage: 'extracting', bytes_processed: 30, bytes_total: 100, percent: 30 });
  assert.equal(p.done, 30);
  assert.equal(p.percent, 30);
});

test('unknown totals and invalid or excessive progress remain bounded', () => {
  for (const input of [{}, { bytes_total: 100, bytes_downloaded: NaN }, { stage: 'extracting', percent: Infinity }]) {
    assert.equal(progressDetails(input).percent, 0);
  }
  assert.equal(progressDetails({ bytes_total: 100, bytes_downloaded: 110 }).percent, 100);
});
