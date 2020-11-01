# Taribot

Rust implementaion of intial [python](https://github.com/Tarinu/taribot) version of taribot. It's still missing few of the functionalities of the initial bot.

## How to run

Copy `.env-dist` as `.env` and fill in the required variables.

### Docker

The easiest way is to just run `docker-compose up`. This will then pull the image from dockerhub and run it. I try to keep it up to date with master, until I figure out how github actions work or [this issue](https://github.com/Tarinu/taribot-rs/issues/4) gets fixed.
Or you can include the `docker-compose.dev.yml` to compile it yourself. You should probably use [buildkit](https://docs.docker.com/develop/develop-images/build_enhancements/) to avoid compiling it for all architectures.

### CLI

The other option is to install [rustup](https://rustup.rs/) and compile it yourself using `cargo build` or `cargo run`. You can also include `--release` to those commands to build a release build.
