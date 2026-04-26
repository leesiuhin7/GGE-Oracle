use pyo3::prelude::*;

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct BasicPlayer {
    pub name: Option<String>,
    pub level: Option<i64>,
    pub legendary_level: Option<i64>,
    pub might: Option<i64>,
    pub honor: Option<i64>,
    pub achievement: Option<i64>,
    pub glory: Option<i64>,
    pub ruins: Option<i64>,
}

#[pymethods]
impl BasicPlayer {
    #[allow(clippy::too_many_arguments)] // Acts as a dataclass for python
    #[new]
    fn new(
        name: Option<String>,
        level: Option<i64>,
        legendary_level: Option<i64>,
        might: Option<i64>,
        honor: Option<i64>,
        achievement: Option<i64>,
        glory: Option<i64>,
        ruins: Option<i64>,
    ) -> Self {
        BasicPlayer {
            name,
            level,
            legendary_level,
            might,
            honor,
            achievement,
            glory,
            ruins,
        }
    }
}

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct BasicAlliance {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub rank_id: Option<i64>,
    pub searching: Option<i64>,
}

#[pymethods]
impl BasicAlliance {
    #[new]
    fn new(
        id: Option<i64>,
        name: Option<String>,
        rank_id: Option<i64>,
        searching: Option<i64>,
    ) -> Self {
        BasicAlliance {
            id,
            name,
            rank_id,
            searching,
        }
    }
}

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct CastleTimers {
    pub protection_time: Option<i64>,
    pub relocate_time: Option<i64>,
}
#[pymethods]
impl CastleTimers {
    #[new]
    fn new(protection_time: Option<i64>, relocate_time: Option<i64>) -> Self {
        CastleTimers {
            protection_time,
            relocate_time,
        }
    }
}

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct Location {
    pub kingdom_id: i64,
    pub id: i64,
    pub x: i64,
    pub y: i64,
    #[allow(clippy::struct_field_names)]
    pub location_type: i64,
}
#[pymethods]
impl Location {
    #[new]
    fn new(kingdom_id: i64, id: i64, x: i64, y: i64, location_type: i64) -> Self {
        Location {
            kingdom_id,
            id,
            x,
            y,
            location_type,
        }
    }
}

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct CoatOfArms {
    pub bg_type: i64,
    pub bg_color1: i64,
    pub bg_color2: i64,
    pub symbol_pos_type: i64,
    pub symbol_type1: i64,
    pub symbol_color1: i64,
    pub symbol_type2: i64,
    pub symbol_color2: i64,
}
#[pymethods]
impl CoatOfArms {
    #[allow(clippy::too_many_arguments)] // Acts as a dataclass for python
    #[new]
    fn new(
        bg_type: i64,
        bg_color1: i64,
        bg_color2: i64,
        symbol_pos_type: i64,
        symbol_type1: i64,
        symbol_color1: i64,
        symbol_type2: i64,
        symbol_color2: i64,
    ) -> Self {
        CoatOfArms {
            bg_type,
            bg_color1,
            bg_color2,
            symbol_pos_type,
            symbol_type1,
            symbol_color1,
            symbol_type2,
            symbol_color2,
        }
    }
}

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct Faction {
    #[allow(clippy::struct_field_names)]
    pub faction_id: Option<i64>,
    pub title_id: Option<i64>,
    pub self_protection_time: Option<i64>,
    pub group_protection_status: Option<i64>,
    pub group_protection_time: Option<i64>,
    pub main_camp_id: Option<i64>,
    pub special_camp_id: Option<i64>,
}
#[pymethods]
impl Faction {
    #[new]
    fn new(
        faction_id: Option<i64>,
        title_id: Option<i64>,
        self_protection_time: Option<i64>,
        group_protection_status: Option<i64>,
        group_protection_time: Option<i64>,
        main_camp_id: Option<i64>,
        special_camp_id: Option<i64>,
    ) -> Self {
        Faction {
            faction_id,
            title_id,
            self_protection_time,
            group_protection_status,
            group_protection_time,
            main_camp_id,
            special_camp_id,
        }
    }
}

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct Document {
    pub id: u32,
    pub server: String,
    pub timestamp: i32,
    pub basic: BasicPlayer,
    pub alliance: BasicAlliance,
    pub timers: CastleTimers,
    pub locations: Option<Vec<Location>>,
    pub coat_of_arms: Option<CoatOfArms>,
    pub faction: Faction,
}

#[pymethods]
impl Document {
    #[allow(clippy::too_many_arguments)] // Acts as a dataclass for python
    #[new]
    fn new(
        id: u32,
        server: String,
        timestamp: i32,
        basic: BasicPlayer,
        alliance: BasicAlliance,
        timers: CastleTimers,
        locations: Option<Vec<Location>>,
        coat_of_arms: Option<CoatOfArms>,
        faction: Faction,
    ) -> Self {
        Document {
            id,
            server,
            timestamp,
            basic,
            alliance,
            timers,
            locations,
            coat_of_arms,
            faction,
        }
    }
}
