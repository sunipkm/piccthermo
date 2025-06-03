#!/usr/bin/env bash
cross build --target armv7-unknown-linux-gnueabihf --release
ssh pyro -t 'mkdir -p ~/thermo-server'
scp target/armv7-unknown-linux-gnueabihf/release/thermo-server pyro:~/thermo-server