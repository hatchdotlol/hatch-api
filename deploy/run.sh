run () {
   RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo build --release
   source .env
   ./target/release/api &
}

if [ "$(git pull | grep -i Already)" = "Already up to date." ]; then
   pgrep api
   running=$?
   
   if [ $running -eq 1 ]; then
      run
   fi

   exit 0;
fi

pkill api
run