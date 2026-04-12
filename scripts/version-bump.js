const { bumpManifest, syncVersionFiles } = require('./version-lib');

try {
  const manifest = bumpManifest();
  syncVersionFiles({ manifest });
  console.log(`Bumped version to ${manifest.marketingVersion} (${manifest.buildNumber}).`);
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
