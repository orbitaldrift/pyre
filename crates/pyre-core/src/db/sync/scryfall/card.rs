use serde::{Deserialize, Serialize};

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Serialize, Deserialize)]
pub struct Card {
    /// Unique identifier for the card object
    pub id: String,
    pub name: String,
    /// e.g. "en"
    pub lang: String,
    /// ISO Date format: YYYY-MM-DD
    pub released_at: chrono::NaiveDate,
    pub scryfall_uri: String,
    pub layout: String,
    pub image_status: String,

    /// Unique identifier for the card, if null see `card_faces`
    pub oracle_id: Option<String>,
    /// Could be null for some layouts, see `card_faces`
    pub image_uris: Option<ImageUris>,
    /// Could be null for some layouts, see `card_faces`
    pub mana_cost: Option<String>,
    /// Converted mana cost, see `card_faces`
    pub cmc: Option<f64>,
    /// e.g. "Legendary Creature - Dragon", see `card_faces`
    pub type_line: Option<String>,
    /// e.g. "Card does x when y", see `card_faces`
    pub oracle_text: Option<String>,
    /// Could be null for some layouts, see `card_faces`
    pub colors: Option<Vec<String>>,
    /// Added lore text, could be null for some layouts
    pub flavor_text: Option<String>,

    #[serde(rename = "card_faces")]
    pub faces: Option<Vec<CardFace>>,

    /// Loyalty, power and toughness depending on the card type
    pub loyalty: Option<String>,
    pub power: Option<String>,
    pub toughness: Option<String>,

    pub color_identity: Vec<String>,
    pub keywords: Vec<String>,
    pub legalities: Legalities,

    pub finishes: Vec<String>,

    pub foil: bool,
    pub nonfoil: bool,

    pub oversized: bool,

    /// Set code
    pub set: String,
    pub set_id: String,
    pub set_name: String,
    pub set_type: String,
    pub scryfall_set_uri: String,

    pub rarity: String,
    pub artist: String,
    pub prices: Prices,

    pub related_uris: RelatedUris,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CardFace {
    pub name: String,

    /// e.g. "Card does x when y"
    pub oracle_text: String,

    /// Mana cost using symbology
    pub mana_cost: String,

    /// e.g. "Legendary Creature - Dragon", no type lines for some tokens/cards
    pub type_line: Option<String>,

    /// Only present if the card has independent cards as its faces
    pub oracle_id: Option<String>,

    /// Converted mana cost, only in layout Reversible
    pub cmc: Option<f64>,

    /// Only double faced or reversible
    pub colors: Option<Vec<String>>,

    /// Added lore text, could be null for some layouts
    pub flavor_text: Option<String>,

    /// If its null, the ones from the og card object should be put here
    pub image_uris: Option<ImageUris>,

    /// Loyalty, power and toughness depending on the face type
    pub loyalty: Option<String>,
    pub power: Option<String>,
    pub toughness: Option<String>,
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

        let data: Vec<Card> = serde_json::from_slice(&blob_json).unwrap();

        // Find unique sets
        let sets: Vec<String> = data
            .iter()
            .map(|card| card.set.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        info!(size = ?sets.len(), "unique sets");

        // Find unique finishes
        let finishes: Vec<String> = data
            .iter()
            .flat_map(|card| card.finishes.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        info!(size = ?finishes.len(), "unique finishes");

        // Find unique layouts
        let layouts: Vec<String> = data
            .iter()
            .map(|card| card.layout.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        info!(layouts = ?layouts, size = ?layouts.len(), "unique layouts");

        // Find unique keywords
        let keywords: Vec<String> = data
            .iter()
            .flat_map(|card| card.keywords.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        info!(size = ?keywords.len(), "unique keywords");

        let serialized = serde_json::to_string_pretty(&data[0]).unwrap();
        info!(data = %serialized, "serialized ScryfallCard");
    }
}
