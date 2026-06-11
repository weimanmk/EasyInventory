import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const pkg = JSON.parse(readFileSync(path.join(root, 'package.json'), 'utf8'));
const targetRoot = process.env.CARGO_TARGET_DIR
  ? path.resolve(process.env.CARGO_TARGET_DIR)
  : path.join(root, 'src-tauri', 'target');
const releaseDir = path.join(root, 'release');

const artifacts = [
  path.join(targetRoot, 'release', 'easyinventory.exe'),
  path.join(targetRoot, 'release', 'bundle', 'nsis', `EasyInventory_${pkg.version}_x64-setup.exe`)
];

const missing = artifacts.filter((artifact) => !existsSync(artifact) || statSync(artifact).size === 0);

if (missing.length > 0) {
  console.error('Release 清单生成失败，缺少 release 产物：');
  for (const artifact of missing) {
    console.error(`- ${artifact}`);
  }
  process.exit(1);
}

function sha256(filePath) {
  return createHash('sha256').update(readFileSync(filePath)).digest('hex');
}

function sizeMb(filePath) {
  return (statSync(filePath).size / 1024 / 1024).toFixed(2);
}

mkdirSync(releaseDir, { recursive: true });

const rows = artifacts.map((artifact) => ({
  name: path.basename(artifact),
  path: path.relative(root, artifact).replaceAll(path.sep, '/'),
  sizeMb: sizeMb(artifact),
  sha256: sha256(artifact)
}));

const body = [
  `# EasyInventory v${pkg.version} Release 清单`,
  '',
  '## 产物',
  '',
  '| 文件 | 大小 | SHA256 |',
  '|---|---:|---|',
  ...rows.map((row) => `| ${row.name} | ${row.sizeMb} MB | \`${row.sha256}\` |`),
  '',
  '## Release 说明模板',
  '',
  '- 本版本是数据安全与可靠性补丁：日志更克制，备份更靠谱，数据库写入更稳。',
  '- 安装包：下载 NSIS setup exe，按向导安装。',
  '- 便携 EXE：适合本地 smoke test，不推荐普通用户直接替代安装包。',
  '- 升级前请在系统设置页执行一次手动备份。',
  '- 已知问题：首次启动如被安全软件拦截，请确认来源和 SHA256 后放行。',
  '',
  '## 本地路径',
  '',
  ...rows.map((row) => `- ${row.path}`)
].join('\n');

const output = path.join(releaseDir, `EasyInventory_${pkg.version}_release_manifest.md`);
writeFileSync(output, `${body}\n`, 'utf8');

const releaseNotes = [
  `## EasyInventory v${pkg.version}`,
  '',
  '### 下载',
  '',
  `- Windows 安装包：EasyInventory_${pkg.version}_x64-setup.exe`,
  '- 便携 EXE：easyinventory.exe',
  '',
  '### SHA256',
  '',
  '| 文件 | SHA256 |',
  '|---|---|',
  ...rows.map((row) => `| ${row.name} | \`${row.sha256}\` |`),
  '',
  '### 这次修了什么',
  '',
  '- 日志终于学会少说点了：命令参数默认只记摘要，常见电话、地址、名称和路径会脱敏。',
  '- 备份不再只会复制主数据库文件：自动备份、手动备份和恢复前快照都走 SQLite 一致性备份。',
  '- 数据库遇到短暂写锁会先等一等，减少“database is locked”这类瞬时失败。',
  '',
  '### 升级说明',
  '',
  '- 升级前请在系统设置页执行一次手动备份。',
  '- 反馈问题前仍建议检查诊断包内容，避免上传真实客户、电话、地址、订单和库存金额。',
  '- 建议保留旧安装包一段时间，确认新版本运行稳定后再清理。',
  '- 数据库保存在本机应用数据目录，安装新版本不会自动上传或迁移到云端。',
  '',
  '### 已知问题',
  '',
  '- 早期发布的 Tauri/NSIS 安装包可能被部分安全软件误报。',
  '- 首次启动如被拦截，请确认下载来源和 SHA256 校验值后再放行。',
  '- 当前版本定位为 Windows 单机软件，不支持多人实时协作。',
  '',
  '### 验证',
  '',
  '- npm run check',
  '- npm run tauri:build',
  '- npm run package:smoke',
  '- npm run release:manifest'
].join('\n');

const notesOutput = path.join(releaseDir, `EasyInventory_${pkg.version}_github_release_notes.md`);
writeFileSync(notesOutput, `${releaseNotes}\n`, 'utf8');

console.log(`Release 清单已生成：${output}`);
console.log(`GitHub Release 说明已生成：${notesOutput}`);
