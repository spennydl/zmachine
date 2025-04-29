## zm-rs

An implementation of infocom's Zmachine in rust.

This implementation conforms to the standards given [here](https://inform-fiction.org/zmachine/standards/)
and supports story files up to and including version 3. It has been verified with the CZECH Z-Machine checker
story file.

The machine expects to find story files in the `games/` directory. To play a game, run
```
$ cargo run {game_file_name}
```
from the root directory, e.g. `$ cargo run zork`.

