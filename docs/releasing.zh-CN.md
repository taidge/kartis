# 发布 Kartis

[English](releasing.md)

版本号维护在 `Cargo.toml`，发布使用指向已通过 CI 的精确提交的附注标签
`vX.Y.Z`。GitHub 仓库为 [`taidge/kartis`](https://github.com/taidge/kartis)。

打标签前更新版本、锁文件、安装示例和 `CHANGELOG.md`，推送提交并等待全部 CI
检查通过。CI 检查拼写、Rust 格式、原生测试及 Clippy、WebAssembly、最低 Rust
版本、文档和 crates.io 发布包。包依赖已发布的版本，无需相邻源码仓库。

发布固定使用 Rust/Cargo 1.95.0，使本地和 CI 使用相同 Cargo 版本生成发布包。
调整此版本时同步更新发布工作流。

首次发布可在 CI 通过后，通过已登录的 crates.io 账户执行
`cargo +1.95.0 publish --locked --all-features`。后续自动发布需在 crates.io 中配置 Trusted Publisher：
owner 为 `taidge`，repository 为 `kartis`，workflow 为 `release.yml`。
GitHub Actions 通过 OIDC 获取临时凭据，无需保存长期 registry token。
自动发布尚未上传的版本必须先完成此配置。

创建并推送版本标签：

```sh
git tag -a v0.2.0 -m "Kartis 0.2.0"
git push origin v0.2.0
```

发布工作流会在该标签上重新运行 CI，确认标签版本与清单一致，再生成 crate。
若该版本已经发布，必须验证其 SHA-256 与本次包完全相同；若尚未发布，则通过
OIDC 登录 crates.io 并上传。只有 registry 验证成功后，才创建 GitHub Release
并附上 crate 和 `SHA256SUMS`。已撤回或校验和不一致的版本会阻止发布。

如果发布后某个步骤失败，可重新运行失败任务，或手动触发工作流并指定同一标签。
保持标签不变，crates.io 版本不可覆盖。也可以单独执行验收：

```sh
cargo +1.95.0 package --locked --all-features
python scripts/release.py verify --tag v0.2.0 --require-published
```

registry 包包含库源码、示例、文档、字体及许可证。大型开发参考数据保留在
GitHub，CI 从仓库运行完整测试。
