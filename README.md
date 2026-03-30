# pigeons

[![Crates.io](https://img.shields.io/crates/v/pigeons.svg)](https://crates.io/crates/pigeons)
[![Documentation](https://docs.rs/pigeons/badge.svg)](https://docs.rs/pigeons)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![AUR](https://img.shields.io/aur/version/pigeons-git)](https://aur.archlinux.org/packages/pigeons-git)

**SSH to any machine without an IP address, behind a NAT/firewall without port forwarding or VPN setup.**

```bash
# on the server
> pigeons roost
roost is running! id: bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330

# on the client — add a route, then ssh as normal
> pigeons add --id bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330 --name my-server
> ssh user@my-server
```

**That's all it takes.** (requires ssh/sshd to be installed)

---

## Installation

```bash
cargo install pigeons
```

Download the binary for your operating system from [GitHub Releases](https://github.com/rustonbsd/pigeons/releases):

Linux
```bash
wget https://github.com/rustonbsd/pigeons/releases/download/0.2.9/pigeons.linux
chmod +x pigeons.linux
sudo mv pigeons.linux /usr/local/bin/pigeons
```

macOS
```bash
curl -LJO https://github.com/rustonbsd/pigeons/releases/download/0.2.9/pigeons.macos
chmod +x pigeons.macos
sudo mv pigeons.macos /usr/local/bin/pigeons
```

Windows
```bash
curl -L -o pigeons.exe https://github.com/rustonbsd/pigeons/releases/download/0.2.9/pigeons.exe
mkdir %LOCALAPPDATA%\pigeons
move pigeons.exe %LOCALAPPDATA%\pigeons\
setx PATH "%PATH%;%LOCALAPPDATA%\pigeons"
```

Verify that the installation was successful:
```bash
# restart your terminal first
> pigeons --help
```

---

## Quick Start

### Server (roost)

Start a roost to accept incoming connections. Keys are persisted by default so the endpoint ID stays the same across restarts:

```bash
> pigeons roost
roost is running! id: bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330
```

Use `--ephemeral` for a throwaway identity, or `--ssh-port` if sshd is on a non-standard port:

```bash
> pigeons roost --ephemeral --ssh-port 2222
```

### Client (fly)

The easiest way to connect is to add a pigeon route, which creates an SSH config entry:

```bash
> pigeons add --id bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330 --name my-server
Pigeon route 'my-server' added to ~/.ssh/config

  Fly with: ssh <user>@my-server
```

Then connect with standard ssh:

```bash
> ssh user@my-server
```

For a quick one-off connection without modifying ssh config:

```bash
> pigeons fly bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330
```

Works through any firewall, NAT, or private network. No configuration needed.

---

## Service Mode

Install pigeons as a system service for always-on access:

```bash
> pigeons service install                   # default SSH port 22
> pigeons service install --ssh-port 2222   # custom SSH port
> pigeons service status                    # check if the service is running
> pigeons service log                       # view service logs
> pigeons service uninstall                 # remove the service
```

Supported on Linux (systemd), macOS (launchd), and Windows (SCM).

---

## Route Management

```bash
# Add a route
> pigeons add --id <ENDPOINT_ID> --name my-server

# List configured routes
> pigeons list

# Remove a route
> pigeons remove my-server
```

---

## How It Works

```
┌─────────────┐          ┌─────────────────┐          ┌─────────────┐
│     SSH     │─────────▶│  QUIC Tunnel    │─────────▶│   pigeons   │
│   Client    │          │  (P2P Network)  │          │    roost     │
└─────────────┘          └─────────────────┘          └─────────────┘
      │                           ▲                            │
      │                           │                            │
      ▼                           │                            ▼
┌─────────────┐          ┌─────────────────┐          ┌──────────────────┐
│ ProxyCommand│          │   pigeons fly   │          │   SSH Server     │
│ pigeons fly │─────────▶│    --stdio      │          │  localhost:22    │
│   --stdio   │          │                 │          └──────────────────┘
└─────────────┘          └─────────────────┘
```

1. **SSH Client**: Invokes `pigeons fly --stdio` via SSH's ProxyCommand
2. **Fly**: Establishes QUIC connection through Iroh's P2P network (automatic NAT traversal)
3. **Roost**: Accepts connection and proxies to local SSH daemon (port 22)
4. **Authentication**: Standard SSH authentication end-to-end over encrypted QUIC tunnel

## Use Cases

- **Remote servers**: Access cloud instances without exposing SSH ports
- **Home networks**: Connect to devices behind router/firewall
- **Corporate networks**: Bypass restrictive network policies
- **IoT devices**: SSH to embedded systems on private networks
- **Development**: Access staging servers and build machines
- **VS Code Remote SSH**: Develop on remote machines seamlessly
- **VNC/RDP over SSH**: Securely access graphical desktops remotely

## Commands

```bash
# Server
> pigeons roost                             # start a roost (persistent keys by default)
> pigeons roost --ephemeral                 # throwaway identity
> pigeons roost --ssh-port 2222             # custom SSH port

# Client
> pigeons fly <ENDPOINT_ID>                 # quick connect (binds local port)
> pigeons fly --stdio <ENDPOINT_ID>         # ProxyCommand mode (used by ssh config)
> pigeons add --id <ID> --name <NAME>       # add SSH config entry
> pigeons list                              # list pigeon routes
> pigeons remove <NAME>                     # remove SSH config entry

# Service
> pigeons service install                   # install as system service
> pigeons service install --ssh-port 2222   # with custom SSH port
> pigeons service status                    # check service status
> pigeons service log                       # view service logs
> pigeons service uninstall                 # remove service
```

## Security Model

- **Endpoint ID access**: Anyone with the Endpoint ID can reach your SSH port
- **SSH authentication**: Standard SSH auth (keys, certificates, passwords) applies
- **Persistent keys**: Uses dedicated `.ssh/pigeons_ed25519` keypair
- **QUIC encryption**: Transport layer encryption between endpoints

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
