import asyncio
import logging
import os
import time
from dataclasses import dataclass

import quart

from gge_oracle import utils
from gge_oracle.auth import Authenticator
from gge_oracle.config import Config
from gge_oracle.fetcher import Manager
from gge_oracle.fetcher import config as fetcher_config
from gge_oracle.storage import File, Storage
from gge_oracle.updater import Updater

logger = logging.getLogger(__name__)

app = quart.Quart(__name__)


class Services:
    auth: Authenticator
    file: File


@app.route("/ping")
async def keep_alive() -> quart.Response:
    return quart.Response(status=200)


@app.route("/file_id")
async def send_file_id() -> quart.Response:
    return quart.Response(os.environ.get("FILE_ID"))


@app.route("/commands/upload")
async def save_file() -> quart.Response:
    otp = quart.request.args.get("otp")
    if otp is None:
        # Require OTP
        return quart.Response(status=400)

    if not Services.auth.verify(otp):
        # OTP is incorrect
        logger.info("Unauthorized upload command attempt failed.")
        return quart.Response(status=401)

    Services.file.force_next_upload()
    logger.info("Upload command succeeded.")
    return quart.Response(status=200)


@dataclass
class Context:
    config: Config
    file: File
    manager: Manager


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
    file = context.file
    manager = context.manager

    # NOTE: Separate input and output files are used as the input file
    # acts as a checkpoint to prevent corruption / data loss in cases
    # where updating the output file fails midway
    DATA_DIR = os.path.abspath("data")
    INPUT_FILEPATH = os.path.join(DATA_DIR, "current.dat")
    DECOMPRESSED_INPUT_FILEPATH = os.path.join(DATA_DIR, "decompressed.dat")
    OUTPUT_FILEPATH = os.path.join(DATA_DIR, "output.dat")

    # Create data directory if it doesn't exist
    os.makedirs(DATA_DIR, exist_ok=True)

    # Downloads file if it doesn't exist locally
    await asyncio.to_thread(file.download, INPUT_FILEPATH)

    # Decompress first to improve access speed
    await asyncio.to_thread(
        utils.decompress_file,
        INPUT_FILEPATH,
        DECOMPRESSED_INPUT_FILEPATH,
    )

    updater = Updater(
        DECOMPRESSED_INPUT_FILEPATH,
        OUTPUT_FILEPATH,
    )
    async with updater:
        async for player_info in manager.fetch_player_info(
            config.fetch_timeout,
            max_buffer=1000,  # Limit memory usage
        ):
            await asyncio.to_thread(updater.update, document=player_info)

    # Copy output as input for the next update
    await asyncio.to_thread(utils.copy_file, OUTPUT_FILEPATH, INPUT_FILEPATH)
    # Upload output if sync is needed
    await asyncio.to_thread(file.upload, OUTPUT_FILEPATH)


async def main() -> None:
    logging.basicConfig()

    CONFIG_FILEPATH = os.environ.get("CONFIG_FILEPATH")
    CREDS_FILEPATH = os.environ.get("CREDS_FILEPATH")
    FILE_ID = os.environ.get("FILE_ID")
    TOTP_URI = os.environ.get("TOTP_URI")

    SYNC_INTERVAL = float(os.environ.get("SYNC_INTERVAL", 0))

    if (
        CONFIG_FILEPATH is None
        or CREDS_FILEPATH is None
        or FILE_ID is None
        or TOTP_URI is None
    ):
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
    file = storage.get_file(FILE_ID, sync_interval=SYNC_INTERVAL)

    Services.auth = Authenticator(TOTP_URI)
    Services.file = file

    PORT = int(os.environ.get("PORT", 10000))
    app_task = asyncio.create_task(app.run_task(host="0.0.0.0", port=PORT))

    while True:
        start = time.perf_counter()
        try:
            await update(Context(
                config=config,
                file=file,
                manager=manager,
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
