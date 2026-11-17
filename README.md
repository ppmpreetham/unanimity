# Unanimity

Download one file over several internet connections at once.

If your laptop has Wi-Fi and Ethernet, or Wi-Fi and a tethered phone, unanimity uses all of them together.

If you only have one connection, your friends can lend you theirs. They join your hotspot, run one command, and their mobile data helps download your file.

## Install

1. Open the [latest release](https://github.com/ppmpreetham/unanimity/releases/latest) and download the file for your system:

   | System               | File name ends with                |
   | -------------------- | ---------------------------------- |
   | Windows              | `x86_64-pc-windows-msvc.zip`       |
   | macOS, Apple silicon | `aarch64-apple-darwin.tar.gz`      |
   | macOS, Intel         | `x86_64-apple-darwin.tar.gz`       |
   | Linux                | `x86_64-unknown-linux-gnu.tar.gz`  |
   | Linux, ARM           | `aarch64-unknown-linux-gnu.tar.gz` |

2. Unpack it.
3. Open a terminal in the unpacked folder and run `./unanimity --help` (on Windows: `.\unanimity.exe --help`).

To run `unanimity` from any folder, move the program to a folder that is on your `PATH`.

On macOS, the first run can be blocked because the program is not signed. Allow it in System Settings, Privacy & Security.

### Build it yourself

You need [Rust](https://rustup.rs). Then, in this folder:

```
cargo install --path crates/cli
```

## Download with your own connections

```
unanimity get https://example.com/big-file.zip
```

The file is saved in the current folder. To choose where it goes:

```
unanimity get https://example.com/big-file.zip -o ~/Downloads
```

To see which connections your device has:

```
unanimity networks
```

## Download with your friends' internet

1. **You:** turn on your hotspot.
2. **Each friend:** join that hotspot, keep mobile data (or another connection) on, and run:

   ```
   unanimity share
   ```

   It shows a 6-digit pairing code. To have everyone use the same code, pick one yourselves:

   ```
   unanimity share --code 482913
   ```

3. **You:** start the download with that code:

   ```
   unanimity get https://example.com/big-file.zip --code 482913
   ```

Your friends are found automatically. When the download ends, they press `Ctrl-C` to stop sharing.

## If a friend is not found

- The first time `unanimity share` runs, the firewall may ask for permission. Allow it.
- If the friend still does not appear, add them by hand. Use their IP address on your hotspot and port `7878`:

  ```
  unanimity get https://example.com/big-file.zip --code 482913 --peer 192.168.137.5:7878
  ```

## Good to know

- A friend only helps if their device has its own internet besides your hotspot, for example mobile data.
- Sharing uses your friends' data allowance.
- Give the pairing code only to people you trust. Anyone on the hotspot who has it can use the shared connection.
- The speed gain depends on the server. Some servers do not allow a file to be downloaded in parts; the file then downloads over one connection.
- A download that stops cannot be resumed yet. Start it again.

## License

GNU AFFERO GENERAL PUBLIC LICENSE
