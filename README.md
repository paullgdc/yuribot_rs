# yuribot_rs

This is a telegram bot written in Rust that scraps top images from choosen subreddits, and then share them on command.

## Installation

### Dependencies

* `libssl-dev` and `libsqlite3-dev` to build the project
* `cargo` ( downloadable [here](https://rustup.rs/))

### Building

* clone the repository

```sh
git clone https://github.com/paullgdc/yuribot_rs.git
cd yuribot_rs/
```

* fill the config file `Yuribot.toml` with your bot's token and the command you want to use to request a picture with

* Or overwrite config by passing env variables prefixed with `YURIBOT_`(ex: `bot_token` -> `YURIBOT_BOT_TOKEN`)
ex :

```toml
database_path = "yuribot_rs.sqlite3"
bot_token = "626245263:AAHnIxc6IQkL26fzPiKCojW8IXeoedoEuFI"
reddit_user_agent = "Yuribot_rs/0.1"
```

* then build the bot (this can take a few minutes in `release` mode)

```sh
cargo build --release
```

* you can seed the database from pictures from /top using this command

```sh
YURIBOT_LOG=yuribot_rs=info cargo run --release -- --seed=200 # can be more than 200 if you need
```

* finally run the bot

```sh
YURIBOT_LOG=yuribot_rs=info cargo run --release
```

## Debugging

You can tune the log verbosity of the bot with the env variable `YURIBOT_LOG`
ex :

* only info,  warning and errors: `YURIBOT_LOG=yuribot_rs=info`
* debug informations : `YURIBOT_LOG=yuribot_rs=debug`
