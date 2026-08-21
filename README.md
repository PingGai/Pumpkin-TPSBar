# TPSBar

[简体中文](README.md) | [English](README.en.md)

TPSBar 是一个面向 Pumpkin 服务端的 Rust/WASM 插件。玩家可用 `/tpsbar` 独立切换自己的性能 BossBar；选择会写入插件私有数据目录，并在重新登录或重启后保留。

![TPSBar 效果展示](assets/screenshots/tpsbar-example.png)

本项目是 PING 开发的第三方 Pumpkin 插件，与 PumpkinMC 官方组织无隶属、赞助或背书关系。

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

TPSBar 根据每名玩家客户端上报的 locale 选择消息，而不是用配置文件强制全服共用一种语言。v0.1.0 内置 `zh_cn` 与 `en_us`；`fallback_locale` 只在客户端语言不受支持时生效。

Pumpkin 已提供官方 I18N WIT，但当前宿主端的枚举到 locale 字符串转换会让许多语言意外回退到英语。因此本版本使用官方 `Player.get_locale()` 获取玩家语言，再读取插件内置语言表；待官方转换层修复并稳定后可迁移到统一翻译注册接口。

## 构建

要求 Rust 1.95 或更高版本，并安装 `wasm32-wasip2` 目标。项目已提供 `rust-toolchain.toml` 固定经过验证的工具链：

Plugin API 固定使用 Pumpkin 提交 `0844e929112d5cda772bc8b0de51e38930142704` 的官方 Git 依赖，确保本地与 CI 使用同一套 API。

```powershell
rustup target add wasm32-wasip2
cargo +1.95.0 fmt --check
cargo +1.95.0 test --target x86_64-pc-windows-msvc
cargo +1.95.0 clippy --target x86_64-pc-windows-msvc --all-targets -- -D warnings
cargo +1.95.0 build --release
```

`.cargo/config.toml` 已将默认目标设为 `wasm32-wasip2`。Cargo 原始构建产物位于 `target/wasm32-wasip2/release/tpsbar.wasm`；GitHub Actions 和 Release 会复制为带版本号的 `tpsbar-v0.1.wasm`。

GitHub Actions 会在 push、Pull Request 和手动触发时运行格式检查、测试、Clippy，并上传可下载的 WASM 构建产物。

### 工具链兼容性说明

截至 Pumpkin 提交 `0844e929112d5cda772bc8b0de51e38930142704`，官方 WIT 含 `generic-9x1` 等不符合 Component Model kebab-case 规则的标识符。默认 Rust 1.90 工具链中的 `wasm-component-ld 0.5.15` 会在最终组件化时拒绝该 WIT；本项目已验证 Rust 1.95 的 `wasm-component-ld 0.5.21` 可以生成可加载的 Release WASM。静态扫描在五个 WIT 文件中找到 91 个同类候选，首个报错并非孤例。

这不是 TPSBar 业务代码错误。Rust 1.95 已验证可以生成可加载的 Release WASM；若使用旧 linker，请升级 Rust 工具链。插件仍必须和目标 Pumpkin 服务端使用匹配的 WIT/API。

## 安装

将 Release 中的 `tpsbar-v0.1.wasm` 放入 Pumpkin 的插件目录并启动服务端。首次加载时，Pumpkin 会请求批准插件的两个私有数据目录权限。插件只申请：

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
