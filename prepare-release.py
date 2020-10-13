import os
import sys
import shutil

import hashlib
import struct
import json

from git import Repo


def packString(buf, string):

    string = string.encode("utf-8")
    buf.extend(struct.pack("<H", len(string)))
    buf.extend(string)

def packFileVersionRecord(buf, filename):
    
    packString(buf, filename)

def generateFileVersionRecords(files, version):

    buf = bytearray()
    size = 0
    
    for filename in files:
        with open(os.path.join("public", filename), "rb") as f:
            size += f.seek(0, 2)
        
    packString(buf, version)
    buf.extend(struct.pack("<I", size))
    buf.extend(struct.pack("<H", len(files)))

    for filename in files:
        packFileVersionRecord(buf, filename)

    return buf


#        if path == "./serviceworker.js":
#            path = "./serviceworker-release.js"
#            #pass


target_dir = sys.argv[1]

versioned_files = [
    "./index.html",
    "./global.css",
    "./favicon.png",
    "./app.webmanifest",
    "./icon.png",
    "./build/bundle.css",
    "./build/bundle.js",
    "./helpers.wasm",
    "./load-icon.svg",
    "./abc-icon.svg",
    "./STK-icon.svg",
    "./expand-icon.svg",
    "./collapse-icon.svg"
]

repo = Repo(".")

is_working_directory_clean = len(repo.head.commit.diff(None)) == 0
if not is_working_directory_clean:
    print("There are unsaved changes. Please save them before making a release.")
    sys.exit(1)

with open("package.json") as f:
    package_info = json.load(f)
    version = package_info["version"]

print("Read version from package.json: \"{}\"".format(version))

if version in repo.tags:
    if repo.tags[version].commit != repo.head.commit:
        print("There is already a tag with this version on a different commit. Please update the version.")
        sys.exit(1)

else:
    print("This version is not yet tagged, tagging...")
    repo.create_tag(version)

os.system("npm run build")

print("Generating version info...")
    
packed_version_info = generateFileVersionRecords(versioned_files, version)

print("Copying files...")

versioned_dir = os.path.join(target_dir, "versioned", version)

try:
    # also create the 'build' subdirectory in the same go
    os.makedirs(os.path.join(versioned_dir, "build"))
except FileExistsError:
    print("This version's directory already exists. Overwriting versions is not something that should happen.")
    sys.exit(1)

# make sure the base 'build' directory exists, too
os.makedirs(os.path.join(target_dir, "build"), exist_ok=True)

with open(os.path.join(target_dir, "version"), "wb") as f:
    f.write(packed_version_info)

for fname in versioned_files:
    shutil.copyfile(os.path.join("public", fname), os.path.join(versioned_dir, fname))
    shutil.copyfile(os.path.join("public", fname), os.path.join(target_dir, fname))

shutil.copyfile("public/serviceworker-release.js", os.path.join(target_dir, "serviceworker.js"))

print("Done!")
