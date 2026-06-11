# Security Policy

## EasyInventory v1.3.2

EasyInventory is a local-first Windows desktop application. Business data is stored in the local SQLite database and is not uploaded by the application.

## Reporting Issues

反馈问题前请脱敏。提交 Issue、截图、日志或诊断包时，请先检查并移除不希望公开的信息，包括但不限于：

- 客户、供应商、联系人和商户名称。
- 电话、地址、门店位置和本机用户名。
- 真实订单、库存数量、进货价、售价、利润和欠款金额。
- 本机完整文件路径、数据库路径、备份路径和导出路径。

v1.3.2 会对运行日志和诊断包中的常见敏感字段做默认脱敏，但自动脱敏不能替代人工检查。

## Local Data Boundary

- 数据库、备份、单据、导出文件和日志默认保存在本机应用数据目录。
- 应用不提供云同步、远程登录或生产环境 API。
- 备份和诊断包由用户自行决定是否分享；分享前请确认内容已脱敏。

## Supported Security Scope

Current security work focuses on:

- Redacted logs and diagnostic exports.
- SQLite consistent backups under WAL mode.
- Runtime SQLite lock waiting through `busy_timeout`.
- Clear release artifacts and SHA256 checksums.

Out of scope for v1.3.2:

- Multi-user permission systems.
- Cloud account security.
- Remote server hardening.
- Regulatory or accounting compliance certification.
