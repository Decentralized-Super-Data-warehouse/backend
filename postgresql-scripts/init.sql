-- Drop the existing database if it exists and create it again
DROP DATABASE IF EXISTS testdb;
CREATE DATABASE testdb;

-- Connect to the newly created database
\c testdb;

-- Drop tables if they exist, then create them
DROP TABLE IF EXISTS project_atrtibute;
DROP TABLE IF EXISTS project;
DROP TABLE IF EXISTS account;
DROP TABLE IF EXISTS entity;
DROP TABLE IF EXISTS app_user;

-- Create the user table
CREATE TABLE app_user (
    id serial primary key not null,
    name varchar(64) not null,
    email varchar(128) unique not null,
    hashed_password varchar(128) not null,
    role varchar(32) not null,
    created_at timestamp with time zone default current_timestamp not null,
    updated_at timestamp with time zone default current_timestamp not null
);

-- Create the entity table
CREATE TABLE entity (
    id serial primary key not null,
    name varchar(128) not null,
    created_at timestamp with time zone default current_timestamp not null,
    updated_at timestamp with time zone default current_timestamp not null
);

-- Create the account table, with a foreign key to entity
CREATE TABLE account (
    id serial primary key not null,
    address varchar(128) unique not null,
    entity_id integer references entity(id) on delete cascade,
    created_at timestamp with time zone default current_timestamp not null,
    updated_at timestamp with time zone default current_timestamp not null

);

-- Base Project Table
CREATE TABLE project (
    id SERIAL PRIMARY KEY,
    name VARCHAR(64) NOT NULL,
    token VARCHAR(64) NOT NULL,
    category VARCHAR(128) NOT NULL,
    contract_address VARCHAR(128) REFERENCES account(address) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Project Attributes Table
CREATE TABLE project_attribute (
    id SERIAL PRIMARY KEY,
    project_id INTEGER REFERENCES project(id) ON DELETE CASCADE,
    key VARCHAR(64) NOT NULL,
    value TEXT,
    value_type VARCHAR(16) NOT NULL, -- To store the data type (e.g., "integer", "float", "string")
    UNIQUE (project_id, key)
);
