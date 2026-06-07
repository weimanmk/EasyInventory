# EasyInventory Release Checklist

## 发布前命令

```powershell
npm install
npm run release:verify
```

`release:verify` 会依次执行完整质量检查、Tauri 打包、安装包 smoke 检查和 Release 清单生成。

## 必传产物

- `src-tauri/target/release/bundle/nsis/EasyInventory_<version>_x64-setup.exe`
- `src-tauri/target/release/easyinventory.exe`
- `release/EasyInventory_<version>_release_manifest.md` 中的 SHA256 校验值

## GitHub Release 说明模板

```markdown
## EasyInventory <version>

### 下载

- Windows 安装包：EasyInventory_<version>_x64-setup.exe
- 便携 EXE：easyinventory.exe

### SHA256

| 文件 | SHA256 |
|---|---|
| EasyInventory_<version>_x64-setup.exe | <从 release manifest 复制> |
| easyinventory.exe | <从 release manifest 复制> |

### 升级说明

- 升级前请在系统设置页执行一次手动备份。
- 建议保留旧安装包一段时间，确认新版本运行稳定后再清理。
- 数据库保存在本机应用数据目录，安装新版本不会自动上传或迁移到云端。

### 已知问题

- 早期发布的 Tauri/NSIS 安装包可能被部分安全软件误报。
- 首次启动如被拦截，请确认下载来源和 SHA256 校验值后再放行。
- 当前版本定位为 Windows 单机软件，不支持多人实时协作。

### 验证

- npm run check
- npm run package:smoke
- npm run release:manifest
```

## 发布后检查

- GitHub Release 页面包含安装包、便携 EXE、版本说明和 SHA256。
- README 下载说明链接到 Releases 页面。
- 新版本启动后可完成初始化、入库、出库、单据导出和备份。
