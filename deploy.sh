printf "%s" "release tag: "
read tag
rm -rf api/*
cd api
gh release download "$tag" --clobber -R hatchdotlol/api
unzip *_x86_64-unknown-linux-musl.zip
./api &
cd ..