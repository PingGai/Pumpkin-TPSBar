# 更新记录

## v0.3 开发版 + Pumpkin 0.2.1 - 2026-10-30

- feat(commands): 新增带补全的 `/tpsbar empty=toggle`，与空参数共用个人开关，并提示可以省略子命令 / Add a suggested explicit toggle command and explain the shorter form.
- fix(preferences): `/tpsbar by mspt|tps|ping` 改为独立保存每名玩家的选择，兼容 0.2 开关数据 / Save personal progress metrics across restarts while preserving 0.2 toggle data.
- refactor(i18n): 语言目录分为玩家消息 `player` 和后台诊断 `diagnostic`，保存错误详情仅进入日志 / Separate player text from diagnostics and keep storage details in logs.
- fix(state): 修复损坏主文件覆盖有效备份的问题，先同步临时备份再替换 / Preserve valid recovery data and sync temporary backups before replacement.
- build(version): 插件内部显示带时间戳与 Pumpkin 版本的开发版本号 / Include the development timestamp and Pumpkin version in plugin metadata.

## v0.2+0.2.1 - 2026-10-28

- feat: 适配 Pumpkin 0.2.1 的插件生命周期与新版 SDK builder API，保留玩家独立开关、指标刷新和卸载清理。
- chore: 锁定 Pumpkin `0.2.1` Plugin API（`224f9f8144ca53a8d0800acb87607d714196e2a6`），将 Rust 工具链与 CI 升至 `1.97.1`。
- chore: 将正式产物命名为 `tpsbar-v0.2+0.2.1.wasm`，开发快照附带完整时间戳版本和 Pumpkin semver。
- docs: 双语 README 新增 PPM 安装命令 `ppm install 0Y9WQt4l`，更新 Pumpkin 兼容性和项目署名。

## v0.1+0.1.0-dev - 2026-08-21

- feat: 新增按玩家独立启用并持久化的 TPS BossBar。
- feat: 新增 TPS、MSPT、Ping 彩色数值显示。
- feat: 新增 `/tpsbar` 切换命令和 `tpsbar:command.toggle` 权限节点。
- feat: 新增刷新周期、目标 TPS、权限等级和颜色阈值配置。
- feat: 新增 BossBar 进度指标配置：默认按 MSPT 填充，50 MSPT 达到满条，并支持 `/tpsbar by mspt|tps|ping` 运行时切换。
- chore: 固定 Rust 1.95 构建工具链，避免旧版 `wasm-component-ld` 触发 Pumpkin WIT 组件化错误。
- feat: 新增简体中文与英文消息。
- docs: 新增默认简体中文、可互相跳转的中英双语 README 与市场发布文案。
- chore: 新增 Apache-2.0 主许可证和可重复生成的第三方许可证归档流程。
- chore: GitHub Actions 为 WASM 构建产物自动追加发布线版本号，并在 push、Pull Request、手动触发和 tag 构建中保持一致的命名规则。
- docs: README 安装说明改用带版本号的 `tpsbar-v0.1.wasm`，同时保留 Cargo 原始构建路径说明。
- chore: 将 v0.1 Release 的下载资产命名为 `tpsbar-v0.1.wasm`。
