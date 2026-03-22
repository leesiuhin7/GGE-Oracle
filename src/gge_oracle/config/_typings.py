from typing import TypedDict


class Client(TypedDict):
    url: str
    server: str
    username: str
    password: str


class ClientConfig(TypedDict):
    clients: list[Client]
    silence_timeout: float
    version: int


class FetcherConfig(TypedDict):
    default_sample_size: int
    interval: float
    msg_interval: float
    timeout: float


class LoggerConfig(TypedDict):
    name: str | None
    level: int


class Config(TypedDict):
    client: ClientConfig
    fetcher: FetcherConfig
    logging: list[LoggerConfig]
