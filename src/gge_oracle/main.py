import asyncio
import logging
import lzma
import os
import time
from dataclasses import dataclass

import quart

from gge_oracle import utils
from gge_oracle.config import Config
from gge_oracle.encoding import Updater
from gge_oracle.fetcher import Manager
from gge_oracle.fetcher import config as fetcher_config
from gge_oracle.storage import Storage

logger = logging.getLogger(__name__)

app = quart.Quart(__name__)


@app.route("/ping")
async def keep_alive() -> quart.Response:
    return quart.Response(status=200)


@app.route("/file_id")
async def send_file_id() -> quart.Response:
    return quart.Response(os.environ.get("FILE_ID"))


@dataclass
class Context:
    config: Config
    file_id: str
    manager: Manager
    storage: Storage


def set_fetcher_config(config: Config) -> None:
    fetcher_config.set_interval(config.msg_interval)
    fetcher_config.set_default_sample_size(config.default_sample_size)
    fetcher_config.set_version(config.client_version)
    fetcher_config.set_silence_timeout(config.client_timeout)


def set_logging_config(config: Config) -> None:
    for logger_config in config.logging:
        logging.getLogger(
            logger_config["name"],
        ).setLevel(logger_config["level"])


async def update(context: Context) -> None:
    config = context.config
    file_id = context.file_id
    manager = context.manager
    storage = context.storage

    DATA_DIR = os.path.abspath("data")
    INPUT_FILEPATH = os.path.join(DATA_DIR, "current.dat")
    DECOMPRESSED_INPUT_FILEPATH = os.path.join(DATA_DIR, "decompressed.dat")
    OUTPUT_FILEPATH = os.path.join(DATA_DIR, "output.dat")

    # Create data directory if it doesn't exist
    os.makedirs(DATA_DIR, exist_ok=True)

    storage.download(file_id, INPUT_FILEPATH)
    # Decompress first to improve access speed
    utils.decompress_file(INPUT_FILEPATH, DECOMPRESSED_INPUT_FILEPATH)

    with (
        open(DECOMPRESSED_INPUT_FILEPATH, "rb") as input_file,
        lzma.open(OUTPUT_FILEPATH, "wb") as output_file
    ):
        updater = Updater(input_file, output_file)
        updater.init()

        async for player_info in manager.fetch_player_info(
            config.fetch_timeout,
            max_buffer=1000,  # Limit memory usage
        ):
            await asyncio.to_thread(updater.update, document=player_info)

        updater.finalize()

    storage.upload(file_id, OUTPUT_FILEPATH)  # Save output


async def main() -> None:
    logging.basicConfig()

    CONFIG_FILEPATH = os.environ.get("CONFIG_FILEPATH")
    CREDS_FILEPATH = os.environ.get("CREDS_FILEPATH")
    FILE_ID = os.environ.get("FILE_ID")
    if CONFIG_FILEPATH is None or CREDS_FILEPATH is None or FILE_ID is None:
        logger.critical(
            "Mandatory environment variables are missing. Exiting.",
        )
        return

    # Config
    config = Config.from_file(os.path.abspath(CONFIG_FILEPATH))
    set_logging_config(config)
    set_fetcher_config(config)

    manager = Manager()
    for client in config.clients:
        manager.add_client(client)

    storage = Storage()
    storage.authenticate(os.path.abspath(CREDS_FILEPATH))

    PORT = int(os.environ.get("PORT", 10000))
    app_task = asyncio.create_task(app.run_task(host="0.0.0.0", port=PORT))

    while True:
        start = time.perf_counter()
        try:
            await update(Context(
                config=config,
                file_id=FILE_ID,
                manager=manager,
                storage=storage,
            ))
        except Exception as e:
            logger.exception(e)
        else:
            logger.info(
                f"Update succeeded in {time.perf_counter() - start:.2f}s.")

        # Enfore interval
        await asyncio.sleep(
            max(start + config.fetch_interval - time.perf_counter(), 0),
        )


if __name__ == "__main__":
    asyncio.run(main())
