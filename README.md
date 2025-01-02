# hatch API

This is the repository for the API used by hatch.lol.

## Running

- Install the Rust toolchain, [MinIO](https://min.io/docs/minio/linux/operations/installation.html), git, and SQLite drivers (on Debian this is `libsqlite-dev` if you don't have it by default)
- Start a MinIO server in the background bound to `localhost:9000`
  - Make a bucket named "pfps" and one named "assets"
  - Add a `default.png` to the pfps bucket for a default profile picture
- Copy `.env.sample` to `.env` and fill out the env variables.
  - You read that right, you will need a [Postal](https://docs.postalserver.io/getting-started) mail server. Postal is a little annoying to set up and also recommends a dedicated server, but sooner or later this dependency should become optional I promise
- Review `src/config.rs` and change values as desired
- `VERSION="$(git rev-parse --short --verify main)" cargo run`

This API has only been tested on Linux. 2-4 GB of RAM is recommended at least.

## Example use

```py
import requests

BASE = "https://api.hatch.lol"

# logging in
login = requests.post(
    f"{BASE}/auth/login",
    json={"username": "...", "password": "..."}
)
token = login.json()["token"]

# getting your info from the token
me = requests.get(f"{BASE}/auth/me", headers={"Token": token})
print(me.json())

# changing user details
update_details = requests.get(f"{BASE}/user", headers={"Token": token}, body={...})
print(update_details.json())
```
