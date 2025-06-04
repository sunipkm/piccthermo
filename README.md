# Temperature and Humidity Sensor Controller on PICTURE-D


## Cross compilation

If you're not working directly on a Raspberry Pi, you'll have to cross-compile your code for the appropriate ARM architecture. Check out [this guide](https://github.com/japaric/rust-cross) for more information, or try the [cross](https://github.com/japaric/cross) project for "zero setup" cross compilation.

### Cargo

For manual cross-compilation without the use of `cross`, you will need to install the appropriate target. Most Raspberry Pi models either need the `armv7-unknown-linux-gnueabihf` target for 32-bit Linux distributions, or `aarch64-unknown-linux-gnu` for 64-bit. For some models, like the Raspberry Pi Zero, a different target triple is required.

Install the relevant target using `rustup`.

```bash
rustup target install armv7-unknown-linux-gnueabihf
```

In the root directory of your project, create a `.cargo` subdirectory, and save the following snippet to `.cargo/config.toml`.

```toml
[build]
target = "armv7-unknown-linux-gnueabihf"
```

### Visual Studio Code

The rust-analyzer extension for Visual Studio Code needs to be made aware of the target platform by setting the `rust-analyzer.cargo.target` configuration option. In the root directory of your project, create a `.vscode` subdirectory, and then save the following snippet to `.vscode/settings.json`.

```json
{
    "rust-analyzer.cargo.target": "armv7-unknown-linux-gnueabihf"
}
```

### User setup and running on boot
#### User setup
Add yourself to the `i2c` and `dialout` groups:
```sh
sudo usermod -aG i2c $USER
sudo usermod -aG dialout $USER
```

#### Running on boot

```sh
sudo install thermo.service /usr/lib/systemd/system
```
Then, to test,
```sh
sudo systemctl start thermo
```

To enable,
```sh
sudo systemctl enable thermo
```