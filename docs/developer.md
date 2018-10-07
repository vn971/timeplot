list and install rustup targets:
  rustup target list
  rustup target install i686-unknown-linux-musl
  rustup target install i686-pc-windows-gnu

build static Linux release with:
  git stash -u
  cargo build --target=i686-unknown-linux-musl --release
  upx target/i686-unknown-linux-musl/release/timeplot
  git stash pop

build Windows release with:
