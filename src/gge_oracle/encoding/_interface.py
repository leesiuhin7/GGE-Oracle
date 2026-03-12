import io
from typing import Callable, IO, NotRequired, TypedDict

from ._utils import (
    add_size_header,
    as_null_signed_varint,
    as_null_string,
    as_null_varint_array,
    as_varint,
    from_null_string,
    from_varint,
    get_varint_bytes,
    read_bytes,
)


class UnpackedDeltaRLE(TypedDict):
    size: int
    prev_value: bytes
    offset: bytes
    deltas: bytes
    prev_delta: NotRequired[tuple[bytes, bytes]]


def unpack_delta_RLE(stream: IO[bytes]) -> UnpackedDeltaRLE:
    # | size (varint) | prev (i64) | offset (i32) | deltas |
    size = from_varint(stream)
    prev_value = stream.read(8)
    offset = stream.read(4)
    deltas = stream.read(int.from_bytes(offset))

    unpacked: UnpackedDeltaRLE = {
        "size": size,
        "prev_value": prev_value,
        "offset": offset,
        "deltas": deltas,
    }
    if size > 12:
        unpacked["prev_delta"] = (
            get_varint_bytes(stream),
            get_varint_bytes(stream),
        )

    return unpacked


def new_delta_RLE() -> list[bytes]:
    prev_value = int.to_bytes(0, length=8, signed=True)
    offset = int.to_bytes(0, length=4)
    return add_size_header(prev_value, offset)


def update_delta_RLE(stream: IO[bytes], data: int | None) -> list[bytes]:
    unpacked = unpack_delta_RLE(stream)

    if data is None:
        delta_varint = as_null_signed_varint(None)
        value = unpacked["prev_value"]
    else:
        delta_varint = as_null_signed_varint(
            data - int.from_bytes(unpacked["prev_value"]),
        )
        value = int.to_bytes(data, length=8, signed=True)

    if "prev_delta" in unpacked:
        prev_delta, prev_count = unpacked["prev_delta"]
        if prev_delta == delta_varint:  # Increment count
            new_count = as_varint(
                from_varint(io.BytesIO(prev_count)) + 1,
            )
            return add_size_header(
                value,
                unpacked["offset"],
                unpacked["deltas"],
                prev_delta,
                new_count,
            )
        else:  # Add delta, shift offset
            count_varint = as_varint(1)
            new_offset = int.to_bytes(
                int.from_bytes(unpacked["offset"])
                + len(prev_delta)
                + len(prev_count),
                length=4,
            )
            return add_size_header(
                value,
                new_offset,
                unpacked["deltas"],
                prev_delta,
                prev_count,
                delta_varint,
                count_varint,
            )
    else:  # Add delta
        count_varint = as_varint(1)
        return add_size_header(
            value,
            unpacked["offset"],
            unpacked["deltas"],
            delta_varint,
            count_varint,
        )


class UnpackedNormalRLE(TypedDict):
    size: int
    offset: bytes
    data: bytes
    prev: NotRequired[bytes]


def unpack_normal_RLE(stream: IO[bytes]) -> UnpackedNormalRLE:
    # | size (varint) | offset (i32) | data (| value | count (varint) |) |
    size = from_varint(stream)
    offset = stream.read(4)
    offset_int = int.from_bytes(offset)
    data = stream.read(offset_int)

    unpacked: UnpackedNormalRLE = {
        "size": size,
        "offset": offset,
        "data": data,
    }

    read_size = size - 4 - offset_int  # Size of the final data
    if read_size > 0:
        unpacked["prev"] = stream.read(read_size)

    return unpacked


def new_normal_RLE() -> list[bytes]:
    offset = int.to_bytes(0, length=4)
    return add_size_header(offset)


def update_normal_RLE(
    stream: IO[bytes],
    data: bytes,
    read_data_func: Callable[[IO[bytes]], bytes],
) -> list[bytes]:
    unpacked = unpack_normal_RLE(stream)

    if "prev" in unpacked:
        # NOTE: This directly access stream, changes in layout may affect this
        stream.seek(-len(unpacked["prev"]), 1)  # Shift pointer to re-read
        prev_data = read_data_func(stream)
        count = from_varint(stream)

        if prev_data == data:
            new_count = as_varint(count + 1)
            return add_size_header(
                unpacked["offset"],
                unpacked["data"],
                prev_data,
                new_count,
            )
        else:
            count_varint = as_varint(1)
            new_offset = int.to_bytes(
                int.from_bytes(unpacked["offset"]) + len(unpacked["prev"]),
                length=4,
            )
            return add_size_header(
                new_offset,
                unpacked["data"],
                unpacked["prev"],
                data,
                count_varint,
            )
    else:
        count_varint = as_varint(1)
        return add_size_header(
            unpacked["offset"],
            unpacked["data"],
            data,
            count_varint,
        )


class UnpackedDeltaData(TypedDict):
    size: int
    prev_value: bytes
    deltas: bytes


def unpack_delta_data(stream: IO[bytes]) -> UnpackedDeltaData:
    # | size (varint) | prev (i32) | deltas (varint) |
    size = from_varint(stream)
    return {
        "size": size,
        "prev_value": stream.read(4),
        "deltas": stream.read(size - 4),
    }


def new_delta_data() -> list[bytes]:
    prev_value = int.to_bytes(0, length=4, signed=True)
    return add_size_header(prev_value)


def update_delta_data(stream: IO[bytes], data: int | None) -> list[bytes]:
    unpacked = unpack_delta_data(stream)

    if data is None:
        delta_varint = as_null_signed_varint(None)
        return add_size_header(
            unpacked["prev_value"],
            unpacked["deltas"],
            delta_varint,
        )
    else:
        delta_varint = as_null_signed_varint(
            data - int.from_bytes(unpacked["prev_value"])
        )
        return add_size_header(
            int.to_bytes(data, length=4, signed=True),
            unpacked["deltas"],
            delta_varint,
        )


class StreamInterface:
    def __init__(self, stream: IO[bytes]) -> None:
        self._in_stream = stream
        self._out_stream: list[bytes] = []

    def consume(self) -> bytes:
        output = b"".join(self._out_stream)
        self._out_stream.clear()
        return output

    def skip(self) -> None:
        self._out_stream.append(read_bytes(self._in_stream))

    def get_size(self) -> int:
        return from_varint(self._in_stream)

    def get_id(self) -> int:
        return int.from_bytes(self._in_stream.read(4))

    def get_server(self) -> str:
        server = from_null_string(self._in_stream)
        if server is None:
            raise ValueError

        return server

    def update_timestamp(self, timestamp: int) -> None:
        self._out_stream.extend(update_delta_data(self._in_stream, timestamp))

    def update_timer(self, timer: int | None) -> None:
        self._out_stream.extend(update_delta_RLE(self._in_stream, timer))

    def update_delta_data(self, data: int | None) -> None:
        self._out_stream.extend(update_delta_RLE(self._in_stream, data))

    def update_int(self, value: int | None) -> None:
        self._out_stream.extend(update_normal_RLE(
            self._in_stream,
            as_null_signed_varint(value),
            get_varint_bytes,
        ))

    def update_string(self, value: str | None) -> None:
        self._out_stream.extend(update_normal_RLE(
            self._in_stream,
            as_null_string(value),
            lambda stream: read_bytes(stream, size_start=1),
        ))

    def update_varint_array(self, array: list[int] | None) -> None:
        self._out_stream.extend(update_normal_RLE(
            self._in_stream,
            as_null_varint_array(array),
            lambda stream: read_bytes(stream, size_start=1),
        ))
