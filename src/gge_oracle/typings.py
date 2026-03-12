from typing import Any, Required, TypedDict


class CoatOfArmsDict(TypedDict):
    BGT: int  # Background type
    BGC1: int  # Backgroup color 1
    BGC2: int  # Background color 2
    SPT: int  # Symbol pos type
    S1: int  # Symbol type 1
    SC1: int  # Symbol color 1
    S2: int  # Symbol type 1
    SC2: int  # Symbol color 2
    IS: int  # Is set


class AllianceCoatOfArmsDict(TypedDict):
    ACLI: int  # Layout ID
    ACCS: list[int]  # Color IDs


class AllianceDict(TypedDict):
    ACCA: AllianceCoatOfArmsDict  # Alliance coat of arms
    ACFB: dict[str, Any]  # Fallback coat of arms


class FactionDict(TypedDict):
    MC: int  # Main camp ID
    FID: int  # Faction ID
    TID: int  # Faction title ID
    NS: int  # Player (individual) protection time
    PMS: int  # Faction protection status
    PMT: int  # Remaining faction protection time
    SPC: int  # Special camp ID


class Player(TypedDict, total=False):
    OID: Required[int]  # Player ID
    DUM: bool  # Is dummy player
    N: str  # Player name
    E: CoatOfArmsDict
    L: int  # Level
    LL: int  # Legendary level
    H: int  # Honor
    AVP: int  # Achievement Points
    CF: int  # Glory points
    HF: int  # Highest ever glory points
    PRE: int  # Title prefix
    SUF: int  # Title suffix
    TOPX: int  # Top X (unknown???)
    MP: int  # Might points
    R: int  # In ruins (1: True)
    AID: int  # Alliance ID
    AR: int  # Alliance rank
    AN: str  # Alliance name
    aee: AllianceDict
    RPT: int  # Remaining peace time (protection mode duration)
    # (kingdomId, objectId, x, y, type)
    AP: list[
        tuple[int, int, int, int, int]  # Castle
        | list[tuple[int, int, int, int]]  # Unknown
    ]
    VP: list[tuple[int, int, int, int, int]]  # Villages
    SA: int  # Searching for alliace
    VF: int  # Has VIP flag
    PF: int  # Has premium flag
    RRD: int  # Relocate remaining time
    TI: int  # Storm title
    FN: FactionDict  # Faction (probably Berimond)


class ServerResponseContent(TypedDict):
    FR: int
    IGH: int
    L: list[tuple[int, int, Player]]
    LID: int
    LR: int
    LT: int
    SV: str


class Location(TypedDict):
    kingdom_id: int
    id: int
    x: int
    y: int
    type: int


class BasicPlayer(TypedDict):
    name: str | None
    level: int | None
    legendary_level: int | None
    might: int | None
    honor: int | None
    achievement: int | None
    glory: int | None
    ruins: int | None


class BasicAlliance(TypedDict):
    id: int | None
    name: str | None
    rank_id: int | None
    searching: int | None


class CastleTimers(TypedDict):
    protection_time: int | None
    relocate_time: int | None


class CoatOfArms(TypedDict):
    bg_type: int
    bg_color1: int
    bg_color2: int
    symbol_pos_type: int
    symbol_type1: int
    symbol_color1: int
    symbol_type2: int
    symbol_color2: int


class Faction(TypedDict):
    faction_id: int | None
    title_id: int | None
    self_protection_time: int | None
    group_protection_status: int | None
    group_protection_time: int | None
    main_camp_id: int | None
    special_camp_id: int | None


class PlayerDocument(TypedDict):
    id: int
    server: str
    timestamp: int
    basic: BasicPlayer
    alliance: BasicAlliance
    timers: CastleTimers
    locations: list[Location] | None
    coat_of_arms: CoatOfArms | None
    faction: Faction
