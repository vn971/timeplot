TimePlot -- personal time tracking & plotting.

## Installation

* Download the binary from https://abc.def/TODO
* Make it executable (for example, `chmod +x timeplot`)
* Run it
* Watch your productivity graph in ~/.cache/timeplot/  Optionally, point your OS "background" image here so the graph will be your Desktop background.
* Install `gnuplot`.
* Optional. If you want to use the automated category decider instead of writing your own script, install `xdotool`.
* Optional. If you want smarter activity detection, install `xprintidle`.


## Building

1. Install Rust "musl" target:  rustup target install x86_64-unknown-linux-musl
2. build with:  cargo build --target=x86_64-unknown-linux-musl --release
3. shrink the binary size if you're OK to lose some of the backtrace descriptions:
```
  strip target/x86_64-unknown-linux-musl/release/timeplot
```


TODO: document how to write filters
TODO: document how to check logs and update filters
