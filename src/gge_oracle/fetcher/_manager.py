import asyncio
import time
from dataclasses import dataclass
from typing import Any, AsyncGenerator, Awaitable, Callable

from gge_oracle.typings import PlayerDocument
from gge_oracle.utils import cancel_futures

from ._client import Client
from ._client_config import ClientConfig
from ._decoder import unpack_player_info


@dataclass
class ClientCollection:
    client: Client
    config: ClientConfig


class Manager:
    def __init__(self) -> None:
        self._collections: dict[int, ClientCollection] = {}
        self._next_client_id: int = 0

    def add_client(self, config: ClientConfig) -> int:
        client_id = self._next_client_id
        self._next_client_id += 1

        collection = ClientCollection(
            client=Client(config),
            config=config,
        )
        self._collections[client_id] = collection
        return client_id

    def remove_client(self, client_id: int) -> None:
        self._collections.pop(client_id, None)

    async def fetch_player_info(
        self,
        timeout: float,
        max_buffer: int = 0,
    ) -> AsyncGenerator[PlayerDocument, None]:
        queue: asyncio.Queue[PlayerDocument | None] = asyncio.Queue(max_buffer)
        waiter_future = asyncio.gather(
            *(
                self._consume_msgs(collection, queue.put)
                for collection in self._collections.values()
            ),
            return_exceptions=True,
        )
        # Signal EOF with None after all tasks are done
        waiter_future.add_done_callback(
            lambda _: asyncio.create_task(queue.put(None)),
        )

        try:
            end_time = time.perf_counter() + timeout
            # Consume until EOF or timeout
            while (document := await asyncio.wait_for(
                queue.get(),
                end_time - time.perf_counter(),
            )) is not None:
                yield document
        except asyncio.TimeoutError:
            pass

        # Collect tasks in case they raised exceptions
        await cancel_futures(waiter_future)

    async def _consume_msgs(
        self,
        collection: ClientCollection,
        emit: Callable[[PlayerDocument], Awaitable[Any]],
    ) -> None:
        server = collection.config.server
        async for msg in await collection.client.fetch_msgs():
            timestamp = int(time.time())
            player_info = unpack_player_info(msg, server, timestamp)
            for document in player_info:
                await emit(document)
