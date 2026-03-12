import io
from typing import IO

from gge_oracle.typings import CoatOfArms, Location, PlayerDocument

from ._interface import (
    StreamInterface,
    new_delta_data,
    new_delta_RLE,
    new_normal_RLE,
)
from ._utils import as_null_string, as_varint


def unpack_locations(locations: list[Location] | None) -> list[int] | None:
    if locations is None:
        return None

    unpacked: list[int] = []
    for location in locations:
        unpacked.extend([
            location["kingdom_id"],
            location["id"],
            location["x"],
            location["y"],
            location["type"],
        ])
    return unpacked


def unpack_coat_of_arms(coat_of_arms: CoatOfArms | None) -> list[int] | None:
    if coat_of_arms is None:
        return None
    return [
        coat_of_arms["bg_type"],
        coat_of_arms["bg_color1"],
        coat_of_arms["bg_color2"],
        coat_of_arms["symbol_pos_type"],
        coat_of_arms["symbol_type1"],
        coat_of_arms["symbol_color1"],
        coat_of_arms["symbol_type2"],
        coat_of_arms["symbol_color2"],
    ]


class Updater:
    def __init__(self, current: IO[bytes], out: IO[bytes]) -> None:
        self._stream = current
        self._out_stream = out

        # {(id, server): (start pos, size)}
        self._index: dict[tuple[int, str], tuple[int, int]] = {}
        self._used_documents: set[tuple[int, str]] = set()

    def init(self) -> None:
        # Get end pos
        self._stream.seek(0, 2)
        end = self._stream.tell()
        self._stream.seek(0)  # Go back to the start

        interface = StreamInterface(self._stream)
        while self._stream.tell() < end:  # Repeat until EOF
            start = self._stream.tell()
            size = interface.get_size()
            next_pos = self._stream.tell() + size

            # Create index
            player_id = interface.get_id()
            server = interface.get_server()
            self._index.setdefault(
                (player_id, server),
                (start, next_pos - start),
            )

            self._stream.seek(next_pos)  # Move to next

    def update(self, document: PlayerDocument) -> None:
        key = (document["id"], document["server"])
        if key in self._used_documents:
            return  # Already updated
        self._used_documents.add(key)

        if key in self._index:
            start, _ = self._index[key]
            self._stream.seek(start)
            interface = StreamInterface(self._stream)
            # Move pass the header
            interface.get_size()
            interface.get_id()
            interface.get_server()

            self._update_player(document, interface)
        else:
            stream = io.BytesIO()
            interface = StreamInterface(stream)
            self._add_player_base(stream)  # Create new player slot
            stream.seek(0)
            self._update_player(document, interface)

    def finalize(self) -> None:
        for key, (start, size) in self._index.items():
            if key in self._used_documents:
                continue
            self._used_documents.add(key)

            # Copy data to output stream
            self._stream.seek(start)
            data = self._stream.read(size)
            self._out_stream.write(data)

    def _update_player(
        self,
        document: PlayerDocument,
        interface: StreamInterface,
    ) -> None:
        # Timestamp
        interface.update_timestamp(document["timestamp"])
        # Basic
        interface.update_string(document["basic"]["name"])
        interface.update_int(document["basic"]["level"])
        interface.update_int(document["basic"]["legendary_level"])
        interface.update_delta_data(document["basic"]["might"])
        interface.update_int(document["basic"]["honor"])
        interface.update_int(document["basic"]["achievement"])
        interface.update_delta_data(document["basic"]["glory"])
        interface.update_int(document["basic"]["ruins"])
        # Alliance
        interface.update_int(document["alliance"]["id"])
        interface.update_string(document["alliance"]["name"])
        interface.update_int(document["alliance"]["rank_id"])
        interface.update_int(document["alliance"]["searching"])
        # Castle timers
        interface.update_timer(document["timers"]["protection_time"])
        interface.update_timer(document["timers"]["relocate_time"])
        # Locations
        interface.update_varint_array(
            unpack_locations(document["locations"]),
        )
        # Coat of arms
        interface.update_varint_array(
            unpack_coat_of_arms(document["coat_of_arms"]),
        )
        # Factions
        interface.update_int(document["faction"]["faction_id"])
        interface.update_int(document["faction"]["title_id"])
        interface.update_timer(
            document["faction"]["self_protection_time"],
        )
        interface.update_int(
            document["faction"]["group_protection_status"],
        )
        interface.update_timer(
            document["faction"]["group_protection_time"],
        )
        interface.update_int(document["faction"]["main_camp_id"])
        interface.update_int(document["faction"]["special_camp_id"])

        id_bytes = int.to_bytes(document["id"], length=4)
        server_bytes = as_null_string(document["server"])
        data = interface.consume()

        size_bytes = as_varint(len(id_bytes) + len(server_bytes) + len(data))

        # Write to output
        self._out_stream.write(size_bytes)
        self._out_stream.write(id_bytes)
        self._out_stream.write(server_bytes)
        self._out_stream.write(data)

    def _add_player_base(self, stream: IO[bytes]) -> None:
        out: list[bytes] = []
        # Timestamp
        out.extend(new_delta_data())
        # Basic
        out.extend(new_normal_RLE())
        out.extend(new_normal_RLE())
        out.extend(new_normal_RLE())
        out.extend(new_delta_RLE())
        out.extend(new_normal_RLE())
        out.extend(new_normal_RLE())
        out.extend(new_delta_RLE())
        out.extend(new_normal_RLE())
        # Alliance
        out.extend(new_normal_RLE())
        out.extend(new_normal_RLE())
        out.extend(new_normal_RLE())
        out.extend(new_normal_RLE())
        # Castle timers
        out.extend(new_delta_RLE())
        out.extend(new_delta_RLE())
        # Location
        out.extend(new_normal_RLE())
        # Coat of arms
        out.extend(new_normal_RLE())
        # Factions
        out.extend(new_normal_RLE())
        out.extend(new_normal_RLE())
        out.extend(new_delta_RLE())
        out.extend(new_normal_RLE())
        out.extend(new_delta_RLE())
        out.extend(new_normal_RLE())
        out.extend(new_normal_RLE())

        for value in out:
            stream.write(value)
