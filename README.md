# Vkontroller

Run a server that serves a controller in the browser

## Installation

### For Linux

Install it normally by getting the latest package for linux
or building from source:

```bash
git clone https://github.com/flamfrosticboio/vkcontroller
cid vkcontroller
cargo run
```

### For Windows

Install [vigem bus driver](https://vigembus.com/download/)

Then get the latest package for windows or building it from source:

```bash
git clone https://github.com/flamfrosticboio/vkcontroller
cd vkcontroller
cargo run
```

## Usage

> [!NOTE]
> The server looks for `./dist/index.html` to load the website for loading
> the controller frontend. It's possible to change the website into a custom look.

To control where to serve the server, pass an env `HOST` with format `<ip>:<port>`.

Setting it to `0.0.0.0:<port>` will serve on all network connections

Example (Windows):

```bash
set HOST=0.0.0.0:8000
./vkontroller.exe
```

To control logging, use `RUST_LOG` to control logging. Can be set to:

- `error`
- `warn` (recommended)
- `info`
- `debug`
- `trace` (not recommended)
