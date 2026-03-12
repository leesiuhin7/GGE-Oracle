from ._client import Client
from ._session import MessageGenerator, Session


def set_interval(interval: float) -> None:
    Client.INTERVAL = interval


def set_default_sample_size(sample_size: int) -> None:
    Client.DEFAULT_SAMPLE_SIZE = sample_size


def set_version(version: int) -> None:
    MessageGenerator.VERSION = version


def set_silence_timeout(silence_timeout: float) -> None:
    Session.SILENCE_TIMEOUT = silence_timeout
