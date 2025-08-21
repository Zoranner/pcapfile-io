# Git Hooks 配置

这个目录包含了 pcapfile-io 项目的 Git hooks 安装脚本，用于确保代码质量和一致性。

## 功能

### Pre-commit Hook

在每次 `git commit` 前自动执行以下检查：

1. **代码格式检查** - `cargo fmt --all -- --check`
   - 确保代码符合 Rust 标准格式
   - 如果格式不正确，提示运行 `cargo fmt --all` 修复

2. **代码静态分析** - `cargo clippy --all-targets --all-features -- -D warnings`
   - 检查代码中的潜在问题和不良实践
   - 验证代码能够成功编译
   - 所有 Clippy 警告都被视为错误

如果任何检查失败，提交将被阻止。

## 安装

### Windows (PowerShell)

```powershell
# 在项目根目录运行
.\hooks\install-hooks.ps1
```

## 使用

安装后，hooks 会在每次提交时自动运行：

```bash
git add .
git commit -m "你的提交信息"  # 会自动运行检查
```

## 相关命令

```bash
# 手动运行格式检查
cargo fmt --all -- --check

# 自动修复格式
cargo fmt --all

# 手动运行 Clippy 检查
cargo clippy --all-targets --all-features -- -D warnings
```
