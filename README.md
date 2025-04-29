# zm-rs

An implementation of infocom's Z-Machine in rust. Plays classic text adventure story files, such as Zork or
The Hitchiker's Guide to the Galaxy.

## What is a Z-Machine?

The Z-Machine was a solution to the problem of distributing text adventure games to home computers in the
late 80s. The text and code in an adventure game could approach 1MB in size, but computers of the day only
had 8-16KB of memory. Instead of shipping one large executable, Infocom compiled their games to a proprietary
bytecode that could be interpreted and executed by a much smaller virtual machine that could fit in memory.
The game content could then be stored on disk as paged in as needed.

This implementation conforms to the standards given [here](https://inform-fiction.org/zmachine/standards/).
It supports story files up to and including version 3. It has been verified with the CZECH Z-Machine checker
story file.

## Running games

The machine expects to find story files in the `games/` directory. To play a game, run
```
$ cargo run {game_file_name}
```
from the root directory, e.g. `$ cargo run zork`.

