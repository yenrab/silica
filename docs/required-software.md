---
title: Required software
layout: default
permalink: /required-software/
---

# Required software

Everything that must be installed for the compilers to build and for the trials to run. The
checkout brings the compiler binaries (`binaries/`), the trial tree and the board support files;
the tools below come from outside the repository.

Versions are the ones the project is built and tested with (September 2026). Newer versions of the
same tools usually work; older ones are untested.

## At a glance

| Task | Needs |
| ---- | ----- |
| Build the self-hosted compiler (gen1, gen2) | [Host tools](#host-tools-macos-on-apple-silicon) |
| Run the trials on the Mac (`make integrate`) | [Host tools](#host-tools-macos-on-apple-silicon) |
| Build the ESP32-S3 compiler (`make TARGET=ESP32-S3_raw`) | Host tools |
| Build ESP32-S3 images, run programs or trials on a board | Host tools plus the [ESP32-S3 board tools](#esp32-s3-board-tools) and a [board](#the-board) |

## Host tools (macOS on Apple Silicon)

The build and the trials are validated on Apple Silicon Macs only (see the platform notice in
[Build and test the compiler]({{ '/build-and-test/' | relative_url }})).

| Software | Version used | Used for |
| -------- | ------------ | -------- |
| macOS on an arm64 (Apple Silicon) Mac | 26.6 | The emitted programs target macOS 26 (`-mmacosx-version-min=26.0`). |
| Xcode Command Line Tools | Apple clang 21.0 | `clang` assembles every `.sams` file and links (compiler and trial programs); the tools also provide **GNU Make 3.81** (drives every build and the trial tree), **perl 5** (compile timeouts, the trial watchdog, process groups and the board lock in the trial harness), **bash**, and the usual `find`, `awk`, `sed`, `diff`, `xargs`. |
| Homebrew LLVM (optional) | clang 21.1 | When `/opt/homebrew/opt/llvm/bin/clang` exists, the compiler build prefers it for linking. |

Install the command line tools with:

```bash
xcode-select --install
```

Nothing else is needed for the host build or the host trials: the seed compiler is in the
checkout (`binaries/seed-compiler`), and `binaries/silica-compiler` is the newest self-hosted build.

## ESP32-S3 board tools

Needed to turn the ESP32-S3 compiler's output into board images, and to load and run them. No
ESP-IDF component, header or library is linked into a Silica program; only the toolchain and the
flashing tool are used.

| Software | Version used | Used for |
| -------- | ------------ | -------- |
| Espressif Xtensa toolchain `xtensa-esp-elf` (`xtensa-esp32s3-elf-gcc`) | esp-16.1.0_20260609 (GCC 16.1) | Assembles the emitted Xtensa `.sams` and the board runtime, links with its `libgcc` (64-bit division, float64). |
| ESP-IDF Python environment with **esptool** and **pyserial** | Python 3.14, esptool 5.4.0, pyserial 3.5 | Builds the image (`esptool elf2image`), loads it into the board's RAM (or flashes it), and reads the console. |

Both come from the ESP-IDF 6.2 installer, which puts them under `~/.espressif`:

```bash
git clone -b v6.2 --recursive https://github.com/espressif/esp-idf.git
cd esp-idf
./install.sh esp32s3
```

The board tools look for them in those default places; if yours are elsewhere, point the tools at
them:

| Variable | Default | Read by |
| -------- | ------- | ------- |
| `XTENSA_BIN` | `~/.espressif/tools/xtensa-esp-elf/esp-16.1.0_20260609/xtensa-esp-elf/bin` | `board/tools/build_image.sh` |
| `ESPTOOL` | `~/.espressif/python_env/idf6.2_py3.14_env/bin/esptool` | `board/tools/build_image.sh` |
| `BOARD_PYTHON` | `~/.espressif/python_env/idf6.2_py3.14_env/bin/python` | the board trial target (`trials/targets/`) |

(`board/` is `compiler/silica-compiler/src_selfhost/emitter/ESP32-S3_raw/board/`.)

### The ESP32-S3 compiler

Board programs and board trials are compiled by `binaries/silica-compiler-ESP32-S3_raw`: a
self-hosted compiler that runs on the Mac and emits ESP32-S3 (Xtensa) assembly. Build and publish it
with the host tools:

```bash
cd compiler/silica-compiler/src_selfhost
make TARGET=ESP32-S3_raw
```

It is installed as `binaries/silica-NNNNNN-ESP32_S3_raw-macos-applesilicon`, with its own numbering,
and `binaries/silica-compiler-ESP32-S3_raw` points at the newest one. It never replaces
`binaries/silica-compiler`.

### The board

- An ESP32-S3 board whose UART0 (GPIO43/44) is wired to a USB serial bridge. The port was brought
  up and is tested on the board described in
  [PORT_TEST_BOARD_byu_idaho_v4.0_feb26.md](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/src_selfhost/emitter/ESP32-S3_raw/board/PORT_TEST_BOARD_byu_idaho_v4.0_feb26.md)
  (BYU-I eBadge v4.0, ESP32-S3-MINI-1-N4R2, CP2102N bridge).
- A USB **data** cable (charge-only cables enumerate nothing). macOS's built-in driver serves the
  CP2102N; the board appears as `/dev/cu.usbserial-*`. If the board has a power switch, it must be on.
- The trial runner uses the one USB serial port it finds. With several, set `BOARD_PORT=/dev/cu....`
  (an interactive run asks). `BOARD_MAC=<mac>` makes a run refuse any board but that one.

## Checking an installation

```bash
make --version | head -1          # GNU Make 3.81
clang --version | head -1         # Apple clang 21 (or newer)
perl -v | sed -n 2p               # perl 5

# ESP32-S3 board tools
~/.espressif/tools/xtensa-esp-elf/esp-16.1.0_20260609/xtensa-esp-elf/bin/xtensa-esp32s3-elf-gcc --version | head -1
~/.espressif/python_env/idf6.2_py3.14_env/bin/python -c "import esptool, serial; print(esptool.__version__, serial.VERSION)"
~/.espressif/python_env/idf6.2_py3.14_env/bin/python \
    compiler/silica-compiler/src_selfhost/emitter/ESP32-S3_raw/board/tools/run_on_board.py --probe
```

The last command connects to the board and prints its port, chip and MAC address; it fails with a
plain message when no board is connected.

## See also

- [Build and test the compiler]({{ '/build-and-test/' | relative_url }}): building the compilers and running the trials.
- [trials/README.md](https://github.com/yenrab/silica/blob/main/trials/README.md) and [trials/targets/README.md](https://github.com/yenrab/silica/blob/main/trials/targets/README.md): the trial tree, and running it on a board.
- [board/README.md](https://github.com/yenrab/silica/blob/main/compiler/silica-compiler/src_selfhost/emitter/ESP32-S3_raw/board/README.md): the ESP32-S3 board support files.
