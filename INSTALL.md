# Installation

## Step 1: Authentication Token

In order to access start.gg's API, you must first have an authentication token
linked to your account. Instructions for generating one can be found in the
[developer API docs](https://developer.start.gg/docs/authentication).

Once you have an auth token, it must be provided to StartRNR. In order, the
program checks for a token in:

1. A command-line flag `--auth`.
2. An environment variable `AUTH_TOKEN`,
3. A file `auth.txt` within the config directory:
  - Windows: `%APPDATA%\Roaming\startrnr/auth.txt`
  - MacOS: `~/Library/Application Support/startrnr/auth.txt`
  - Linux: `~/.config/startrnr/auth.txt`

The last method is recommended, as StartRNR can simply read from that file
whenever it needs to.

## Step 2: Dependencies

StartRNR requires these dependencies:

- [Rust](https://www.rust-lang.org/tools/install)
- [OpenSSL](https://github.com/openssl/openssl#build-and-install)
- [SQLite](https://www.sqlite.org/download.html)

Follow the instructions to download and install each.

## Step 3: Compiling

Once you have all the necessary dependencies, build and install StartRNR by
running the following command:

``` sh
cargo install --git https://github.com/kiana-S/StartRNR
```

