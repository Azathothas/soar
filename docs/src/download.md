# Download Files

Soar provides the ability to download arbitrary files from the internet.

<div class="video-wrapper">
    <video src="/video/download.mp4" controls></video>
</div>

## Basic Usage

To download a file, use the `download` command or the `dl:` alias:

```sh
soar download <url>
```

Example: Download Soar nightly
```sh
soar download https://github.com/QaidVoid/soar/releases/download/nightly/soar-nightly-x86_64-linux
```

To set the output filename, use the `-o` flag:

```sh
soar download https://github.com/QaidVoid/soar/releases/download/nightly/soar-nightly-x86_64-linux -o soar-nightly
```

Example: Download multiple files and save them to the `downloads` directory
```sh
soar download https://github.com/QaidVoid/soar/releases/download/nightly/soar-nightly-x86_64-linux https://github.com/QaidVoid/soar/releases/download/nightly/soar-nightly-aarch64-linux -o downloads/
```

<div class="warning">
    It currently doesn't support multiple custom output filenames. So if you specify an output filename, it will be used for all downloads, meaning only the last download will be saved if you specify multiple URLs.
</div>
