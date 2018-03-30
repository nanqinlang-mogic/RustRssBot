# RustRssBot
[![Build Status](https://github.com/nanqinlang/SVG/blob/master/build%20passing.svg)](https://github.com/nanqinlang-mogic/RustRssBot)
[![author](https://github.com/nanqinlang/SVG/blob/master/author-nanqinlang-lightgrey.svg)](https://github.com/nanqinlang-mogic/RustRssBot)
[![license](https://github.com/nanqinlang/SVG/blob/master/license-GPLv3-orange.svg)](https://github.com/nanqinlang-mogic/RustRssBot)

A telegram rssbot based on `RustRssBot` via rust lang.

Origin Project: https://github.com/iovxw/rssbot


## Usage
### server
to make a rss bot, you should run these to make it server.
```bash
wget https://github.com/nanqinlang-mogic/RustRssBot/releases/download/1.0/rssbot
chmod +x rssbot
nohup ./rssbot datafile ${TELEGRAM-BOT-TOKEN} &
```

### client
the following is each command and function of the bot:

| command      | function              | parameter                                     |
| :---         | :---                  | :---                                          |
| list         | show subscribe list   | /list (${Channel-Username}) (display-url)     |
| subscribe    | subscribe one         | /subscribe (${Channel-Username}) ${rss-url}   |
| unsubscribe  | unsubscribe one       | /unsubscribe (${Channel-Username}) ${rss-url} |


## Compilation
if you want to compile it by yourself, you should download `source` and prepare a `Rust & Cargo` environment.

### download source
```bash
git clone https://github.com/nanqinlang-mogic/RustRssBot.git
```

### Rust & Cargo environment
advise you to use [rustup](https://www.rustup.rs/), the installation is in [there](https://github.com/rust-lang-nursery/rustup.rs/#other-installation-methods).
```bash
curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly
```

### start compilation
install dependences:
```bash
apt-get install -y openssl libssl-dev pkg-config
```
then start compile it:
```bash
cargo build --release
```
then, the implement file is in `~/target/release/rssbot`