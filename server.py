from http.server import HTTPServer, BaseHTTPRequestHandler, SimpleHTTPRequestHandler
from http import HTTPStatus
import urllib.parse
import posixpath
import mimetypes
import shutil

import hashlib
import struct

import ssl
import os

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

def packString(buf, string):

    string = string.encode("utf-8")
    buf.extend(struct.pack("<H", len(string)))
    buf.extend(string)

def packFileVersionRecord(buf, filename):
    
    packString(buf, filename)

def generateFileVersionRecords(files):

    buf = bytearray()
    filehashes = []
    size = 0
    
    for filename in files:
        with open(filename, "rb") as f:
            data = f.read()
            hasher = hashlib.new("sha256")
            hasher.update(data)
            filehash = hasher.hexdigest()
            filehashes.append((filename, hasher.digest()))
            size += len(data)

    hasher = hashlib.new("sha256")
    # sort so that the order of files doesn't matter
    for (filename, filehash) in sorted(filehashes):
        hasher.update(filename.encode("utf-8"))
        hasher.update(bytes(1)) # 0-byte separator
        hasher.update(filehash)
        hasher.update(bytes(1))
        
    apphash = hasher.hexdigest()
    print("\n\nCurrent version: " + apphash + "\n")
    packString(buf, apphash)
    buf.extend(struct.pack("<I", size))
    buf.extend(struct.pack("<H", len(files)))

    for filename in files:
        packFileVersionRecord(buf, filename)

    return buf

class Handler(BaseHTTPRequestHandler):

    server_version = "calm/0.1"

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)

    def do_GET(self):

        path = urllib.parse.urlsplit(self.path).path
        path = urllib.parse.unquote(path)
        path = posixpath.normpath(path)

        components = path.split("/")[1:]
        path = "."
        for component in components:
            if component == os.pardir:
                self.send_response(HTTPStatus.FORBIDDEN)
                self.end_headers()
                return
            path += "/" + component

        print()
        print("Request for path: " + path)
        print("Headers:")
        for (header, value) in self.headers.items():
            print("  " + header + ": " + value)
        print()

        if path == "./":
            path = "./index.html"

        if path == "./serviceworker.js":
            path = "./serviceworker-release.js"
            #pass

        if path == "./version":
            print("getting version...")
            data = generateFileVersionRecords(versioned_files)
            content_type = "application/octet-stream"

            self.send_response(HTTPStatus.OK)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", len(data))
            self.end_headers()
            self.wfile.write(data)
            print()
            print(data)
            print()
            return

        if path.split("/")[1] == "versioned":
            version, *path = path.split("/")[2:]
            path = "./" + "/".join(path)

        else:
            version = None

        if os.path.isfile(path):
            content_type = mimetypes.guess_type(path)[0]
            if content_type is None:
                if path.endswith(".wasm"):
                    content_type = "application/wasm"
                else:
                    content_type = "text/plain"

            with open(path, "rb") as f:
                #if version:
                #    # hash file to check version
                #    hasher = hashlib.new("sha256")
                #    data = f.read()
                #    hasher.update(data)

                #    print("requested version: {}".format(version))
                #    print("current version:   {}".format(hasher.hexdigest()))

                #    if version != hasher.hexdigest():
                #        print ("VERSION MISMATCH!!")
                #        self.send_response(HTTPStatus.NOT_FOUND)
                #        self.end_headers()
                #        return

                #    print ("all good, versions match.")
                
                content_length = f.seek(0, 2);
                f.seek(0, 0) # go back to start

                self.send_response(HTTPStatus.OK)
                self.send_header("Content-Type", content_type)
                self.send_header("Content-Length", content_length)
                self.end_headers()
                print("headers sent")

                shutil.copyfileobj(f, self.wfile)

        else:
            self.send_response(HTTPStatus.NOT_FOUND)
            self.end_headers()
            
server = HTTPServer(("0.0.0.0", 5001), Handler)
server.socket = ssl.wrap_socket(server.socket, keyfile="/home/antonius/minica/portland-3/key.pem", certfile="/home/antonius/minica/portland-3/cert.pem", server_side=True)
server.serve_forever()
