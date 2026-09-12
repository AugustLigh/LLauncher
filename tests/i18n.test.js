import test from 'node:test';
import assert from 'node:assert';
import { BUNDLES, resolveLocale } from '../src/i18n/resolve.js';

test('resolveLocale behavior', () => {
  assert.strictEqual(resolveLocale('en-us'), 'en');
  assert.strictEqual(resolveLocale('en'), 'en');
  assert.strictEqual(resolveLocale('ru-RU'), 'ru');
  assert.strictEqual(resolveLocale('ru'), 'ru');
  assert.strictEqual(resolveLocale('zh-cn'), 'zh-cn');
  assert.strictEqual(resolveLocale('zh-tw'), 'zh-tw');
  assert.strictEqual(resolveLocale('ja'), 'ja-jp');
  assert.strictEqual(resolveLocale('ja-JP'), 'ja-jp');
  assert.strictEqual(resolveLocale('id'), 'id-id');
  assert.strictEqual(resolveLocale('id-ID'), 'id-id');
  assert.strictEqual(resolveLocale('ko'), 'ko-kr');
  assert.strictEqual(resolveLocale('ko-KR'), 'ko-kr');
  assert.strictEqual(resolveLocale('de'), 'de-de');
  assert.strictEqual(resolveLocale('fr'), 'fr-fr');
  assert.strictEqual(resolveLocale('es'), 'es-es');
  assert.strictEqual(resolveLocale('vi'), 'vi-vn');
  assert.strictEqual(resolveLocale('th'), 'th-th');
  assert.strictEqual(resolveLocale('pt'), 'pt-br');
  assert.strictEqual(resolveLocale('pt-br'), 'pt-br');
  assert.strictEqual(resolveLocale('zh-tw'), 'zh-tw');
  assert.strictEqual(resolveLocale('zh-hk'), 'zh-tw');
  assert.strictEqual(resolveLocale('zh-hant'), 'zh-tw');
  assert.strictEqual(resolveLocale('zh-cn'), 'zh-cn');
  assert.strictEqual(resolveLocale('fr-fr'), 'fr-fr');
  assert.strictEqual(resolveLocale('it'), 'en');
  assert.strictEqual(resolveLocale(null), 'en');
});

test('i18n parity: all keys in en.js must exist in other bundles', () => {
  const enBundle = BUNDLES.en;
  
  function checkKeys(obj1, obj2, path) {
    for (const key in obj1) {
      if (typeof obj1[key] === 'object' && obj1[key] !== null) {
        assert.ok(obj2[key] !== undefined, `Missing nested object ${path}.${key}`);
        checkKeys(obj1[key], obj2[key], `${path}.${key}`);
      } else {
        assert.ok(obj2[key] !== undefined, `Missing key ${path}.${key}`);
      }
    }
  }

  checkKeys(enBundle, BUNDLES.ru, 'ru');
  checkKeys(enBundle, BUNDLES['zh-cn'], 'zh-cn');
  checkKeys(enBundle, BUNDLES['zh-tw'], 'zh-tw');
  checkKeys(enBundle, BUNDLES['ja-jp'], 'ja-jp');
  checkKeys(enBundle, BUNDLES['id-id'], 'id-id');
  checkKeys(enBundle, BUNDLES['ko-kr'], 'ko-kr');
  checkKeys(enBundle, BUNDLES['de-de'], 'de-de');
  checkKeys(enBundle, BUNDLES['fr-fr'], 'fr-fr');
  checkKeys(enBundle, BUNDLES['es-es'], 'es-es');
  checkKeys(enBundle, BUNDLES['pt-br'], 'pt-br');
  checkKeys(enBundle, BUNDLES['vi-vn'], 'vi-vn');
  checkKeys(enBundle, BUNDLES['th-th'], 'th-th');
});
