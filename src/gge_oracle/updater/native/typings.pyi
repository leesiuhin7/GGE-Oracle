class BasicPlayer:
    def __init__(
        self,
        name: str | None,
        level: int | None,
        legendary_level: int | None,
        might: int | None,
        honor: int | None,
        achievement: int | None,
        glory: int | None,
        ruins: int | None,
    ) -> None: ...


class BasicAlliance:
    def __init__(
        self,
        id: int | None,
        name: str | None,
        rank_id: int | None,
        searching: int | None,
    ) -> None: ...


class CastleTimers:
    def __init__(
        self,
        protection_time: int | None,
        relocate_time: int | None,
    ) -> None: ...


class Location:
    def __init__(
        self,
        kingdom_id: int,
        id: int,
        x: int,
        y: int,
        location_type: int,
    ) -> None: ...


class CoatOfArms:
    def __init__(
        self,
        bg_type: int,
        bg_color1: int,
        bg_color2: int,
        symbol_pos_type: int,
        symbol_type1: int,
        symbol_color1: int,
        symbol_type2: int,
        symbol_color2: int,
    ) -> None: ...


class Faction:
    def __init__(
        self,
        faction_id: int | None,
        title_id: int | None,
        self_protection_time: int | None,
        group_protection_status: int | None,
        group_protection_time: int | None,
        main_camp_id: int | None,
        special_camp_id: int | None,
    ) -> None: ...


class Document:
    def __init__(
        self,
        id: int,
        server: str,
        timestamp: int,
        basic: BasicPlayer,
        alliance: BasicAlliance,
        timers: CastleTimers,
        locations: list[Location] | None,
        coat_of_arms: CoatOfArms | None,
        faction: Faction,
    ) -> None: ...
