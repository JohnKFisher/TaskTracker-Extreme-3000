const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const VERSION_PATH = path.join(ROOT, 'version.json');
const PACKAGE_JSON_PATH = path.join(ROOT, 'package.json');
const CARGO_TOML_PATH = path.join(ROOT, 'src-tauri', 'Cargo.toml');
const TAURI_CONF_PATH = path.join(ROOT, 'src-tauri', 'tauri.conf.json');

function parseManifest(raw) {
  const parsed = JSON.parse(raw);
  const version = parsed.marketingVersion;
  const buildNumber = parsed.buildNumber;

  if (!/^\d+\.\d+\.\d+$/.test(version || '')) {
    throw new Error(`Invalid marketingVersion "${version}". Expected x.y.z.`);
  }

  if (!Number.isInteger(buildNumber) || buildNumber < 1) {
    throw new Error(`Invalid buildNumber "${buildNumber}". Expected a positive integer.`);
  }

  return { marketingVersion: version, buildNumber };
}

function readVersionManifest(versionPath = VERSION_PATH) {
  return parseManifest(fs.readFileSync(versionPath, 'utf8'));
}

function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function syncVersionFiles({
  root = ROOT,
  manifest = readVersionManifest(path.join(root, 'version.json')),
  checkOnly = false,
} = {}) {
  const packageJsonPath = path.join(root, 'package.json');
  const cargoTomlPath = path.join(root, 'src-tauri', 'Cargo.toml');
  const tauriConfPath = path.join(root, 'src-tauri', 'tauri.conf.json');

  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  const tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'));
  const cargoToml = fs.readFileSync(cargoTomlPath, 'utf8');

  const nextPackageJson = { ...packageJson, version: manifest.marketingVersion };
  const nextTauriConf = { ...tauriConf, version: manifest.marketingVersion };
  const nextCargoToml = cargoToml.replace(
    /^version = ".*"$/m,
    `version = "${manifest.marketingVersion}"`,
  );

  const packageChanged = JSON.stringify(packageJson) !== JSON.stringify(nextPackageJson);
  const tauriChanged = JSON.stringify(tauriConf) !== JSON.stringify(nextTauriConf);
  const cargoChanged = cargoToml !== nextCargoToml;

  if (checkOnly) {
    if (packageChanged || tauriChanged || cargoChanged) {
      throw new Error('Versioned files are out of sync with version.json. Run npm run version:sync.');
    }
    return manifest;
  }

  if (packageChanged) writeJson(packageJsonPath, nextPackageJson);
  if (tauriChanged) writeJson(tauriConfPath, nextTauriConf);
  if (cargoChanged) fs.writeFileSync(cargoTomlPath, nextCargoToml);

  return manifest;
}

function bumpPatchVersion(currentVersion) {
  const [major, minor, patch] = currentVersion.split('.').map(Number);
  return `${major}.${minor}.${patch + 1}`;
}

function bumpManifest({
  root = ROOT,
  versionPath = path.join(root, 'version.json'),
} = {}) {
  const manifest = readVersionManifest(versionPath);
  const nextManifest = {
    marketingVersion: bumpPatchVersion(manifest.marketingVersion),
    buildNumber: manifest.buildNumber + 1,
  };
  writeJson(versionPath, nextManifest);
  return nextManifest;
}

module.exports = {
  ROOT,
  VERSION_PATH,
  PACKAGE_JSON_PATH,
  CARGO_TOML_PATH,
  TAURI_CONF_PATH,
  parseManifest,
  readVersionManifest,
  syncVersionFiles,
  bumpPatchVersion,
  bumpManifest,
};
