# zero2prod

阅读 《从零构建Rust生产级服务》实践，这里使用的是rust的 `axum` web框架进行实践

该项目是通过 `https://github.com/fan-tastic-z/rust-project-template` 模版生成

建议在环境中添加如下配置方便开发：

```bash
alias cwr="cargo watch -q -c -x 'run -q'"

# Cargo watch install
function cwi() {
    cargo watch -x "install --patch ."
}

# usage `cwe xp_file_name`
function cwe() {
    cargo watch -q -c -x "run -q --example '$1'"
}

# - 'cwt test_my_fn' for a test function name match
# - 'cwt test_file_name test_my_fn'
function cwt() {
    if [[ $# -eq 1 ]];then
        cargo watch -q -c -x "test '$1' -- --nocapture"
    elif [[ $# -eq 2 ]];then
        cargo watch -q -c -x "test --test '$1' '$2' -- --nocapture"
    else
        cargo watch -q -c -x "test -- --nocapture"
    fi
}
```
