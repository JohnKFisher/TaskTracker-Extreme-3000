const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const os = require('os');
const path = require('path');

const {
  parseManifest,
  syncVersionFiles,
  bumpManifest,
} = require('../scripts/version-lib');

function createFixtureRoot() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'tasktracker-version-'));
  fs.mkdirSync(path.join(root, 'src-tauri'), { recursive: true });

  fs.writeFileSync(
    path.join(root, 'version.json'),
    `${JSON.stringify({ marketingVersion: '2.0.0', buildNumber: 4 }, null, 2)}\n`,
  );
  fs.writeFileSync(
    path.join(root, 'package.json'),
    `${JSON.stringify({ name: 'fixture', version: '1.0.0' }, null, 2)}\n`,
  );
  fs.writeFileSync(
    path.join(root, 'src-tauri', 'tauri.conf.json'),
    `${JSON.stringify({ version: '1.0.0' }, null, 2)}\n`,
  );
  fs.writeFileSync(
    path.join(root, 'src-tauri', 'Cargo.toml'),
    '[package]\nname = "fixture"\nversion = "1.0.0"\nedition = "2021"\n',
  );

  return root;
}

test('parseManifest validates version schema', () => {
  assert.deepEqual(
    parseManifest('{"marketingVersion":"2.0.0","buildNumber":4}'),
    { marketingVersion: '2.0.0', buildNumber: 4 },
  );
  assert.throws(() => parseManifest('{"marketingVersion":"2.0","buildNumber":4}'));
  assert.throws(() => parseManifest('{"marketingVersion":"2.0.0","buildNumber":0}'));
});

test('syncVersionFiles updates tracked version fields', () => {
  const root = createFixtureRoot();
  syncVersionFiles({ root });

  const packageJson = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
  const tauriConf = JSON.parse(fs.readFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'));
  const cargoToml = fs.readFileSync(path.join(root, 'src-tauri', 'Cargo.toml'), 'utf8');

  assert.equal(packageJson.version, '2.0.0');
  assert.equal(tauriConf.version, '2.0.0');
  assert.match(cargoToml, /^version = "2.0.0"$/m);
});

test('syncVersionFiles check mode fails when files drift', () => {
  const root = createFixtureRoot();
  assert.throws(
    () => syncVersionFiles({ root, checkOnly: true }),
    /out of sync/,
  );
});

test('bumpManifest increments patch version and build number', () => {
  const root = createFixtureRoot();
  const nextManifest = bumpManifest({ root });

  assert.deepEqual(nextManifest, {
    marketingVersion: '2.0.1',
    buildNumber: 5,
  });

  const saved = JSON.parse(fs.readFileSync(path.join(root, 'version.json'), 'utf8'));
  assert.deepEqual(saved, nextManifest);
});
