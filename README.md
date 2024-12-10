# hatch API

This is the repository for the API used by hatch.lol.

## Running

- Install the Rust toolchain, [Minio](https://min.io/docs/minio/linux/operations/installation.html), and SQLite drivers (on Debian this is `libsqlite-dev` if you don't have it by default)
- Start a Minio server in the background bound to `localhost:9000`
  - Make a bucket named "pfps" and one named "assets"
- `cargo run`

This API has only been tested on Linux.

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

# testing the token
me = requests.get(f"{BASE}/auth/me", headers={"token": token})
print(me.json())
# {
#     'user': '...',
#     'displayName': '...',
#     'country': '...',
#     'bio': '...',
#     'highlightedProjects': '...',
#     'profilePicture': '...',
#     'joinDate': '...'
# }
```
