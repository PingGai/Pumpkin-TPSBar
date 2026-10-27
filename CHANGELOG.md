# 更新记录

## v0.2+0.2.1 - 2026-10-28

- feat: 适配 Pumpkin 0.2.1 的插件生命周期与新版 SDK builder API，保留玩家独立开关、指标刷新和卸载清理。
- chore: 锁定 Pumpkin `0.2.1` Plugin API（`224f9f8144ca53a8d0800acb87607d714196e2a6`），将 Rust 工具链与 CI 升至 `1.97.1`。
- chore: 将发布线更新为 `0.2`，同步版本化 WASM 产物与第三方许可证归档。
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
