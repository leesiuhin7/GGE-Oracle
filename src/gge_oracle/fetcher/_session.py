import asyncio
import random
import ssl
import time

import websockets
from websockets import ClientConnection, ConnectionClosed, Origin

from gge_oracle.utils import cancel_futures

from ._client_config import ClientConfig

SSL_CONTEXT = ssl.create_default_context()
SSL_CONTEXT.check_hostname = False
SSL_CONTEXT.verify_mode = ssl.CERT_NONE


class MessageGenerator:
    VERSION: int

    def __init__(self, config: ClientConfig) -> None:
        self._server = config.server
        self._username = config.username
        self._password = config.password

    @property
    def init_msgs(self) -> list[str]:
        return self._generate_init_msgs()

    @property
    def roundtrip_msg(self) -> str:
        return f"%xt%{self._server}%pin%1%<RoundHouseKick>%"

    @property
    def user_agent(self) -> str:
        return f"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{random.randint(140, 145)}.0.0.0 Safari/537.36"

    @property
    def origin(self) -> str:
        return "https://empire-html5.goodgamestudios.com"

    def _generate_init_msgs(self) -> list[str]:
        # NOTE: Using "undefined" for version date
        session_id = "{:.15e}".format(random.random() * 2 ** 1023)
        init_msgs = [
            f"<msg t='sys'><body action='verChk' r='0'><ver v='{self.VERSION}' /></body></msg>",
            f"<msg t='sys'><body action='login' r='0'><login z='{self._server}'><nick><![CDATA[]]></nick><pword><![CDATA[undefined%en%0]]></pword></login></body></msg>",
            "<msg t='sys'><body action='autoJoin' r='-1'></body></msg>",
            "<msg t='sys'><body action='roundTrip' r='1'></body></msg>",
            f"%xt%{self._server}%vck%1%undefined%web-html5%<RoundHouseKick>%{session_id}%",
            "".join([
                '%xt%',
                self._server,
                '%vln%1%{"NOM":"',
                self._username,
                '"}%',
            ]),
            self._generate_login_msg(),
        ]
        return init_msgs

    def _generate_login_msg(self) -> str:
        # NOTE: Omitting RCT field
        return "".join([
            '%xt%',
            self._server,
            '%lli%1%{"CONM":',
            str(random.randint(100, 300)),
            ',"RTM":',
            str(random.randint(15, 50)),
            ',"ID":0,"PL":1,"NOM":"',
            self._username,
            '","PW":"',
            self._password,
            '","LT":null,"LANG":"en","DID":"0","AID":"',
            f"{int(time.time() * 1000)}{random.randint(0, 999999)}",
            '","KID":"","REF":"https://empire.goodgamestudios.com","GCI":"","SID":9,"PLFID":1}%'
        ])


class Session:
    SILENCE_TIMEOUT: float

    def __init__(self, config: ClientConfig) -> None:
        self._config = config
        self._msg_gen = MessageGenerator(config)
        self._ws: ClientConnection

        self._task: asyncio.Task | None = None

        self._queue: asyncio.Queue[str] = asyncio.Queue()
        self._ready_event = asyncio.Event()

    async def send(self, msg: websockets.Data) -> bool:
        try:
            await self._ready_event.wait()
            await self._ws.send(msg)
        except ConnectionClosed:
            return False

        return True

    async def start(self) -> None:
        if self._task is None:
            self._task = asyncio.create_task(self._connect())

    @property
    def active(self) -> bool:
        if self._task is None:
            return False
        return not self._task.done()

    async def recv(self) -> str:
        return await self._queue.get()

    def recv_all(self) -> list[str]:
        return [self._queue.get_nowait() for _ in range(self._queue.qsize())]

    async def _connect(self) -> None:
        async with websockets.connect(
            self._config.url,
            origin=Origin(self._msg_gen.origin),
            user_agent_header=self._msg_gen.user_agent,
            ssl=SSL_CONTEXT,
        ) as self._ws:
            init_msgs_task = asyncio.create_task(self._send_init_msgs())
            ping_task = asyncio.create_task(self._ping_loop())

            tasks: set[asyncio.Task] = set([init_msgs_task, ping_task])

            _, pending = await asyncio.wait(
                tasks,
                return_when=asyncio.FIRST_COMPLETED,
            )
            if init_msgs_task in pending:
                await cancel_futures(*tasks)
                return

            recv_task = asyncio.create_task(self._recv_loop())
            tasks.discard(init_msgs_task)
            tasks.add(recv_task)

            self._ready_event.set()
            _, pending = await asyncio.wait(
                tasks,
                return_when=asyncio.FIRST_COMPLETED,
            )
            await cancel_futures(*tasks)

    async def _send_init_msgs(self) -> None:
        init_msgs = self._msg_gen.init_msgs

        await self._ws.send(init_msgs[0])
        await self._ws.recv()

        await self._ws.send(init_msgs[1])
        for _ in range(3):
            await self._ws.recv()

        await self._ws.send(init_msgs[2])
        await self._ws.recv()

        await self._ws.send(init_msgs[3])
        await self._ws.send(init_msgs[4])
        for _ in range(2):
            await self._ws.recv()

        await self._ws.send(init_msgs[5])
        await self._ws.recv()

        await self._ws.send(init_msgs[6])

    async def _ping_loop(self) -> None:
        msg = self._msg_gen.roundtrip_msg
        while True:
            await asyncio.sleep(60)
            await self._ws.send(msg)

    async def _recv_loop(self) -> None:
        try:
            while True:
                msg = await asyncio.wait_for(
                    self._ws.recv(True),
                    timeout=self.SILENCE_TIMEOUT,
                )
                self._queue.put_nowait(msg)
        except asyncio.TimeoutError:
            pass
