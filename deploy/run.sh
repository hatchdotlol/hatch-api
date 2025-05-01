run () {
   source .env
   go run .
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