"""Stand-in for the HiFi API, for load tests.

Serves the two endpoints the backend calls when a track is queued: its
metadata (/info/) and its playback manifest (/track/), the latter with a
direct file URL so no further request is needed.
"""

import base64
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse


def track_id(query):
    try:
        return int(query.get("id", ["0"])[0])
    except ValueError:
        return 0


def info(track):
    return {
        "version": "2.0",
        "data": {
            "id": track,
            "title": f"Load test track {track}",
            "duration": 180,
            "audioQuality": "LOSSLESS",
            "album": {"title": "Load test", "cover": None},
            "artists": [{"name": "Load test"}],
        },
    }


def playback(track):
    manifest = {
        "mimeType": "audio/flac",
        "codecs": "flac",
        "encryptionType": "NONE",
        "urls": [f"https://audio.invalid/{track}.flac"],
    }
    return {
        "version": "2.0",
        "data": {
            "trackId": track,
            "manifestMimeType": "application/vnd.tidal.bts",
            "manifest": base64.b64encode(json.dumps(manifest).encode()).decode(),
        },
    }


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        url = urlparse(self.path)
        track = track_id(parse_qs(url.query))
        if track <= 0:
            return self.reply(404, {"detail": "not found"})
        if url.path.rstrip("/") == "/info":
            return self.reply(200, info(track))
        if url.path.rstrip("/") == "/track":
            return self.reply(200, playback(track))
        return self.reply(404, {"detail": "not found"})

    def reply(self, status, body):
        payload = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, *args):
        pass


if __name__ == "__main__":
    ThreadingHTTPServer(("0.0.0.0", 8000), Handler).serve_forever()
