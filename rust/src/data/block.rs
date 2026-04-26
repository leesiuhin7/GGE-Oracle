use std::io::{Cursor, Read, Seek, Write};

use super::interface::Interface;
use super::structures;
use crate::data::primitives::{
    DecodeStringError, DecodeVarintError, encode_optional_string, encode_varint_u64,
};
use crate::types::Document;

#[allow(dead_code)]
#[derive(Debug)]
pub enum HeaderError {
    Io(std::io::Error),
    Varint(DecodeVarintError),
    String(DecodeStringError),
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum UpdateError {
    Header(HeaderError),
    Rle(structures::rle::Error),
    Delta(structures::delta::Error),
    DeltaRle(structures::delta_rle::Error),
    Io(std::io::Error),
}

impl From<HeaderError> for UpdateError {
    fn from(value: HeaderError) -> Self {
        Self::Header(value)
    }
}

impl From<structures::rle::Error> for UpdateError {
    fn from(value: structures::rle::Error) -> Self {
        Self::Rle(value)
    }
}

impl From<structures::delta::Error> for UpdateError {
    fn from(value: structures::delta::Error) -> Self {
        Self::Delta(value)
    }
}

impl From<structures::delta_rle::Error> for UpdateError {
    fn from(value: structures::delta_rle::Error) -> Self {
        Self::DeltaRle(value)
    }
}

impl From<std::io::Error> for UpdateError {
    fn from(value: std::io::Error) -> Self {
        UpdateError::Io(value)
    }
}

pub struct Block<'a, R: Read + Seek> {
    interface: Interface<'a, R>,
}

impl<'a, R: Read + Seek> Block<'a, R> {
    pub fn new(reader: &'a mut R) -> Self {
        Block {
            interface: Interface::new(reader),
        }
    }

    pub fn new_buffer() -> std::io::Result<Cursor<Vec<u8>>> {
        let mut cursor: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        // Headers
        // Using dummy values since they are only skipped
        cursor.write_all(&encode_varint_u64(0))?;
        cursor.write_all(&u32::to_be_bytes(0))?;
        cursor.write_all(&encode_optional_string(Some(String::new())))?;
        // Timestamp
        cursor.write_all(&structures::delta::empty())?;
        // Basic
        cursor.write_all(&structures::rle::empty())?;
        cursor.write_all(&structures::rle::empty())?;
        cursor.write_all(&structures::rle::empty())?;
        cursor.write_all(&structures::delta_rle::empty())?;
        cursor.write_all(&structures::rle::empty())?;
        cursor.write_all(&structures::rle::empty())?;
        cursor.write_all(&structures::delta_rle::empty())?;
        cursor.write_all(&structures::rle::empty())?;
        // Alliance
        cursor.write_all(&structures::rle::empty())?;
        cursor.write_all(&structures::rle::empty())?;
        cursor.write_all(&structures::rle::empty())?;
        cursor.write_all(&structures::rle::empty())?;
        // Castle timers
        cursor.write_all(&structures::delta_rle::empty())?;
        cursor.write_all(&structures::delta_rle::empty())?;
        // Location
        cursor.write_all(&structures::rle::empty())?;
        // Coat of arms
        cursor.write_all(&structures::rle::empty())?;
        // Factions
        cursor.write_all(&structures::rle::empty())?;
        cursor.write_all(&structures::rle::empty())?;
        cursor.write_all(&structures::delta_rle::empty())?;
        cursor.write_all(&structures::rle::empty())?;
        cursor.write_all(&structures::delta_rle::empty())?;
        cursor.write_all(&structures::rle::empty())?;
        cursor.write_all(&structures::rle::empty())?;

        cursor.rewind()?; // Read from begining
        Ok(cursor)
    }

    pub fn update(
        mut self,
        writer: &mut impl Write,
        document: Document,
    ) -> Result<(), UpdateError> {
        // Headers
        self.interface.get_size().map_err(HeaderError::Varint)?; // Skip size header
        self.interface
            .push_id(document.id)
            .map_err(HeaderError::Io)?;
        self.interface
            .push_server(document.server)
            .map_err(HeaderError::String)?;
        // Timestamp
        self.interface.push_timestamp(document.timestamp)?;
        // Basic
        self.interface.push_string(document.basic.name)?;
        self.interface.push_i64(document.basic.level)?;
        self.interface.push_i64(document.basic.legendary_level)?;
        self.interface.push_delta_i64(document.basic.might)?;
        self.interface.push_i64(document.basic.honor)?;
        self.interface.push_i64(document.basic.achievement)?;
        self.interface.push_delta_i64(document.basic.glory)?;
        self.interface.push_i64(document.basic.ruins)?;
        // Alliance
        self.interface.push_i64(document.alliance.id)?;
        self.interface.push_string(document.alliance.name)?;
        self.interface.push_i64(document.alliance.rank_id)?;
        self.interface.push_i64(document.alliance.searching)?;
        // Castle timers
        self.interface.push_timer(document.timers.protection_time)?;
        self.interface.push_timer(document.timers.relocate_time)?;
        // Locations
        let location_data = document.locations.map(|locations| {
            locations
                .iter()
                .flat_map(|location| {
                    [
                        location.kingdom_id,
                        location.id,
                        location.x,
                        location.y,
                        location.location_type,
                    ]
                    .into_iter()
                })
                .collect()
        });
        self.interface.push_vec(location_data)?;
        // Coat of arms
        let coat_of_arms_data = document.coat_of_arms.map(|coat_of_arms| {
            vec![
                coat_of_arms.bg_type,
                coat_of_arms.bg_color1,
                coat_of_arms.bg_color2,
                coat_of_arms.symbol_pos_type,
                coat_of_arms.symbol_type1,
                coat_of_arms.symbol_color1,
                coat_of_arms.symbol_type2,
                coat_of_arms.symbol_color2,
            ]
        });
        self.interface.push_vec(coat_of_arms_data)?;
        // Factions
        self.interface.push_i64(document.faction.faction_id)?;
        self.interface.push_i64(document.faction.title_id)?;
        self.interface
            .push_timer(document.faction.self_protection_time)?;
        self.interface
            .push_i64(document.faction.group_protection_status)?;
        self.interface
            .push_timer(document.faction.group_protection_time)?;
        self.interface.push_i64(document.faction.main_camp_id)?;
        self.interface.push_i64(document.faction.special_camp_id)?;

        let bytes = self.interface.finalize();
        writer.write_all(&bytes)?;
        Ok(())
    }
}
