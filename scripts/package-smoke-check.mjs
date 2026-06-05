import { existsSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const pkg = JSON.parse(readFileSync(path.join(root, 'package.json'), 'utf8'));
const targetRoot = process.env.CARGO_TARGET_DIR
  ? path.resolve(process.env.CARGO_TARGET_DIR)
  : path.join(root, 'src-tauri', 'target');

const artifacts = [
  path.join(targetRoot, 'release', 'easyinventory.exe'),
  path.join(targetRoot, 'release', 'bundle', 'nsis', `EasyInventory_${pkg.version}_x64-setup.exe`)
];

const missing = artifacts.filter((artifact) => !existsSync(artifact) || statSync(artifact).size === 0);

if (missing.length > 0) {
  console.error('安装包验收失败，缺少 release 产物：');
  for (const artifact of missing) {
    console.error(`- ${artifact}`);
  }
  process.exit(1);
}

console.log('安装包验收通过：');
for (const artifact of artifacts) {
  console.log(`OK ${artifact}`);
}
