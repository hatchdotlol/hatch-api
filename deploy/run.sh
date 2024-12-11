if [ "$(git pull | grep -i Already)" = "Already up to date." ]; then
   exit 0;
fi

pkill api
cargo build --release
ADMIN_KEY="use a secure key for this" VERSION="$(git rev-parse --short --verify main)" ./target/release/api &