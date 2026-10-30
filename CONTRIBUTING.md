# 贡献与提交规范 / Contributing

从本规范加入仓库起，新提交采用 Conventional Commits，中文在前，英文在后。历史提交和已发布的 CHANGELOG 不追溯改写。

```text
<type>(<scope>): 中文描述 / English description
```

按改动目的选择 `feat`（功能）、`fix`（修复）、`refactor`（整理代码）、`perf`（性能）、`test`（测试）、`docs`（文档）、`build`（构建）、`ci`（自动检查）或 `chore`（维护）。scope 可省略。描述用日常语言说明改动；互不相关的类型分开提交。

```text
fix(commands): 让每名玩家单独保存进度指标 / Save the progress metric for each player
docs(readme): 补充个人指标设置说明 / Explain personal metric settings
```

New commits use Conventional Commits with Chinese first and English second, separated by ` / `. Choose a type that describes the change, keep unrelated purposes in separate commits, and use plain language. Existing commits and released changelog entries are preserved.

提交前运行 / Before committing:

```powershell
rustup run 1.97.1 cargo fmt --all -- --check
rustup run 1.97.1 cargo test --locked --target x86_64-pc-windows-msvc
rustup run 1.97.1 cargo clippy --locked --target x86_64-pc-windows-msvc --all-targets -- -D warnings
```

Linux 使用 `x86_64-unknown-linux-gnu` 主机目标。发布构建使用 `build/build.ps1 -Release`，并先运行 `scripts/update-licenses.ps1` 同步锁定依赖的许可证。普通构建不带 `-Release`，输出带时间戳的开发版。

On Linux, use the `x86_64-unknown-linux-gnu` host target. Run `scripts/update-licenses.ps1` before release builds, then `build/build.ps1 -Release`. Omit `-Release` for timestamped development builds.
