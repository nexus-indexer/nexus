# Nexus Blockchain Indexer
### This is a reimplementation of the Chronicle Indexer (https://github.com/developeruche/chronicle).

## Getting Started

1. Build the project
```sh
cargo build --release
```

2. Set up your environment
```sh
cp example.config.toml .config.toml
```

3. Run the binary
```sh
./target/release/nexus --config-path .config.toml
```
