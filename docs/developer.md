list and install rustup targets:
  rustup target list
  rustup target install i686-unknown-linux-musl
  rustup target install i686-pc-windows-gnu

build static Linux release with:
  tools/release.sh

build Windows release with:


Platform-specific TODO:
* idle time on Windows
* idle time on MacOS
* ?use something more lightweight on Linux?
* fail-safe log parsing (and less .unwrap() in general)
