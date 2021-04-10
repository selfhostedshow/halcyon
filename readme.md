# Halcyon - Home Assistant Linux Companion

Halcyon is a Home Assistant app for Linux built in rust.

## Building

### Prerequisites

* Rust
* libssl-dev (or equivalent package for your distro)

To build Halcyon, run `cargo build --release`.

## Running

Only the setup command is working at the moment
`cargo run setup`

Or if you built the release executable (found in target/release)
`halcyon setup`
### Yaml Config

The setup process will create a config file for you if you don't have one.

Otherwise, the minimum config looks like
```yaml
---
ha:
  host: "ip:port"
```


Once you run the set up command, a full config file will look like
```yaml
---
ha:
  host: "ip:host"
  long-lived-token: ...
  device-id: ...
  webhook-id: ...
```
The setup process should create the tokens and ids for you as you move through the process