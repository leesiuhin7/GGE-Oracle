from typing import IO


def zigzag_encode(value: int) -> int:
    return (value << 1) | (value < 0)


def zigzag_decode(value: int) -> int:
    return (value >> 1) ^ -(value & 1)


def as_varint(value: int) -> bytes:
    out = bytearray()

    out.append((value & 0x7f) | 0x80)
    value >>= 7
    while value > 0:
        out.append((value & 0x7f) | 0x80)
        value >>= 7

    out[-1] &= 0x7f  # Set the MSB of the last byte to 0
    return out


def from_varint(stream: IO[bytes]) -> int:
    out = 0
    shift = 0
    while True:
        value = stream.read(1)[0]
        out |= (value & 0x7f) << shift
        if (value & 0x80) == 0:
            return out

        shift += 7


def as_signed_varint(value: int) -> bytes:
    return as_varint(zigzag_encode(value))


def from_signed_varint(stream: IO[bytes]) -> int:
    return zigzag_decode(from_varint(stream))


def as_null_signed_varint(value: int | None) -> bytes:
    # Maps None to 0, shift normal zigzag values by +1
    zigzag_value = 0 if value is None else zigzag_encode(value) + 1
    return as_varint(zigzag_value)


def from_null_signed_varint(stream: IO[bytes]) -> int | None:
    zigzag_value = from_varint(stream)
    if zigzag_value == 0:
        return None
    return zigzag_decode(zigzag_value) - 1


def get_varint_bytes(stream: IO[bytes]) -> bytes:
    i = 1
    while stream.read(1)[0] & 0x80:
        i += 1

    stream.seek(-i, 1)
    return stream.read(i)


def as_null_string(value: str | None) -> bytes:
    if value is None:
        return as_varint(0)

    value_bytes = value.encode("utf-8")
    return as_varint(len(value_bytes) + 1) + value_bytes


def from_null_string(stream: IO[bytes]) -> str | None:
    length = from_varint(stream) - 1
    if length == -1:
        return None
    if length == 0:
        return ""
    return stream.read(length).decode("utf-8")


def as_null_varint_array(array: list[int] | None) -> bytes:
    if array is None:
        return as_varint(0)

    varints = [as_signed_varint(value) for value in array]
    return b"".join([
        as_varint(sum(len(varint) for varint in varints) + 1),
        *varints,
    ])


def from_null_varint_array(stream: IO[bytes]) -> list[int] | None:
    size = from_varint(stream) - 1
    if size == -1:
        return None

    out: list[int] = []
    end = stream.tell() + size
    while stream.tell() < end:
        out.append(from_varint(stream))


def read_bytes(stream: IO[bytes], size_start: int = 0) -> bytes:
    start = stream.tell()
    size = from_varint(stream) - size_start
    total_size = stream.tell() - start + max(size, 0)
    stream.seek(start)
    return stream.read(total_size)


def add_size_header(*chunks: bytes) -> list[bytes]:
    return [as_varint(sum(len(chunk) for chunk in chunks)), *chunks]
