#!/usr/bin/env bash

while :; do
    case $1 in
        --force) FORCE=1 ;;
        --help) echo "Usage: $0 [--force]"; exit 0 ;;
        *) break ;;
    esac
    shift
done
if [ -n "$FORCE" ]; then
    echo "Force mode enabled. All files will be pushed."
else
    echo "Normal mode. Only new or modified files will be pushed."
fi

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
ssh -q pyro -t 'stat "thermo-server/thermo-server" &> /dev/null'
if [ $? -ne 0 ] || [ -n "$FORCE" ]; then
    scp target/$TARGET/release/thermo-server pyro:~/thermo-server
else
    echo "Not updating server binary."
fi
scp target/$TARGET/release/thermo-tester pyro:~/thermo-server
scp target/$TARGET/release/humi-tester pyro:~/thermo-server
scp target/$TARGET/release/thermo-ident pyro:~/thermo-server
scp target/$TARGET/release/thermo-cputemp pyro:~/thermo-server
ssh -q pyro -t 'stat "thermo-server/thermo.env" &> /dev/null'
if [ $? -ne 0 ]; then
    scp thermo.env thermo.service pyro:~/thermo-server
fi
scp disable_ethernet_gadget.sh pyro:~/thermo-server
