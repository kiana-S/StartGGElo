# StartRNR

StartGGElo is a tool for using [start.gg](https://www.start.gg/)'s video game tournament data to
generate and maintain [Elo ratings](https://wikipedia.org/wiki/Elo_rating_system) for each competitive player.

Once Elo ratings are generated, StartGGElo can be used to generate ranking lists, predict the
probability of a player winning a match, generate seedings for future tournaments, and more.

> **Warning**<br>
> StartRNR is still under development; currently, it only supports generating player ratings.

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
