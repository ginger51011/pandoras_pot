<div align="center">
    <h1>🔥pandoras_pot🍯</h1>
    <i>Unleash Unfathomable Curses on Unsuspecting Bots... In Rust!</i>
<br />
<br />

[![GitHub License](https://img.shields.io/github/license/ginger51011/pandoras_pot)](https://github.com/ginger51011/pandoras_pot/blob/main/LICENSE)
[![Crates.io (pandoras_pot)](https://img.shields.io/crates/v/pandoras_pot)](https://crates.io/crates/pandoras_pot)
[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/ginger51011/pandoras_pot/rust.yml)](https://github.com/ginger51011/pandoras_pot/actions/)
</div>

# Summary
Inspired by [HellPot](https://github.com/yunginnanet/HellPot), `pandoras_pot`
is an HTTP honeypot that aims to bring even more misery on unruly web crawlers that
don't respect your `robots.txt`.

The goal with `pandoras_pot` is to have maximum data output sent to incoming
unwanted connections, while not using up all the resources of your webserver
that probably could be doing better things with its time.

To ensure that bots don't detect `pandoras_pot`, it generates random data that kind
of looks like a website (to a bot), really *really* fast. Like crazy fast. One could even
say blazingly fast. *Hopefully*.

`pandoras_pot` supports multiple modes of generation, depending on its
configuration. It can for example generate random strings as data, or "actual"
sentances using Markov chains. Neato!

# Features

- Blazingly fast
- Written in Rust
- TOML configuration format, see example below (but sane defaults without config!)
- Optional health port, for reverse proxy health checks
- Multiple generator modes, and it is very easy to add more! Send plain random data, text generated using Markov chains, or a static file!
- Configurable abuse protection (max concurrent producing connections, time and size limits)
- Did I mention that it is written in Rust?

# Setting it up

## Web and Reverse Proxy

The most likely use-case is to use another server as a reverse proxy, and then
select some paths that should be forwarded to `pandoras_pot`, like
`/wp-login.php`, `/.git/config`, and `/.env`.

Note that the URIs you use should have `Disallow` set in your `/robots.txt`,
otherwise you might get in trouble from things like googlebot who will dislike
your strange page of death. For the paths above, you could have a `robots.txt`
like the one below:

```
User-agent: *
Disallow: /wp-login.php
Disallow: /.git
Disallow: /.env
```

Common reverse proxies include `nginx`, `httpd` (apache), and `Caddy`.

In Caddy you could add the following to match the `/robots.txt` we have already created:

```Caddyfile
(pandorust) {
    @pandorust_paths {
        path /wp-login.php /.git* /.env*
    }
    handle @pandorust_paths {
        reverse_proxy localhost:6669 # Or whatever you run pandoras_pot on
    }
}

# ...

example.com {
    # ...
    # Your actual website
    # ...

    import pandorust
}
```

After this you can simply run (if you installed using `cargo install pandoras_pot`):

```sh
pandoras_pot
```

Done!

## Using Docker

The easiest way to set up `pandoras_pot` is using docker. You can optionally
pass an argument to a config file using the docker `--build-arg CONFIG=<path to
your config>` flag (but it should be available in the build context).

Start by cloning the repo by running

```sh
git clone git@github.com:ginger51011/pandoras_pot.git
cd pandoras_pot
```

Then you can build an image and deploy it, here naming and tagging it with `pandoras_pot`
and making it available on port `localhost:6669`:

```sh
docker build -t pandoras_pot . # You can add --build-arg CONFIG=<...> here
docker run --name=pandoras_pot --restart=always -p 6669:8080 -d pandoras_pot
```

## Configuration

`pandoras_pot` uses toml as a configuration format. If you are not using docker,
you can either pass a config like an argument like so:

```sh
pandoras_pot <path-to-config>
```

or put it in a file at `$HOME/.config/pandoras_pot/config.toml`.

A sample file can be found below:

```toml
[http]
# Make sure this matches your Dockerfile's "EXPOSE" if using Docker
port = "8080"
# Routes to send misery to. Is overridden by `http.catch_all`
routes = ["/wp-login.php", "/.env"]
# If all routes are to be served.
catch_all = true
# How many connections that can be made over `http.rate_limit_period` seconds. Will
# not set any limit if set to 0.
rate_limit = 0
# Amount of seconds that `http.rate_limit` checks on. Does nothing if rate limit is set
# to 0.
rate_limit_period = 300 # 5 minutes
# Enables `http.health_port` to be used for health checks (to see if
# `pandoras_pot` is running). Useful if you want to use your chad gaming PC
# that might not always be up and running to back up an instance running on
# your RPi 3 web server.
health_port_enabled = false
# Port to be used for health checks. Should probably not be accessible from the
# outside. Has no effect if `http.health_port_enabled` is `false`.
health_port = "8081"

[generator]
# The size of each generated chunk in bytes. Has a big impact on performance, so
# play around a bit! Note that if this is set too low (like 10 bytes), `pandoras_pot`
# will refuse to run.
chunk_size = 16384 # 1024 * 16
# The type of generator to be used
type = { name = "random" }

# For generator.type it is also possible to set a markov chain generator, using
# a text file as a source of data. Then you can use this (but uncommented, duh):
# type = { name = "markov_chain", data = "<path to some text file>" }

# Another alternative is a static generator, that always outputs the full contents
# of a file. Does not respect chunking.
# type = { name = "static", data = "<path to some file>" }

# The max amount of simultaneous generators that can produce output.
# Useful for preventing abuse. `0` means no limit.
max_concurrent = 100

# The amount of time in seconds a generator can be active before
# it stops sending. `0` means no limit.
time_limit = 0

# The amount of data in bytes that a generator can
# send before it stops sending. `0` means no limit.
size_limit = 0

[logging]
# Output file for logs.
output_path = "pandoras.log"

# If pretty logs should be written to standard output.
print_pretty_logs = true

# If no logs at all should be printed to stdout. Overrides other stdout logging
# settings.
no_stdout = false
```

# Measuring Output

You can easily measure how fast your setup sends data by using `curl`. Note that using
`localhost` might not be reliable, as it does not show what an outsider might see. A better
option might be to use another machine.

This example assume that you have `http.catch_all` enabled, otherwise you should add a
valid route.

```sh
curl localhost:8080/ >> /dev/null
```

# Support

I do not accept any donations. If you however find any software I
write for fun useful, please consider donating to an efficient charity that
save or improve lives the most per `$CURRENCY`.

[GiveWell.org](https://givewell.org) is an excellent website that can help you
donate to the worlds most efficient charities. Alternatives listing the current
best charities for helping our planet is [Founders Pledge](https://www.founderspledge.com/funds/climate-change-fund), and for
animal welfare [Animal Charity Evaluators](https://animalcharityevaluators.org/donation-advice/recommended-charity-fund/).

- Residents of Sweden can do tax-deductable donations to GiveWell via [Ge Effektivt](https://geeffektivt.se)
- Residents of Norway can do the same via [Gi Effektivt](https://gieffektivt.no/)

This list is not exhaustive; your country may have an equivalent.
