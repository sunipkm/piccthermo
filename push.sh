#!/usr/bin/env bash
TARGET="armv7-unknown-linux-gnueabihf"
REMOTE_TARGET=$(ssh pyro uname -m)
if [ $? -eq 0 ]; then
    if [ $REMOTE_TARGET = "aarch64" ]; then
        echo "Remote system is ARM64."
        TARGET="aarch64-unknown-linux-gnu"
    fi
fi
cross build --target $TARGET --release
ssh pyro -t 'mkdir -p ~/thermo-server'
scp target/$TARGET/release/thermo-server pyro:~/thermo-server
ssh -q pyro -t 'stat "thermo-server/thermo.env" &> /dev/null'
if [ $? -ne 0 ]; then
    scp thermo.env thermo.service pyro:~/thermo-server
fi
scp disable_ethernet_gadget.sh pyro:~/thermo-server
