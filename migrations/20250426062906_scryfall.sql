CREATE SCHEMA scryfall;

-- Layout types table
CREATE TABLE scryfall.layouts (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) UNIQUE NOT NULL
);

-- Card Types table
CREATE TABLE scryfall.card_types (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) UNIQUE NOT NULL
);

-- Sets table
CREATE TABLE scryfall.sets (
    id UUID PRIMARY KEY,
    code VARCHAR(10) UNIQUE NOT NULL,
    name VARCHAR(100) NOT NULL,
    set_type VARCHAR(50) NOT NULL,
    scryfall_uri TEXT NOT NULL,
    released_at DATE
);

-- Colors table (for lookup)
CREATE TABLE scryfall.colors (
    id SERIAL PRIMARY KEY,
    code CHAR(1) UNIQUE NOT NULL,
    name VARCHAR(20) UNIQUE NOT NULL
);

-- Keywords table
CREATE TABLE scryfall.keywords (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) UNIQUE NOT NULL
);

-- Finishes table
CREATE TABLE scryfall.finishes (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) UNIQUE NOT NULL
);

-- Main cards table
CREATE TABLE scryfall.cards (
    id UUID PRIMARY KEY,
    oracle_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    lang VARCHAR(10) NOT NULL,
    released_at DATE NOT NULL,
    scryfall_uri TEXT NOT NULL,
    layout_id INTEGER REFERENCES scryfall.layouts(id) NOT NULL,
    image_status VARCHAR(50),
    image_small TEXT,
    image_normal TEXT,
    image_large TEXT,
    image_png TEXT,
    image_art_crop TEXT,
    image_border_crop TEXT,
    mana_cost VARCHAR(100),
    cmc DECIMAL(10, 2) NOT NULL,
    type_line TEXT,
    oracle_text TEXT,
    legality_standard VARCHAR(20) DEFAULT 'not_legal',
    legality_future VARCHAR(20) DEFAULT 'not_legal',
    legality_historic VARCHAR(20) DEFAULT 'not_legal',
    legality_timeless VARCHAR(20) DEFAULT 'not_legal',
    legality_gladiator VARCHAR(20) DEFAULT 'not_legal',
    legality_pioneer VARCHAR(20) DEFAULT 'not_legal',
    legality_explorer VARCHAR(20) DEFAULT 'not_legal',
    legality_modern VARCHAR(20) DEFAULT 'not_legal',
    legality_legacy VARCHAR(20) DEFAULT 'not_legal',
    legality_pauper VARCHAR(20) DEFAULT 'not_legal',
    legality_vintage VARCHAR(20) DEFAULT 'not_legal',
    legality_penny VARCHAR(20) DEFAULT 'not_legal',
    legality_commander VARCHAR(20) DEFAULT 'not_legal',
    legality_oathbreaker VARCHAR(20) DEFAULT 'not_legal',
    legality_standardbrawl VARCHAR(20) DEFAULT 'not_legal',
    legality_brawl VARCHAR(20) DEFAULT 'not_legal',
    legality_alchemy VARCHAR(20) DEFAULT 'not_legal',
    legality_paupercommander VARCHAR(20) DEFAULT 'not_legal',
    legality_duel VARCHAR(20) DEFAULT 'not_legal',
    legality_oldschool VARCHAR(20) DEFAULT 'not_legal',
    legality_premodern VARCHAR(20) DEFAULT 'not_legal',
    legality_predh VARCHAR(20) DEFAULT 'not_legal',
    foil BOOLEAN DEFAULT FALSE,
    nonfoil BOOLEAN DEFAULT FALSE,
    oversized BOOLEAN DEFAULT FALSE,
    promo BOOLEAN DEFAULT FALSE,
    reprint BOOLEAN DEFAULT FALSE,
    variation BOOLEAN DEFAULT FALSE,
    set_id UUID REFERENCES scryfall.sets(id) NOT NULL,
    collector_number VARCHAR(20) NOT NULL,
    digital BOOLEAN DEFAULT FALSE,
    rarity VARCHAR(20) NOT NULL,
    flavor_text TEXT,
    artist VARCHAR(100),
    booster BOOLEAN DEFAULT FALSE,
    price_usd NUMERIC,
    price_usd_foil NUMERIC,
    price_usd_etched NUMERIC,
    edhrec_uri TEXT,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Junction table for card colors (many-to-many)
CREATE TABLE scryfall.card_colors (
    card_id UUID REFERENCES scryfall.cards(id) ON DELETE CASCADE,
    color_id INTEGER REFERENCES scryfall.colors(id) ON DELETE CASCADE,
    PRIMARY KEY (card_id, color_id)
);

-- Junction table for card color identity (many-to-many)
CREATE TABLE scryfall.card_color_identity (
    card_id UUID REFERENCES scryfall.cards(id) ON DELETE CASCADE,
    color_id INTEGER REFERENCES scryfall.colors(id) ON DELETE CASCADE,
    PRIMARY KEY (card_id, color_id)
);

-- Junction table for card keywords (many-to-many)
CREATE TABLE scryfall.card_keywords (
    card_id UUID REFERENCES scryfall.cards(id) ON DELETE CASCADE,
    keyword_id INTEGER REFERENCES scryfall.keywords(id) ON DELETE CASCADE,
    PRIMARY KEY (card_id, keyword_id)
);

-- Junction table for card finishes (many-to-many)
CREATE TABLE scryfall.card_finishes (
    card_id UUID REFERENCES scryfall.cards(id) ON DELETE CASCADE,
    finish_id INTEGER REFERENCES scryfall.finishes(id) ON DELETE CASCADE,
    PRIMARY KEY (card_id, finish_id)
);

-- Create indexes for common search patterns
CREATE INDEX idx_cards_name ON scryfall.cards(name);
CREATE INDEX idx_cards_set_id ON scryfall.cards(set_id);
CREATE INDEX idx_cards_cmc ON scryfall.cards(cmc);
CREATE INDEX idx_cards_rarity ON scryfall.cards(rarity);

-- Create GIN indexes for text search
CREATE INDEX idx_cards_oracle_text_gin ON scryfall.cards USING gin(to_tsvector('english', oracle_text));
CREATE INDEX idx_cards_type_line_gin ON scryfall.cards USING gin(to_tsvector('english', type_line));

-- Functions for getting cards by color combinations (simplified version)
CREATE OR REPLACE FUNCTION scryfall.get_cards_by_colors(color_codes CHAR[])
RETURNS SETOF UUID AS $$
BEGIN
    RETURN QUERY
    SELECT DISTINCT cc.card_id
    FROM card_colors cc
    JOIN colors c ON cc.color_id = c.id
    WHERE c.code = ANY(color_codes)
    GROUP BY cc.card_id
    HAVING COUNT(DISTINCT cc.color_id) = array_length(color_codes, 1);
END;
$$ LANGUAGE plpgsql;

-- Populate initial lookup tables
INSERT INTO scryfall.colors (code, name) VALUES
('W', 'White'),
('U', 'Blue'),
('B', 'Black'),
('R', 'Red'),
('G', 'Green');

INSERT INTO scryfall.layouts (name) VALUES
('normal'),
('split'),
('flip'),
('transform'),
('modal_dfc'),
('meld'),
('leveler'),
('class'),
('case'),
('saga'),
('adventure'),
('mutate'),
('prototype'),
('battle'),
('planar'),
('scheme'),
('vanguard'),
('token'),
('double_faced_token'),
('emblem'),
('augment'),
('host'),
('art_series'),
('reversible_card');

INSERT INTO scryfall.finishes (name) VALUES
('nonfoil'),
('foil'),
('etched'),
('glossy');
