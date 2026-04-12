const { syncVersionFiles } = require('./version-lib');

const checkOnly = process.argv.includes('--check');

try {
  const manifest = syncVersionFiles({ checkOnly });
  const message = checkOnly
    ? `Version files are in sync at ${manifest.marketingVersion} (${manifest.buildNumber}).`
    : `Synced version files to ${manifest.marketingVersion} (${manifest.buildNumber}).`;
  console.log(message);
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
