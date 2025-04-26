use serde::{Deserialize, Serialize};

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Serialize, Deserialize)]
pub struct ScryfallCard {
    /// Unique identifier for the card object
    pub id: String,
    /// Unique identifier for the card, if null see `card_faces`
    pub oracle_id: Option<String>,
    pub name: String,
    /// e.g. "en"
    pub lang: String,
    /// ISO Date format: YYYY-MM-DD
    pub released_at: chrono::NaiveDate,
    pub scryfall_uri: String,
    pub layout: String,
    pub image_status: String,
    /// Could be null for some layouts, see `card_faces`
    pub image_uris: Option<ImageUris>,
    /// Could be null for some layouts, see `card_faces`
    pub mana_cost: Option<String>,
    /// Converted mana cost, see `card_faces` (layout: Reversible)
    pub cmc: Option<f64>,
    /// Could be null for some layouts, see `card_faces`
    pub type_line: Option<String>,
    /// e.g. "Creature — Human Wizard", see `card_faces`
    pub oracle_text: Option<String>,
    /// Could be null for some layouts, see `card_faces`
    pub colors: Option<Vec<String>>,
    pub color_identity: Vec<String>,
    pub keywords: Vec<String>,
    pub legalities: Legalities,
    pub foil: bool,
    pub nonfoil: bool,
    pub finishes: Vec<String>,
    pub oversized: bool,
    pub promo: bool,
    pub reprint: bool,
    pub variation: bool,
    /// Set code
    pub set: String,
    pub set_id: String,
    pub set_name: String,
    pub set_type: String,
    pub scryfall_set_uri: String,
    pub collector_number: String,
    pub digital: bool,
    pub rarity: String,
    /// Added lore text, could be null for some layouts
    pub flavor_text: Option<String>,
    pub artist: String,
    pub booster: bool,
    pub prices: Prices,
    pub related_uris: RelatedUris,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageUris {
    pub small: String,
    pub normal: String,
    pub large: String,
    pub png: String,
    pub art_crop: String,
    pub border_crop: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Legalities {
    pub standard: String,
    pub future: String,
    pub historic: String,
    pub timeless: String,
    pub gladiator: String,
    pub pioneer: String,
    pub explorer: String,
    pub modern: String,
    pub legacy: String,
    pub pauper: String,
    pub vintage: String,
    pub penny: String,
    pub commander: String,
    pub oathbreaker: String,
    pub standardbrawl: String,
    pub brawl: String,
    pub alchemy: String,
    pub paupercommander: String,
    pub duel: String,
    pub oldschool: String,
    pub premodern: String,
    pub predh: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Prices {
    pub usd: Option<String>,
    pub usd_foil: Option<String>,
    pub usd_etched: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RelatedUris {
    /// Not always available
    pub edhrec: Option<String>,
}

#[cfg(test)]
mod tests {
    use tracing::info;

    use super::*;

    #[tokio::test]
    async fn test_scryfall_card_deserialization() {
        let _t = pyre_telemetry::Telemetry::default().init_scoped();

        let blob_json = tokio::fs::read("default-cards.json")
            .await
            .expect("Failed to read file");

        let data: Vec<ScryfallCard> = serde_json::from_slice(&blob_json).unwrap();

        let serialized = serde_json::to_string_pretty(&data[0]).unwrap();
        info!(data = %serialized, "serialized ScryfallCard");
    }
}
