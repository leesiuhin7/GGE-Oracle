from dataclasses import dataclass


@dataclass
class ClientConfig:
    url: str
    server: str
    username: str
    password: str
