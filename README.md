# Rust 项目的初始化模版

开发项目时，会有不少关于项目的ci, lint, typo等配置，通过该模板创建项目，帮助快速构建这些配置：

- github action 配置，用于在CI对代码进行完整的检查和构建
- pre-commit 配置，用于在commit时进行代码规范检查，如: `cargo fmt  -- --check`, `cargo deny check -d`, `cargo clippy`, `typos`,等
- cargo deny 配置，用于检查rust依赖的安全性
- cliff 配置，用于自动生成CHANGELOG
- typo 配置，用于检查代码中的拼写错误

## 安装相关工具

### 安装 cargo generate

用于根据模板生成代码

```bash
cargo install cargo-generate
```

更多参考：<https://github.com/cargo-generate/cargo-generate>

### 安装 pre-commit

pre-commit 是一个代码检查工具，可以在提交代码前进行代码检查。

```bash
pip install pre-commit
```

更多参考： <https://pre-commit.com/>

### 安装 Cargo deny

用于检查依赖的安全性

```bash
cargo install --locked cargo-deny
```

更多参考： <https://github.com/EmbarkStudios/cargo-deny>

也可以根据自己的需要自定义配置: <https://embarkstudios.github.io/cargo-deny/>

如果在执行 `cargo deny check` 时提示如下错误：

```bash
2024-08-06 03:31:18 [ERROR] failed to open advisory database: "/Users/xxx/.cargo/advisory-dbs/github.com-2f857891b7f43c59" does not appear to be a git repository: Could not retrieve metadata of "/Users/xxx/.cargo/advisory-dbs/github.com-2f857891b7f43c59": No such file or directory (os error 2)
```

可以尝试执行 `cargo deny fetch` 解决

### 安装 typos

typos 是一个拼写检查工具

```bash
cargo install typos-cli
```

更多参考： <https://github.com/crate-ci/typos>

### 安装 git cliff

git cliff 是一个生成 changelog 的工具。

```bash
cargo install git-cliff
```

更多参考： <https://git-cliff.org/docs/>

## 使用

### cargo generate 创建项目

```bash
cargo generate fan-tastic-z/rust-project-template
```

## github action

github action 中这部分替换 是为了保证github action在当前template下可以正常运行，如果生成项目之后这部分可以根据需要进行删除

```YAML
- name: Get repository name
    run: |
        echo "repo_name=$(basename $GITHUB_REPOSITORY)" >> "$GITHUB_ENV"
- name: Replace package name in Cargo.toml
    run: |
    sed -i "s/zero2prod/$repo_name/g" Cargo.toml
```

### 替换 cliff 配置

替换 `cliff.toml` 配置文件中 下面配置中 replace 的值为自己的仓库地址

```toml
postprocessors = [
  { pattern = '\$REPO', replace = "https://github.com/fan-tastic-z/rust-project-template" }, # replace repository URL
]
```

### 安装 git hook scripts

在项目目录下执行

```bash
pre-commit install
```
