set -e

tag=`gh release list -R hatchdotlol/hatch-api --json name | jq -r ".[0].name"`
if [ $(ls /home/server/api | grep "$tag") != "" ]; then
        exit 0
fi

rm -rf api/*
cd api
gh release download "$tag" -p "hatch-api_*_x86_64-unknown-linux-musl.zip" --clobber -R hatchdotlol/api
unzip *_x86_64-unknown-linux-musl.zip
mv ./api "./api_$tag"
rm *.zip
pkill api*
./api* &
cd ..
