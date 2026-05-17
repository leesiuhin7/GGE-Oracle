import asyncio
import shutil

import zstandard as zstd


async def cancel_futures(*futures: asyncio.Future) -> None:
    for future in futures:
        future.cancel()
    await asyncio.gather(*futures, return_exceptions=True)


def decompress_file(src_path: str, dst_path: str) -> None:
    decompressor = zstd.ZstdDecompressor()
    with (
        open(src_path, "rb") as src_file,
        open(dst_path, "wb") as dst_file,
    ):
        decompressor.copy_stream(src_file, dst_file)


def copy_file(src_path: str, dst_path: str) -> None:
    shutil.copyfile(src_path, dst_path)
