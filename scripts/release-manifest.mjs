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
  '- 本版本修复 9.5 × 5.5 英寸连续纸分页和同路径单据档案重复问题。',
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
  '这次让连续纸知道哪里该停 :)',
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
  '- 出库单明确写入 9.5 × 5.5 英寸、横向、单页打印参数，不再靠猜缩放比例。',
  '- 打印区域固定为 A1:K21，并按已验证的标定方案宽高各适配一页。',
  '- 同一路径、同一类型的单据档案只保留一条，历史重复记录会在升级时自动合并。',
  '',
  '### 升级说明',
  '',
  '- 升级前请在系统设置页执行一次手动备份。',
  '- 已经导出的旧 XLSX 不会自动改变，请安装新版后重新导出单据再打印。',
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
