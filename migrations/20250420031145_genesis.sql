-- Add migration script here

-- Create enum type for provider kinds
CREATE TYPE provider_kind AS ENUM ('discord');

-- Create users table
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    avatar VARCHAR(255) NOT NULL,
    name VARCHAR(32) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    auth_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create providers table
CREATE TABLE providers (
    id SERIAL,
    user_id INTEGER NOT NULL,
    external_id VARCHAR(255) NOT NULL,
    kind provider_kind NOT NULL,
    username VARCHAR(32) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, user_id, external_id, kind),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Add indexes
CREATE INDEX idx_users_email ON users(email);
