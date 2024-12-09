# assumes script is running in regular shell in cwd
# please do so
mkdir api

# get the latest release tag
tag=`gh release list -R hatchdotlol/hatch-api --json name | jq -r ".[0].name"`

# if the latest exe exists and its not running, run it or skadoodle
if [ $(ls api | grep "$tag") != "" ]; then
        if [ "$(ps | grep api)" = "" ]; then
          ./api/api* &
        fi 
        exit 0
fi

rm -rf api/*
cd api

# download latest release
gh release download "$tag" -p "hatch-api_*_x86_64-unknown-linux-musl.zip" --clobber -R hatchdotlol/api
unzip *_x86_64-unknown-linux-musl.zip

mv ./api "./api_$tag"
rm *.zip
pkill api*

./api* &