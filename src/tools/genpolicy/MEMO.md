# MEMO

```bash
# cargo build && RUST_LOG=info RUST_BACKTRACE=1 target/debug/genpolicy -y ../../agent/samples/policy/yaml/pod/pod-one-container.yaml
cargo build && RUST_LOG=info RUST_BACKTRACE=1 target/debug/genpolicy -y pod-same-containers.yaml
cat pod-same-containers.yaml | grep "io.katacontainers.config.agent.policy" | awk -F': ' '{print $2}' | base64 --decode > output.rego
```
