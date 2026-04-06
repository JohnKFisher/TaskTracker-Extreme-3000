// Increments the build counter and updates the build stamp in renderer/index.html.
// Run automatically via "prebuild" in package.json before every npm run build.

const fs = require('fs');
const path = require('path');

const numFile = path.join(__dirname, '..', 'data', 'build-number.json');
let num = 1;
try {
  num = JSON.parse(fs.readFileSync(numFile, 'utf-8')).number + 1;
} catch { /* first run or missing file — start at 1 */ }

fs.writeFileSync(numFile, JSON.stringify({ number: num }, null, 2));

const date = new Date().toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' });
const stamp = `Built: ${date}, #${num}`;

const htmlFile = path.join(__dirname, '..', 'renderer', 'index.html');
let html = fs.readFileSync(htmlFile, 'utf-8');
html = html.replace(/<div id="build-stamp">.*?<\/div>/, `<div id="build-stamp">${stamp}</div>`);
fs.writeFileSync(htmlFile, html);

console.log('Build stamp updated:', stamp);
