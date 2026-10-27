# TPSBar

[简体中文](README.md) | [English](README.en.md)

TPSBar 是一个面向 Pumpkin 服务端的 Rust/WASM 插件。玩家可用 `/tpsbar` 独立切换自己的性能 BossBar；选择会写入插件私有数据目录，并在重新登录或重启后保留。

![TPSBar 效果展示](assets/screenshots/tpsbar-example.png)

本项目是由 PING 监督、设计功能并负责发布、由 **LLM 生成代码**的第三方 Pumpkin 插件，与 PumpkinMC 官方组织无隶属、赞助或背书关系。

```text
TPS: 20.00  MSPT: 19.24 ms  PING: 8 ms
```

界面布局参考 Purpur 的 TPSBar 使用习惯。

## 功能

- 默认关闭，只对主动执行 `/tpsbar` 的玩家显示。
- 每名玩家拥有独立 BossBar、Ping 和持久化开关状态。
- 从 Pumpkin 官方接口读取 TPS、MSPT 与 Ping，不修改世界或游戏逻辑。
- BossBar 血条默认按 MSPT 填充：`50 MSPT` 时填满；可在配置或 `/tpsbar by mspt|tps|ping` 间切换进度指标。
- TPS、MSPT、Ping 的数值按健康程度分别着色；标签和单位保持浅灰色。
- 默认每 20 tick 刷新一次；一次采样服务全部已启用玩家。
- 默认仅 Pumpkin 权限等级 3（管理员）及以上可用。
- 内置简体中文和英文消息，按客户端 locale 选择，未知语言回退到配置语言。
- 插件卸载、玩家离线或权限被撤销时，只移除本插件持有的 BossBar。

## 权限

| 节点 | 默认值 | 用途 |
| --- | --- | --- |
| `tpsbar:command.toggle` | `op level 3` | 切换自己的 TPSBar |

Pumpkin 当前要求插件权限使用 `插件命名空间:节点` 格式，所以这里不是 Bukkit/Paper 常见的点号节点。默认等级可在配置中设为 `0..4`；设为 `0` 等同普通玩家默认可用。

## 配置与数据

首次加载会在 Pumpkin 分配给插件的私有数据目录中生成 `config.toml`。完整默认值见 [`assets/config.default.toml`](assets/config.default.toml)。

MSPT 默认颜色区间：

- `[0, 35)`：绿色
- `[35, 50)`：黄色
- `[50, 80)`：金色
- `[80, +∞)`：红色

BossBar 进度默认配置为：

- `bar.metric = "mspt"`：MSPT 达到 `bar.mspt_full`（默认 50）时填满。
- `bar.metric = "tps"`：沿用目标 TPS（默认 20）作为满条值。
- `bar.metric = "ping"`：Ping 达到 `bar.ping_full`（默认 200）时填满。

管理员也可以在运行中执行 `/tpsbar by mspt`、`/tpsbar by tps` 或 `/tpsbar by ping` 立即切换。命令切换只影响当前运行实例；重启后以插件配置文件为准。

状态文件采用带 schema 版本的 JSON。保存使用临时文件与备份恢复流程，避免写入中断直接破坏已有选择。

## I18N 策略

TPSBar 根据每名玩家客户端上报的 locale 选择消息，而不是用配置文件强制全服共用一种语言。v0.2.0 内置 `zh_cn` 与 `en_us`；`fallback_locale` 只在客户端语言不受支持时生效。

TPSBar 根据每名玩家客户端上报的 locale，在简体中文和英文消息间选择；其他语言使用配置的 `fallback_locale`。

## 构建

要求 Rust 1.96 或更高版本，并安装 `wasm32-wasip2` 目标。项目通过 `rust-toolchain.toml` 固定为已验证的 Rust 1.97.1：

Plugin API 固定在 Pumpkin `0.2.1+26.3-26.51` 标签提交 `224f9f8144ca53a8d0800acb87607d714196e2a6`，本地与 CI 构建都会使用这份上游代码。

```powershell
rustup target add wasm32-wasip2
cargo +1.97.1 fmt --check
cargo +1.97.1 test --target x86_64-pc-windows-msvc
cargo +1.97.1 clippy --target x86_64-pc-windows-msvc --all-targets -- -D warnings
cargo +1.97.1 build --release
```

`.cargo/config.toml` 已将默认目标设为 `wasm32-wasip2`。`build/version.txt` 是唯一的发布线版本来源，当前为 `0.2`；Cargo 元数据固定为 `0.2.0`。Cargo 原始构建产物位于 `target/wasm32-wasip2/release/tpsbar.wasm`。

本地开发构建可运行 `\.\build\build.ps1`，生成带 12 位时间戳的快照到 `publish/`。正式构建可运行 `\.\build\build.ps1 -Release -OutputDirectory ..\..\OUTPUT\tpsbar`，生成 `tpsbar-v0.2.wasm`。GitHub Actions 对非 tag 构建使用快照版本，对匹配的 `v0.2` tag 使用正式版本。

GitHub Actions 会在 push、Pull Request 和手动触发时运行格式检查、测试、Clippy，并上传可下载的 WASM 构建产物。

### Pumpkin 兼容性

TPSBar v0.2 面向 Pumpkin `0.2.1+26.3-26.51`，并已在该版本的服务端完成实际加载验证。

## 安装

在 Pumpkin 可执行文件所在目录打开终端，运行以下命令通过 PPM 安装：

```powershell
ppm install 0Y9WQt4l
```

也可以将 Release 中的 `tpsbar-v0.2.wasm` 放入 Pumpkin 的插件目录并启动服务端。首次加载时，Pumpkin 会请求批准插件的两个私有数据目录权限。插件只申请：

- `fs.read.data`
- `fs.write.data`

它不申请网络、系统信息或插件私有目录之外的文件访问权限。

## 兼容性

- Java 客户端：支持标题中的分段富文本颜色。
- Bedrock 客户端：Pumpkin 当前会将 BossBar 标题转换为纯文本，数字仍可显示，BossBar 本身的颜色仍会变化，但标题分段颜色可能丢失。
- TPS 会按配置的目标 TPS 封顶，避免官方 `1000 / MSPT` 原始值在低 MSPT 时显示为超过 20。

## 许可证

Copyright 2025-2026 PING

项目使用 [Apache License 2.0](LICENSE)。第三方依赖摘要见 [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)，许可证正文归档在 `LICENSES/`。更新锁文件后运行：

```powershell
./scripts/update-licenses.ps1
```
