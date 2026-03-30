# pigeons

[![Crates.io](https://img.shields.io/crates/v/pigeons.svg)](https://crates.io/crates/pigeons)
[![Documentation](https://docs.rs/pigeons/badge.svg)](https://docs.rs/pigeons)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![AUR](https://img.shields.io/aur/version/pigeons-git)](https://aur.archlinux.org/packages/pigeons-git)

**SSH to any machine without ip, behind a NAT/firewall without port forwarding or VPN setup.**

```bash
# on server
> pigeons server --persist

    Connect to this this machine:

    pigeons my-user@bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330


# on client
> pigeons user@bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330
# or with certificate
> pigeons -i ~/.ssh/id_rsa_my_cert my-user@bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330
```

**That's all it takes.** (requires ssh/(an ssh server) to be installed)

---

## Installation

```bash
cargo install pigeons
```

Download and setup the binary automatically for your operating system from [GitHub Releases](https://github.com/rustonbsd/pigeons/releases):

Linux
```bash
# Linux
wget https://github.com/rustonbsd/pigeons/releases/download/0.2.9/pigeons.linux
chmod +x pigeons.linux
sudo mv pigeons.linux /usr/local/bin/pigeons
```

macOS
```bash
# macOS arm
curl -LJO https://github.com/rustonbsd/pigeons/releases/download/0.2.9/pigeons.macos
chmod +x pigeons.macos
sudo mv pigeons.macos /usr/local/bin/pigeons
```

Windows
```bash
# Windows x86 64bit
curl -L -o pigeons.exe https://github.com/rustonbsd/pigeons/releases/download/0.2.9/pigeons.exe
mkdir %LOCALAPPDATA%\pigeons
move pigeons.exe %LOCALAPPDATA%\pigeons\
setx PATH "%PATH%;%LOCALAPPDATA%\pigeons"
```

Verify that the installation was successful
```bash
# restart your terminal first
> pigeons --help
```

---

## Client Connection

```bash
# Install for your distro (see above)
# Connect from anywhere
> pigeons my-user@38b7dc10df96005255c3beaeaeef6cfebd88344aa8c85e1dbfc1ad5e50f372ac
```

Works through any firewall, NAT, or private network. No configuration needed.

![Connecting to remote server](/media/t-rec_connect.gif)
<br>

---

## Server Setup

```bash
# Install for your distro (see above)
# (use with tmux or install as service on linux)

> pigeons server --persist

    Connect to this this machine:

    pigeons my-user@bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330

    (using persistent keys in /home/my-user/.ssh/irohssh_ed25519)

    Server listening for iroh connections...
    client -> pigeons -> direct connect -> pigeons -> local ssh :22
    Waiting for incoming connections...
    Press Ctrl+C to exit

```

or use ephemeral keys

```bash
# Install for your distro (see above)
# (use with tmux or install as service on linux)

> pigeons server

    Connect to this this machine:

    pigeons my-user@bb8e1a5661a6dfa9ae2dd978922f30f524f6fd8c99b3de021c53f292aae74330

    warning: (using ephemeral keys, run 'pigeons server --persist' to create persistent keys)

    client -> pigeons -> direct connect -> pigeons -> local ssh :22
    Waiting for incoming connections...
    Press Ctrl+C to exit
    Server listening for iroh connections...

```

Display its Endpoint ID and share it to allow connection

![Starting server/Installing as service](/media/t-rec_server_service.gif)
<br>

## Connection information
```bash
// note: works only with persistent keys
> pigeons info

    Your pigeons endpoint id: 38b7dc10df96005255c3beaeaeef6cfebd88344aa8c85e1dbfc1ad5e50f372ac
    pigeons version 0.2.9
    https://github.com/rustonbsd/pigeons

    Your server pigeons endpoint id:
      pigeons my-user@38b7dc10df96005255c3beaeaeef6cfebd88344aa8c85e1dbfc1ad5e50f372ac

    Your service pigeons endpoint id:
      pigeons my-user@4fjeeiui4jdm96005255c3begj389xk3aeaeef6cfebd88344aa8c85e1dbfc1ad
```

---

## How It Works

```
┌─────────────┐          ┌─────────────────┐          ┌─────────────┐
│     SSH     │─────────▶│  QUIC Tunnel    │─────────▶│  pigeons   │
│   Client    │          │  (P2P Network)  │          │   server    │
└─────────────┘          └─────────────────┘          └─────────────┘
      │                           ▲                            │
      │                           │                            │
      ▼                           │                            ▼
┌─────────────┐          ┌─────────────┐          ┌──────────────────┐
│ ProxyCommand│          │  pigeons   │          │   SSH Server     │
│ pigeons    │──────────│    proxy    │          │ localhost:22     │
│ proxy %h    │          │             │          └──────────────────┘
└─────────────┘          └─────────────┘
```

1. **SSH Client**: Invokes `pigeons proxy` via SSH's ProxyCommand
2. **Proxy**: Establishes QUIC connection through Iroh's P2P network (automatic NAT traversal)
3. **Server**: Accepts connection and proxies to local SSH daemon (port 22)
4. **Authentication**: Standard SSH security end-to-end over encrypted QUIC tunnel

## Use Cases

- **VNC/RDP over SSH**: Securely access graphical desktops remotely
- **VisualStudio SSH Extension**: Develop on remote machines seamlessly
- **Remote servers**: Access cloud instances without exposing SSH ports
- **Home networks**: Connect to devices behind router/firewall
- **Corporate networks**: Bypass restrictive network policies
- **IoT devices**: SSH to embedded systems on private networks
- **Development**: Access staging servers and build machines

## Commands

```bash
# Get your Endpoint ID and info
> pigeons info

# Server modes
> pigeons server --persist          # Interactive mode, e.g. use tmux (default SSH port 22)
> pigeons server --ssh-port 2222    # Custom SSH port (using ephemeral keys)

# Service mode
> pigeons service install                   # Background daemon (linux and windows only, default port 22)
> pigeons service install --ssh-port 2222   # Background daemon with custom SSH port
> pigeons service uninstall                 # Uninstall service

# Client connection
> pigeons user@<ENDPOINT_ID>                    # Connect to remote server
> pigeons connect user@<ENDPOINT_ID>            # Explicit connect command, works with all standard ssh params and flags
```

## Security Model

- **Endpoint ID access**: Anyone with the Endpoint ID can reach your SSH port
- **SSH authentication**: SSH key file, certificate and password auth are supported
- **Persistent keys**: Uses dedicated `.ssh/iroh_ssh_ed25519` keypair
- **QUIC encryption**: Transport layer encryption between endpoints

## Status

- [x] Password authentication
- [x] Persistent SSH keys
- [x] Linux service mode
- [x] Add howto gifs
- [x] Add -p flag for persistence
- [x] Windows service mode
- [x] (almost) all ssh commands supported
- [ ] MacOS service mode

## Custom Relay Setup

see: [CUSTOM_RELAY.md](CUSTOM_RELAY.md)

## License

Licensed under either of Apache License 2.0 or MIT license at your option.
