# `pandoras_pot` - Unleash Unfathomable Curses on Unsuspecting Bots... In Rust!

Inspired by [HellPot](https://github.com/yunginnanet/HellPot), `pandoras_pot`
aims to bring even more misery on unruly web crawlers that doesn't respect your
`robots.txt`.

The goal with `pandoras_pot` is to have maximum data output, while not using up
all the resources of your webserver that probably could be doing better things
with its time.

To ensure that bots don't detect `pandoras_pot`, it generates random data that kind
of looks like a website (to a bot), really *really* fast. Like crazy fast.

## Setting it up

The easiest way to set up `pandoras_pot` is using docker. You can optionally
pass an argument to a config file using the docker `--build-arg CONFIG=<path to
your config>` flag (but it should be available in the build context).

To build an image and deploy it, here naming and tagging it with `pandoras_pot`
and making it available on port `localhost:6669`, you can run the following:

```sh
docker build -t pandoras_pot . # You can add --build-arg CONFIG=<...> here
docker run --name=pandoras_pot --restart=always -p 6669:8080 -d pandoras_pot
```
