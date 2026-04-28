import asyncio

from .native import Updater as NativeUpdater
from .native import typings  # type: ignore


class Updater:
    def __init__(self, input_filename: str, output_filename: str) -> None:
        self._updater = NativeUpdater(input_filename, output_filename)

    async def __aenter__(self) -> None:
        await asyncio.to_thread(self._updater.__enter__)

    async def __aexit__(self, exc_type, exc, tb):
        return await asyncio.to_thread(self._updater.__exit__, exc_type, exc, tb)

    def __del__(self) -> None:
        del self._updater

    def update(self, document: typings.Document) -> None:
        self._updater.update(document)
