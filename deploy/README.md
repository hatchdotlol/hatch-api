# Steps to deploy

- install git, rust toolchain, and minio (and python but you should probably have it)
- authenticate if needed and clone this repo to `hatch-api`
- start a minio server in the bg with api on `localhost:9000`
  - make a bucket named "pfps" and one named "assets" until rust does it for you
- `python3 run_this.py`
- host port 8000 to the world (in my case, tell gr that im going to copy-paste commands again)
