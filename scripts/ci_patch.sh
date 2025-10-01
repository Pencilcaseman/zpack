#!/usr/bin/bash

sudo apt install libssl-dev pkg-config
sudo apt update
sudo apt upgrade

cargo install cargo-cache
cargo update
cargo cache -a

