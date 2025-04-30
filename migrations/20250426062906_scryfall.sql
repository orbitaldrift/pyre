CREATE SCHEMA scryfall

-- Layout types table
CREATE TABLE layouts (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) UNIQUE NOT NULL
)

-- Sets table
CREATE TABLE sets (
    id UUID PRIMARY KEY,
    code VARCHAR(10) UNIQUE NOT NULL,
    name VARCHAR(100) NOT NULL,
    set_type VARCHAR(50) NOT NULL,
    scryfall_uri TEXT NOT NULL
)

-- Colors table (for lookup)
CREATE TABLE colors (
    id SERIAL PRIMARY KEY,
    code CHAR(1) UNIQUE NOT NULL,
    name VARCHAR(20) UNIQUE NOT NULL
)

-- Keywords table
CREATE TABLE keywords (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) UNIQUE NOT NULL
)

-- Finishes table
CREATE TABLE finishes (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) UNIQUE NOT NULL
)

-- Main cards table
CREATE TABLE cards (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    lang VARCHAR(10) NOT NULL,

    released_at DATE NOT NULL,
    scryfall_uri TEXT NOT NULL,
    layout_id INTEGER REFERENCES layouts(id) NOT NULL,
    image_status VARCHAR(50),

    legality_standard BOOLEAN DEFAULT FALSE,
    legality_future BOOLEAN DEFAULT FALSE,
    legality_historic BOOLEAN DEFAULT FALSE,
    legality_timeless BOOLEAN DEFAULT FALSE,
    legality_gladiator BOOLEAN DEFAULT FALSE,
    legality_pioneer BOOLEAN DEFAULT FALSE,
    legality_explorer BOOLEAN DEFAULT FALSE,
    legality_modern BOOLEAN DEFAULT FALSE,
    legality_legacy BOOLEAN DEFAULT FALSE,
    legality_pauper BOOLEAN DEFAULT FALSE,
    legality_vintage BOOLEAN DEFAULT FALSE,
    legality_penny BOOLEAN DEFAULT FALSE,
    legality_commander BOOLEAN DEFAULT FALSE,
    legality_oathbreaker BOOLEAN DEFAULT FALSE,
    legality_standardbrawl BOOLEAN DEFAULT FALSE,
    legality_brawl BOOLEAN DEFAULT FALSE,
    legality_alchemy BOOLEAN DEFAULT FALSE,
    legality_paupercommander BOOLEAN DEFAULT FALSE,
    legality_duel BOOLEAN DEFAULT FALSE,
    legality_oldschool BOOLEAN DEFAULT FALSE,
    legality_premodern BOOLEAN DEFAULT FALSE,
    legality_predh BOOLEAN DEFAULT FALSE,

    foil BOOLEAN DEFAULT FALSE,
    nonfoil BOOLEAN DEFAULT FALSE,
    oversized BOOLEAN DEFAULT FALSE,
    rarity VARCHAR(20) NOT NULL,

    artist VARCHAR(100),

    set_id UUID REFERENCES sets(id) NOT NULL,

    price_usd REAL,
    price_usd_foil REAL,
    price_usd_etched REAL,
    
    edhrec_uri TEXT
)

-- Junction table for card color identity (many-to-many)
CREATE TABLE card_color_identity (
    card_id UUID REFERENCES cards(id) ON DELETE CASCADE,
    color_id INTEGER REFERENCES colors(id) ON DELETE CASCADE,
    PRIMARY KEY (card_id, color_id)
)

-- Junction table for card keywords (many-to-many)
CREATE TABLE card_keywords (
    card_id UUID REFERENCES cards(id) ON DELETE CASCADE,
    keyword_id INTEGER REFERENCES keywords(id) ON DELETE CASCADE,
    PRIMARY KEY (card_id, keyword_id)
)

-- Junction table for card finishes (many-to-many)
CREATE TABLE card_finishes (
    card_id UUID REFERENCES cards(id) ON DELETE CASCADE,
    finish_id INTEGER REFERENCES finishes(id) ON DELETE CASCADE,
    PRIMARY KEY (card_id, finish_id)
)

CREATE INDEX idx_cards_set_id ON cards(set_id)
CREATE INDEX idx_cards_rarity ON cards(rarity)
CREATE INDEX idx_cards_price_usd ON cards(price_usd)

-- Card Faces
CREATE TABLE card_faces (
    id SERIAL PRIMARY KEY,
    card_id UUID REFERENCES cards(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,

    mana_cost VARCHAR(100),
    cmc REAL,
    type_line TEXT,

    oracle_text TEXT,
    flavor_text TEXT,

    power INTEGER,
    toughness INTEGER,
    loyalty INTEGER,
    
    image_small TEXT,
    image_normal TEXT,
    image_large TEXT,
    image_png TEXT,
    image_art_crop TEXT,
    image_border_crop TEXT
)

-- Junction table for card colors (many-to-many)
CREATE TABLE card_colors (
    card_id INTEGER REFERENCES card_faces(id) ON DELETE CASCADE,
    color_id INTEGER REFERENCES colors(id) ON DELETE CASCADE,
    PRIMARY KEY (card_id, color_id)
)

-- Create indexes for common search patterns
CREATE INDEX idx_cards_name ON card_faces(name)
CREATE INDEX idx_cards_cmc ON card_faces(cmc)

-- Create GIN indexes for text search
CREATE INDEX idx_cards_oracle_text_gin ON card_faces USING gin(to_tsvector('english', oracle_text))
CREATE INDEX idx_cards_type_line_gin ON card_faces USING gin(to_tsvector('english', type_line))

CREATE TABLE symbols (
    id SERIAL PRIMARY KEY,
    symbol VARCHAR(16) UNIQUE NOT NULL,
    svg_uri TEXT UNIQUE NOT NULL,
    description TEXT NOT NULL,
    cmc REAL
)

;

-- Functions: TODO

-- Populate known colors
INSERT INTO scryfall.colors (code, name) VALUES
('W', 'White'),
('U', 'Blue'),
('B', 'Black'),
('R', 'Red'),
('G', 'Green');