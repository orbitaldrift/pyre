use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Serialize, Deserialize)]
pub struct DbCard {
    pub id: Uuid,
    pub name: String,
    pub lang: String,

    pub released_at: chrono::NaiveDate,
    pub scryfall_uri: String,

    pub layout_id: i32,
    /// Pulled in from related
    pub layout: Option<String>,

    pub image_status: String,

    /// Pulled in from related
    pub card_faces: Option<Vec<DbFace>>,

    /// Pulled in from many-to-many
    pub color_identities: Option<Vec<String>>,
    /// Pulled in from many-to-many
    pub keywords: Option<Vec<String>>,
    /// Pulled in from many-to-many
    pub finishes: Option<Vec<String>>,

    pub legality_standard: bool,
    pub legality_future: bool,
    pub legality_historic: bool,
    pub legality_timeless: bool,
    pub legality_gladiator: bool,
    pub legality_pioneer: bool,
    pub legality_explorer: bool,
    pub legality_modern: bool,
    pub legality_legacy: bool,
    pub legality_pauper: bool,
    pub legality_vintage: bool,
    pub legality_penny: bool,
    pub legality_commander: bool,
    pub legality_oathbreaker: bool,
    pub legality_standardbrawl: bool,
    pub legality_brawl: bool,
    pub legality_alchemy: bool,
    pub legality_paupercommander: bool,
    pub legality_duel: bool,
    pub legality_oldschool: bool,
    pub legality_premodern: bool,
    pub legality_predh: bool,

    pub foil: bool,
    pub nonfoil: bool,
    pub oversized: bool,
    pub rarity: String,

    pub artist: String,

    pub set_id: Uuid,
    /// Pulled in from related
    pub set: Option<DbSet>,

    pub price_usd: Option<f32>,
    pub price_usd_foil: Option<f32>,
    pub price_usd_etched: Option<f32>,

    pub edhrec_uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DbFace {
    pub id: Uuid,
    pub card_id: Uuid,
    pub name: String,

    pub mana_cost: String,
    pub cmc: f32,

    /// Full-text search support
    pub type_line: String,

    /// Full-text search support
    pub oracle_text: String,

    /// Many cards don't have flavor text
    pub flavor_text: Option<String>,

    /// Pulled in from many-to-many
    pub colors: Option<Vec<String>>,

    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub loyalty: Option<i32>,

    pub image_small: String,
    pub image_normal: String,
    pub image_large: String,
    pub image_png: String,
    pub image_art_crop: String,
    pub image_border_crop: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DbSet {
    pub id: Uuid,
    pub code: String,
    pub set_type: String,
    pub card_count: i32,
    pub scryfall_uri: String,
}
