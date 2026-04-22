// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod error;
mod services;
mod skill;
mod config;
mod database;
mod security;
mod platform;
mod queue;
mod dialog;
mod market;
mod plugin;
mod api;
mod cli;
mod utils;

use core::app::Application;
use error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("Starting Desktop Agent v0.1.0");

    // Create and run application
    let app = Application::new().await?;

    // Run Tauri
    app.run().await?;

    Ok(())
}
