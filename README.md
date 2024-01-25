<div align="center">
    <h1>🔥pandoras_pot🍯</h1>
    <i>Unleash Unfathomable Curses on Unsuspecting Bots... In Rust!</i>
</div>
<br />

[![Crates.io (pandoras_pot)](https://img.shields.io/crates/v/pandoras_pot)](https://crates.io/crates/cvars)

## Summary
Inspired by [HellPot](https://github.com/yunginnanet/HellPot), `pandoras_pot`
aims to bring even more misery on unruly web crawlers that doesn't respect your
`robots.txt`.

The goal with `pandoras_pot` is to have maximum data output, while not using up
all the resources of your webserver that probably could be doing better things
with its time.

To ensure that bots don't detect `pandoras_pot`, it generates random data that kind
of looks like a website (to a bot), really *really* fast. Like crazy fast.

## Setting it up

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

### Using Docker

The easiest way to set up `pandoras_pot` is using docker. You can optionally
pass an argument to a config file using the docker `--build-arg CONFIG=<path to
your config>` flag (but it should be available in the build context).

To build an image and deploy it, here naming and tagging it with `pandoras_pot`
and making it available on port `localhost:6669`, you can run the following:

```sh
docker build -t pandoras_pot . # You can add --build-arg CONFIG=<...> here
docker run --name=pandoras_pot --restart=always -p 6669:8080 -d pandoras_pot
```

## Configuration

`pandoras_pot` uses toml as a configuration format. If you are not using docker,
you can either pass a config like an argument like so:

```sh
./pandoras_pot <path-to-config>
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

[generator]
# Changing these will drastically impact performance. Play around a bit!
# The minimum possible length of a generated string segment
min_chunk_size = 1024
# The maximum possible length of a generated string segment
max_chunk_size = 8000

[logging]
# Output file for logs. Will not write to logs if
# not present.
output_path = "pandoras.log"

# If pretty logs should be written to standard output.
print_pretty_logs = true

# If no logs at all should be printed to stdout. Overrides other stdout logging
# settings.
no_stdout = false
```

## Measuring Output

One simple command to measure output is to use the following command on
GNU/Linux (make sure to install pipe viewer `pv`):

```sh
curl -s localhost:8080/ | pv >> /dev/null
```

this assumes that the `/` route is served by `pandoras_pot`.
