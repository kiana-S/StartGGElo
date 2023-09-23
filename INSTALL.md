# Installation

## Step 1: Authentication Token

In order to access start.gg's API, you must first have an authentication token
linked to your account. Instructions for generating one can be found in the
[developer API docs](https://developer.start.gg/docs/authentication).

Once you have an auth token, it must be provided to StartGGElo. In order, the
program checks for a token in:

- A command-line flag `--auth`.
- An environment variable `AUTH_TOKEN`,
- A file `auth.txt` within the config directory (see the [[README]] for a list
  of directories in each OS).

## Step 2: Dependencies

StartGGElo is written in Rust, so install the [Rust
compiler](https://www.rust-lang.org/tools/install).

In addition, StartGGElo needs these run-time dependencies:

- [OpenSSL](https://www.openssl.org/)
- [SQLite](https://www.sqlite.org/)

## Step 3: Compiling

Once you have all the necessary dependencies, build and install StartGGElo by
running the following command:

``` sh
cargo install --git https://github.com/kiana-S/StartGGElo
```

