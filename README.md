# BadNews: Matrix Bot, Bringer of Bad News

## What is this?

`BadNews` is a [Matrix](https://matrix.org) bot, watching your systemd/journald
logs, and reporting bad news to you.

## Why?

A mix of wanting to toy around with the [matrix_sdk
crate](https://github.com/matrix-org/matrix-rust-sdk) and getting some simple
alerts from my hosted services.

## Setup

Write up a configuration file at `config.yaml`:

```yaml
homeserver: "https://matrix.example.net"
username: "bad-news"
password: "matrix password for user bad-news"
state_dir: "state/"
room_id: "!DeaDbEef:example.net"
units:
  - name: nginx.service
    filter: "\\[warn\\] .*"
```

Then run the bot:

``` sh
cargo run -- --config config.yaml
```

## Contributing

I accept contributions via [GitHub](https://github.com/alarsyo/bad-news) Pull
Requests and [GitLab](https://gitlab.com/alarsyo/bad-news) Merge Requests.

### Sending patches by mail
You can also send patches to
[~alarsyo/patches@lists.sr.ht](https://lists.sr.ht/~alarsyo/patches) with the
prefix `PATCH bad-news`.

You can use the following commands to set up `git` appropriately;

``` sh
git config sendemail.to '~alarsyo/patches@lists.sr.ht'
git config format.subjectPrefix 'PATCH bad-news'
```
