build static Linux release with:
  rustup target install i686-unknown-linux-musl
  git stash -u
  cargo build --target=i686-unknown-linux-musl --release
  upx target/i686-unknown-linux-musl/release/timeplot
  git stash pop
