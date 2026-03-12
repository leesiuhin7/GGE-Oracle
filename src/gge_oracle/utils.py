import asyncio
import lzma
import shutil


async def cancel_futures(*futures: asyncio.Future) -> None:
    for future in futures:
        future.cancel()
    await asyncio.gather(*futures, return_exceptions=True)


def decompress_file(src_path: str, dst_path: str) -> None:
    with (
        lzma.open(src_path, "rb") as src_file,
        open(dst_path, "wb") as dst_file,
    ):
        shutil.copyfileobj(src_file, dst_file)
