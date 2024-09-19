-- Drop the existing database if it exists and create it again
DROP DATABASE IF EXISTS testdb;
CREATE DATABASE testdb;

-- Connect to the newly created database
\c testdb;

-- Drop tables if they exist, then create them
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
    address varchar(64) unique not null,
    entity_id integer references entity(id) on delete cascade,
    created_at timestamp with time zone default current_timestamp not null,
    updated_at timestamp with time zone default current_timestamp not null

);
-- Create the project table, with a foreign key to account
CREATE TABLE project (
    id serial primary key not null,
    token varchar(64) not null,
    category varchar(128) not null,
    contract_address varchar(64) references account(address) on delete cascade,
    num_chains integer,
    core_developers integer,
    code_commits integer,
    total_value_locked float,
    created_at timestamp with time zone default current_timestamp not null,
    updated_at timestamp with time zone default current_timestamp not null
);
