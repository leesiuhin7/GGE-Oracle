import json

from gge_oracle.fetcher import ClientConfig

from . import _typings


class Config:
    def __init__(self, config: _typings.Config) -> None:
        self._config = config

    @staticmethod
    def from_file(filepath: str) -> "Config":
        with open(filepath, "r") as file:
            data = json.load(file)

        return Config(data)

    @property
    def client_version(self) -> int:
        return self._config["client"]["version"]

    @property
    def client_timeout(self) -> float:
        return self._config["client"]["silence_timeout"]

    @property
    def clients(self) -> list[ClientConfig]:
        return [
            ClientConfig(
                url=client["url"],
                server=client["server"],
                username=client["username"],
                password=client["password"],
            )
            for client in self._config["client"]["clients"]
        ]

    @property
    def default_sample_size(self) -> int:
        return self._config["fetcher"]["default_sample_size"]

    @property
    def fetch_interval(self) -> float:
        return self._config["fetcher"]["interval"]

    @property
    def msg_interval(self) -> float:
        return self._config["fetcher"]["msg_interval"]

    @property
    def fetch_timeout(self) -> float:
        return self._config["fetcher"]["timeout"]
