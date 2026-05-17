import json
import math
from pathlib import Path
import time

from pydrive2.auth import GoogleAuth
from pydrive2.drive import GoogleDrive
from pydrive2.files import GoogleDriveFile


class File:
    def __init__(self, file: GoogleDriveFile, sync_interval: float = 0) -> None:
        self._file: GoogleDriveFile = file
        self._prev_upload: float = time.monotonic()
        self._sync_interval: float = sync_interval

    def upload(self, filepath: str, force: bool = False) -> bool:
        """
        Upload the content of the file specified by filepath if
        enough time has elapsed since the previous upload.

        :param filepath: the path of the file
        :type filepath: str
        :param force: whether the file should be uploaded even if the 
            elapsed time since the previous upload is less than 
            sync_interval, defaults to False
        :type force: bool, optional
        :return: False if the file was not uploaded, True otherwise
        :rtype: bool
        """
        time_elapsed = time.monotonic() - self._prev_upload
        if (not force) and (time_elapsed < self._sync_interval):
            return False

        self._file.SetContentFile(filepath)
        self._file.Upload()
        self._prev_upload = time.monotonic()
        return True

    def download(self, filepath: str, force: bool = False) -> bool:
        """
        Download file to filepath if it doesn't already exist.

        :param filepath: the path where the file should be written to
        :type filepath: str
        :param force: whether the file at the specified path should be 
            overwritten if it exists, defaults to False
        :type force: bool, optional
        :return: False if the file was not downloaded, True otherwise
        :rtype: bool
        """
        path = Path(filepath)
        if (not force) and path.is_file():
            return False

        self._file.GetContentFile(filepath)
        return True

    def force_next_upload(self) -> None:
        """
        Forces the next :meth:`upload` to upload the file. This is
        identical to setting `force=True` for next :meth:`upload`
        """
        self._prev_upload = -math.inf


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

    def get_file(self, file_id: str, sync_interval: float = 0) -> File:
        file = self._drive.CreateFile({"id": file_id})
        return File(file, sync_interval)
