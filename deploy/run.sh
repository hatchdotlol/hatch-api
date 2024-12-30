if [ "$(git pull | grep -i Already)" = "Already up to date." ]; then
   exit 0;
fi

pkill api
cargo build --release
source .env ./target/release/api &