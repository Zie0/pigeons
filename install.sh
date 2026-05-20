#!/bin/sh
# Copyright 2023 n0. All rights reserved. Dual MIT/Apache license.

set -e

bucket_url="https://vorc.s3.amazonaws.com"
bin_name="pigeons"

if [ "$OS" = "Windows_NT" ]; then
    echo "Error: this installer only works on linux & macOS." 1>&2
    exit 1
else
    case $(uname -sm) in
    "Darwin x86_64") target="darwin-x86_64" ;;
    "Darwin arm64") target="darwin-aarch64" ;;
    "Linux x86_64") target="linux-amd64" ;;
    "Linux arm64"|"Linux aarch64") target="linux-aarch64" ;;
    *) echo "Unsupported platform: $(uname -sm)" 1>&2; exit 1 ;;
    esac
fi

echo "Downloading $bin_name for $target"
curl -fL "${bucket_url}/${bin_name}-${target}-latest" -o "$bin_name"
chmod +x "$bin_name"

dest=/usr/local/bin
if [ ! -w "$dest" ]; then
    echo "Installing to $dest (requires sudo)"
    sudo mv "$bin_name" "$dest/$bin_name"
else
    mv "$bin_name" "$dest/$bin_name"
fi
echo "Installed to $dest/$bin_name"
