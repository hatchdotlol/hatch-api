from getpass import getpass
import subprocess
import os
import sys
import random

import requests
import paramiko

user = "aj"
host = "192.168.1.6"

directory = f"/home/{user}/deploy"
file = f"hatch-api-{subprocess.check_output(['git', 'rev-parse', 'HEAD']).decode().strip()}-{str(random.random())[2:]}"

print("- Building API")

go_env = os.environ.copy()
go_env["GOOS"] = "linux"
go_env["GOARCH"] = "amd64"
go_env["CGO_ENABLED"] = "1"
go_env["CC"] = "x86_64-linux-musl-gcc"
go_env["CXX"] = "x86_64-linux-musl-g++"

subprocess.check_call(
    ["go", "build", "--ldflags", '-linkmode external -extldflags "-static"', "."],
    env=go_env,
)

print("- Uploading to server")

password = getpass("Password: ")

ssh = paramiko.SSHClient()
ssh.set_missing_host_key_policy(paramiko.AutoAddPolicy())

ssh.connect(host, port=22, username=user, password=password)


def prin(p, t):
    progress = p / t
    arrow = "#" * int(round(progress * 50))
    spaces = " " * (50 - len(arrow))
    sys.stdout.write(f"\r{arrow}{spaces} {int(progress * 100)}%")
    sys.stdout.flush()

    if p == t:
        sys.stdout.write("\n")


api_loc = f"{directory}/{file}"

try:
    sftp = ssh.open_sftp()
    sftp.put("hatch-api", api_loc, prin)
finally:
    sftp.close()

os.remove("hatch-api")

print("- Restarting API")

ssh.exec_command("lsof -i tcp:8080 | awk 'NR!=1 {print $2}' | xargs kill")
ssh.exec_command(
    f"cd {directory} && source .env && chmod +x {api_loc} && {api_loc} >/dev/null 2>&1 &"
)
