"""Sube un AAB a una pista de Google Play (spec `tablet-distribution`).

Uso: PLAY_SERVICE_ACCOUNT_JSON='{...}' python play_upload.py app-release.aab [pista]

La pista por defecto es `internal`. La cuenta de servicio necesita permiso de
publicación sobre la app en Play Console. La primera subida de una app nueva
tiene que hacerse a mano en Play Console; después, este script.
"""
import json
import os
import sys

from google.oauth2 import service_account
from googleapiclient.discovery import build
from googleapiclient.http import MediaFileUpload

PACKAGE = "com.lumik.tablet"


def main() -> None:
    aab = sys.argv[1]
    track = sys.argv[2] if len(sys.argv) > 2 else "internal"
    info = json.loads(os.environ["PLAY_SERVICE_ACCOUNT_JSON"])
    creds = service_account.Credentials.from_service_account_info(
        info, scopes=["https://www.googleapis.com/auth/androidpublisher"]
    )
    edits = build("androidpublisher", "v3", credentials=creds, cache_discovery=False).edits()

    edit_id = edits.insert(packageName=PACKAGE, body={}).execute()["id"]
    bundle = edits.bundles().upload(
        packageName=PACKAGE,
        editId=edit_id,
        media_body=MediaFileUpload(aab, mimetype="application/octet-stream", resumable=True),
    ).execute()
    version_code = bundle["versionCode"]
    edits.tracks().update(
        packageName=PACKAGE,
        editId=edit_id,
        track=track,
        body={"track": track, "releases": [{"versionCodes": [str(version_code)], "status": "completed"}]},
    ).execute()
    edits.commit(packageName=PACKAGE, editId=edit_id).execute()
    print(f"versionCode {version_code} publicado en la pista {track}")


if __name__ == "__main__":
    main()
