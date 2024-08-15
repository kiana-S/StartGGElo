# StartRNR

**StartRNR** is an implementation of the cutting-edge player rating system known
as **Relative Network Rating (RNR)** for competitive gaming. It uses
[start.gg](https://www.start.gg/)'s tournament data to generate and maintain a
network of relative advantages between players.

Once the advantage network is generated, StartRNR can be used to predict the
probability of a player winning a match, generate provably optimal seedings for
tournaments, inspect the match history of two players, and create competitive
rankings automatically.

**All of these features work for any game, in any region, without restriction.**

> [!WARNING]
> StartRNR is unstable and under active development. The design and user
> interface of this program is experimental and may be subject to change.
> 
> Currently, the power ranking and seeding features have not been implemented.

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

You will need to provide a start.gg API token to access tournament data. Details can be found in [INSTALL.md](INSTALL.md).

## Usage

Once StartRNR is installed, run:

``` sh
startrnr sync
```

The program will walk you through creating a dataset, then run its rating
algorithm. **This may take up to a few hours to finish running!**

Once the rating data has been generated, these commands can be used to access it:

``` sh
# Access a player's data
startrnr player info <player>

# Analyze matchup of two players
startrnr player matchup <player1> <player2>
```

A player can be specified by their tag or by their
[discriminator](https://help.start.gg/en/articles/4855957-discriminators-on-start-gg).

## Details - The RNR System

*For more information on RNR, see the [details page](DETAILS.md).*
