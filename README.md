# StartRNR

**StartRNR** is an implementation of the cutting-edge player rating system known
as **Relative Network Rating (RNR)** for competitive gaming. It uses
[start.gg](https://www.start.gg/)'s tournament data to generate and maintain a
network of relative advantages between players.

Once the advantage network is generated, StartRNR can be used to predict the
probability of a player winning a match, generate provably optimal seedings for
tournaments, and create rankings of players automatically.

**All of these features work for any game, in any region, without restriction.**

> **Warning**<br>
> StartRNR is still under development; currently, it only supports generating the network.

## Installation

*For more information, see the [installation page](INSTALL.md).*

Build and install StartRNR using `cargo`:

``` sh
cargo install --git https://github.com/kiana-S/StartRNR
```

Alternatively, if you use Nix:

``` sh
nix profile install github:kiana-S/StartRNR
```

## Configuration

StartRNR stores its rating databases in its config directory, which is located at:

- Windows: `%APPDATA%\Roaming\startrnr`
- MacOS: `~/Library/Application Support/startrnr`
- Linux: `~/.config/startrnr`

There are few reasons to access this directory directly, but you can if you want to transfer your
datasets between computers.

## RNR system basics

*For more information on StartRNR's rating system, see the [details page](DETAILS.md).*
