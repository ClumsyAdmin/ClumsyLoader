use color_eyre::Result;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use reqwest::Client;
use serde::Deserialize;
use indicatif::{ProgressBar, ProgressStyle};
use futures_util::StreamExt;
use ratatui::{
    prelude::*,
    widgets::{List, ListItem, ListState, Paragraph},
    crossterm::{
        execute,
        event::{self, Event, KeyCode, KeyEventKind},
        terminal::{enable_raw_mode, disable_raw_mode, Clear, ClearType},
    },
};


#[derive(Debug, Deserialize)]
struct ServerListResponse {
    data: Vec<ServerData>,
}

#[derive(Debug, Deserialize)]
struct ServerData {
    attributes: Server,
}

#[derive(Debug, Deserialize)]
struct Server {
    identifier: String,
    uuid: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct BackupListResponse {
    data: Vec<BackupData>,
}

#[derive(Debug, Deserialize)]
struct BackupDownloadLinkResponse {
    attributes: Attributes,
}

#[derive(Debug, Deserialize)]
struct Attributes {
    url: String,
}

#[derive(Debug, Deserialize)]
struct BackupData {
    attributes: Backup,
}

#[derive(Debug, Deserialize)]
struct Backup {
    uuid: String,
    name: String,
    created_at: String,
    bytes: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = run().await {
        eprintln!("Error: {:?}", e);
        println!("Press Enter to exit...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
    }
    Ok(())
}


async fn run() -> Result<()> {
    color_eyre::install()?;
    
    println!("Enter your Bloom panel API key:");
    let mut bloom_api_key = String::new();
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    reader.read_line(&mut bloom_api_key).await?;
    let bloom_api_key = bloom_api_key.trim().to_string();

    println!("Enter your Bloom panel URL (or press Enter for default mc.bloom.host):");
    let mut bloom_panel_url = String::new();
    reader.read_line(&mut bloom_panel_url).await?;
    let bloom_panel_url = bloom_panel_url.trim();
    
    let bloom_panel_url = if bloom_panel_url.is_empty() {
        "https://mc.bloom.host".to_string()
    } else {
        format!("https://{}", bloom_panel_url)
    };

    execute!(std::io::stdout(), Clear(ClearType::All))?;

    let client = Client::builder()
        .user_agent("ClumsyLoader/0.1")
        .build()?;
    
    // Fetch servers
    let servers = fetch_servers(&client, &bloom_panel_url, &bloom_api_key).await?;
    if servers.is_empty() {
        println!("No servers found.");
        return Ok(());
    }

    // Select a server from the UI
    let selected_server = select_from_list("Servers:", &servers, display_server)?;
    //let selected_server = select_from_list("Select a server", &servers, |s| &s.name)?;
    let selected_server_uuid = &servers[selected_server].uuid;
    let selected_server_short_uuid = &servers[selected_server].identifier;

    // Fetch backups for the selected server
    let backups = fetch_backups(&client, &bloom_panel_url, &bloom_api_key, selected_server_uuid).await?;
    if backups.is_empty() {
        println!("No backups found for the selected server.");
        return Ok(());
    }

    execute!(std::io::stdout(), Clear(ClearType::All))?;

    // Select a backup from the UI (display name + date)
    let selected_backup = select_from_list("Backups:", &backups, display_backup)?;

    let backup_url = generate_backup_dl_link(&client, &bloom_panel_url, &bloom_api_key, selected_server_short_uuid, &backups[selected_backup].uuid).await?;

    download_backup(&client, &backup_url, &backups[selected_backup].uuid, backups[selected_backup].bytes).await?;

    Ok(())
}

async fn fetch_servers(client: &Client, url: &str, api_key: &str) -> Result<Vec<Server>> {
    let response = client.get(format!("{}/api/client", url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Accept", "application/json")
        .send()
        .await?
        .json::<ServerListResponse>()
        .await?;

    // Extract attributes (Server) from ServerData
    let servers: Vec<Server> = response.data.into_iter().map(|s| s.attributes).collect();
    Ok(servers)
}

async fn fetch_backups(client: &Client, url: &str, api_key: &str, server_uuid: &str) -> Result<Vec<Backup>> {
    let response = client.get(format!("{}/api/client/servers/{}/backups", url, server_uuid))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Accept", "application/json")
        .send()
        .await?
        .json::<BackupListResponse>()
        .await?;

    // Extract attributes (Backup) from BackupData
    let backups: Vec<Backup> = response.data.into_iter().map(|b| b.attributes).collect();
    Ok(backups)
}

async fn generate_backup_dl_link(client: &Client, url: &str, api_key: &str, server_uuid: &str, backup_uuid: &str) -> Result<String> {
    let response = client.get(format!("{}/api/client/servers/{}/backups/{}/download", url, server_uuid, backup_uuid))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Accept", "application/json")
        .send()
        .await?;

    let download_link_response: BackupDownloadLinkResponse = response.json().await?;
    let download_link = download_link_response.attributes.url;

    Ok(download_link)
}

async fn download_backup(client: &Client, url: &str, backup_uuid: &str, backup_bytes: u64) -> Result<()> {
    let response = client.get(url)
        .send()
        .await?;

    if response.headers().get("Content-Type").map_or(false, |v| v == "text/html") {
        let error_message = response.text().await?;
        return Err(color_eyre::eyre::eyre!("Failed to download backup: {}", error_message));
    }
    let mut downloaded = 0;
    let total_size = backup_bytes;

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})",
        )
        .unwrap()
        .progress_chars("#>-"),
    );
    pb.set_position(downloaded);
    pb.reset_eta();

