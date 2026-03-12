import json
import logging

from gge_oracle.typings import (
    BasicAlliance,
    BasicPlayer,
    CastleTimers,
    CoatOfArms,
    Faction,
    Location,
    Player,
    PlayerDocument,
    ServerResponseContent,
)

logger = logging.getLogger(__name__)


def unpack_smaple_size(msg: str) -> int | None:
    try:
        data = _extract_data(msg)
        json_data: ServerResponseContent = json.loads(data)
        return json_data["LR"]
    except Exception:
        return None


def unpack_player_info(
    msg: str,
    server: str,
    timestamp: int,
) -> list[PlayerDocument]:
    try:
        data = _extract_data(msg)
        json_data: ServerResponseContent = json.loads(data)
        items = json_data["L"]
        return [
            _unpack_player(player, server, timestamp)
            for _, _, player in items
        ]
    except Exception:
        return []


def _extract_data(msg: str) -> str:
    if not msg.startswith("%xt%hgh"):
        raise ValueError

    return msg[12:-1]


def _unpack_player(
    player: Player,
    server: str,
    timestamp: int,
) -> PlayerDocument:
    """
    Extract player information from the inputs.

    :param player: A JSON deserialized object containing player information
    :type player: Player
    :param server: The game server where the player is registered in
    :type server: str
    :param timestamp: The timestamp of when the player information is
        received
    :type timestamp: int
    :return: The extracted player information
    :rtype: PlayerDocument
    """
    basic_player = BasicPlayer(
        name=player.get("N"),
        level=player.get("L"),
        legendary_level=player.get("LL"),
        might=player.get("MP"),
        honor=player.get("H"),
        achievement=player.get("AVP"),
        glory=player.get("CF"),
        ruins=player.get("R"),
    )

    basic_alliance = BasicAlliance(
        id=player.get("AID"),
        name=player.get("AN"),
        rank_id=player.get("AR"),
        searching=player.get("SA"),
    )

    timers = CastleTimers(
        protection_time=player.get("RPT"),
        relocate_time=player.get("RRD"),
    )

    locations = [
        Location(
            kingdom_id=location[0],
            id=location[1],
            x=location[2],
            y=location[3],
            type=location[4],
        )
        for location in player.get("AP", [])
        if (
            len(location) == 5
            and isinstance(location[0], int)
            and isinstance(location[1], int)
            and isinstance(location[2], int)
            and isinstance(location[3], int)
            and isinstance(location[4], int)
        )
    ] if "AP" in player else None

    coat_of_arms = CoatOfArms(
        bg_type=player["E"]["BGT"],
        bg_color1=player["E"]["BGC1"],
        bg_color2=player["E"]["BGC2"],
        symbol_pos_type=player["E"]["SPT"],
        symbol_type1=player["E"]["S1"],
        symbol_color1=player["E"]["SC1"],
        symbol_type2=player["E"]["S2"],
        symbol_color2=player["E"]["SC2"],
    ) if "E" in player else None

    faction_dict = player.get("FN", {})
    faction = Faction(
        faction_id=faction_dict.get("FID"),
        title_id=faction_dict.get("TID"),
        self_protection_time=faction_dict.get("NS"),
        group_protection_status=faction_dict.get("PMS"),
        group_protection_time=faction_dict.get("PMT"),
        main_camp_id=faction_dict.get("MC"),
        special_camp_id=faction_dict.get("SPC"),
    )

    return {
        "id": player["OID"],
        "server": server,
        "timestamp": timestamp,
        "basic": basic_player,
        "alliance": basic_alliance,
        "timers": timers,
        "locations": locations,
        "coat_of_arms": coat_of_arms,
        "faction": faction,
    }
