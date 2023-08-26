# StartGGElo

StartGGElo is a tool for using [start.gg](https://www.start.gg/)'s video game tournament data to
generate and maintain [Elo ratings](https://wikipedia.org/wiki/Elo_rating_system) for each competitive player.

Once Elo ratings are generated, StartGGElo can be used to generate ranking lists, predict the
probability of a player winning a match, generate seedings for future tournaments, and more.

> **Note**<br>
> StartGGElo is still under development, and it currently only supports generating player ratings.

## Installation

TODO

## Configuration

StartGGElo stores its rating database in its config directory, which is located at:

- Windows: `%APPDATA%\Roaming\ggelo`
- Mac/Linux: `~/.config/ggelo` or `~/.ggelo`

This directory also contains StartGGElo's config file, which defines how it calculates its ratings.

## Elo system basics

*For more information on StartGGElo's rating system, see the [details page](DETAILS.md).*

As the name implies, StartGGElo uses the Elo system for its ratings. In the Elo system, all newcomers to the
game are assigned an **initial rating**, and this rating is adjusted whenever a player loses or wins matches.
The initial rating for StartGGElo is 1500, but this is configurable.

Whenever a player enters a tournament, StartGGElo will use start.gg's API to determine how many sets
that player won within that tournament. This number is the player's **score** for that tournament. If the score
the player earned is larger than their Elo rating would predict, then their rating is increased.
