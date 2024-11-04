# Installation

There are several ways to install Soar on your Linux system. Choose the method that best suits your needs.

<div class="video-wrapper">
    <video src="/video/installation.mp4" controls></video>
</div>

## Quick Installation

The fastest way to install Soar is using our installation script:

```sh
curl -fsSL https://soar.qaidvoid.dev/install.sh | sh
```

Or if you prefer using wget:

```sh
wget -qO- https://soar.qaidvoid.dev/install.sh | sh
```


## Manual Installation

### From Binary Releases

1. Visit the [releases page](https://github.com/QaidVoid/soar/releases)
2. Download the appropriate binary for your architecture:
   - `soar-x86_64-linux` for 64-bit Intel/AMD systems
   - `soar-aarch64-linux` for 64-bit ARM systems

3. Make the binary executable:
```sh
chmod +x soar
```

4. Move the binary to your desired location, for example `/usr/local/bin`:
```sh
sudo mv soar /usr/local/bin/soar
```

5. Verify the installation by running `soar --version`:
```sh
soar --version
```
This should output the version number of the installed Soar binary.

## From Source

To install Soar from source, you need to have Rust and Cargo installed. Follow these steps:

1. Clone the Soar repository:
```sh
git clone https://github.com/QaidVoid/soar.git
```

2. Navigate to the Soar directory:
```sh
cd soar
```

3. Build and install the project:
```sh
cargo install --path .
```

4. Verify the installation by running `soar --version`:
```sh
soar --version
```
This should output the version number of the installed Soar binary.

## Next Steps

After installing Soar, you might want to:
1. [Configure Soar](./configuration.md) for your specific needs
2. Learn about [package management](./package-management.md)
3. Try [installing your first package](./install.md)