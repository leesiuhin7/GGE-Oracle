import json

from pydrive2.auth import GoogleAuth
from pydrive2.drive import GoogleDrive
from pydrive2.files import GoogleDriveFile


class Storage:
    def __init__(self) -> None:
        self._drive: GoogleDrive
        self._files: dict[str, GoogleDriveFile] = {}

    def authenticate(self, filepath: str) -> None:
        with open(filepath, "r") as file:
            data = json.load(file)
        client_email = data["client_email"]

        gauth = GoogleAuth()
        gauth.auth_method = "service"
        gauth.settings["service_config"] = {
            "client_json_file_path": filepath,
            "client_user_email": client_email,
        }
        gauth.ServiceAuth()

        self._drive = GoogleDrive(gauth)

    def upload(self, file_id: str, filepath: str) -> None:
        file = self._get_file(file_id)
        file.SetContentFile(filepath)
        file.Upload()

    def download(self, file_id: str, filepath: str) -> None:
        file = self._get_file(file_id)
        file.GetContentFile(filepath, acknowledge_abuse=True)

    def _get_file(self, file_id: str) -> GoogleDriveFile:
        if file_id in self._files:
            return self._files[file_id]
        return self._drive.CreateFile({"id": file_id})
