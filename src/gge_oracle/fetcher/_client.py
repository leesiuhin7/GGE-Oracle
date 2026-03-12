import asyncio
import time
from typing import AsyncGenerator

from ._client_config import ClientConfig
from ._decoder import unpack_smaple_size
from ._session import Session


class Client:
    INTERVAL: float
    DEFAULT_SAMPLE_SIZE: int

    def __init__(self, config: ClientConfig) -> None:
        self._config = config
        self._session = Session(self._config)

    async def fetch_msgs(self) -> AsyncGenerator[str, None]:
        # Start new session if needed
        if not self._session.active:
            self._session = Session(self._config)
            await self._session.start()

        return self._fetch_msgs()

    async def _fetch_msgs(self) -> AsyncGenerator[str, None]:
        server = self._config.server
        for i in range(6):
            count = 5
            max_count = self.DEFAULT_SAMPLE_SIZE
            is_default: bool = True

            while count < max_count + 5:
                start = time.perf_counter()
                # Send message
                msg = "".join([
                    f'%xt%{server}%hgh%1%',
                    "{",
                    f'"LT":6,"LID":{i + 1},"SV":"{min(count, max_count + 5)}"'
                    "}%",
                ])
                count += 10

                success = await self._session.send(msg)
                for msg in self._session.recv_all():
                    if is_default:
                        sample_size = unpack_smaple_size(msg)
                        if sample_size is not None:
                            max_count = sample_size

                    yield msg

                # NOTE: This assumes there will not be any messages in
                # the future if send fails
                if not success:
                    return

                # Enfore minimum interval
                await asyncio.sleep(
                    max(start + self.INTERVAL - time.perf_counter(), 0),
                )