    let backup_file_name = format!("{}.tar.gz", backup_uuid);
    let mut backup_file = File::create(backup_file_name).await?;
    
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
        backup_file.write_all(&chunk).await?;
    }

    pb.finish_with_message("Download complete");
    Ok(())
}

fn display_server(server: &Server) -> String {
    server.name.clone()
}

fn display_backup(backup: &Backup) -> String {
    format!("{} - {}", backup.name, backup.created_at)
}

fn select_from_list<Item, DisplayFn>(title: &str, items: &[Item], display_fn: DisplayFn) -> Result<usize>
where
    DisplayFn: Fn(&Item) -> String,
{
    // Enable raw mode for immediate keypress handling
    enable_raw_mode()?;

    // Initialize the terminal
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    // Initialize the list state
    let mut state = ListState::default();
    state.select(Some(0));  // Select the first item by default

    loop {
        // Render the UI
        terminal.draw(|frame| {
            let area = frame.area();

            // Create a vertical layout with a title and a list
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3),  // Title area
                        Constraint::Min(1),     // List area
                    ]
                    .as_ref(),
                )
                .split(area);

            let title_paragraph = Paragraph::new(title)
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center);
            frame.render_widget(title_paragraph, chunks[0]);

            let list_items: Vec<ListItem> = items
                .iter()
                .map(|item| ListItem::new(display_fn(item)))
                .collect();

            let list = List::new(list_items)
                .highlight_symbol("> ")
                .highlight_style(Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD));

            frame.render_stateful_widget(list, chunks[1], &mut state);
        })?;

        if let Event::Key(key) = event::read()? {
            // Filter out KeyEventKind::Release events for Windows - https://ratatui.rs/faq/#why-am-i-getting-duplicate-key-events-on-windows
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        disable_raw_mode()?;
                        return Err(color_eyre::eyre::eyre!("Selection aborted"));
                    }
                    KeyCode::Down => {
                        state.select_next();
                    }
                    KeyCode::Up => {
                        state.select_previous();
                    }
                    KeyCode::Enter => {
                        disable_raw_mode()?;
                        return Ok(state.selected().unwrap_or(0));
                    }
                    _ => {}
                }
            }
        }
    }
}