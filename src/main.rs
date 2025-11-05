// cargo run  -- --id 813026

use std::collections::HashMap;

use anyhow::Result;
use clap::Parser;
use reqwest::blocking::Client;
use serde_json::Value;
use colored::Colorize;

/// Summarize pitch types per pitcher for a single MLB game.
#[derive(Parser)]
struct Opts {
    /// Game id from MLB API. If provided, date/team args are ignored.
    #[arg(long)]
    id: u64,
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    let client = Client::builder()
        .user_agent("pitchers-cli/0.1")
        .build()?;

    let game_id = opts.id;

    let feed = fetch_game_feed(&client, game_id)?;
    let summary = summarize_pitches(&feed);

    print_summary(&summary);

    Ok(())
}

fn fetch_game_feed(client: &Client, game_pk: u64) -> Result<Value> {
    let url = format!("https://statsapi.mlb.com/api/v1.1/game/{}/feed/live", game_pk);
    let resp: Value = client.get(&url).send()?.error_for_status()?.json()?;
    Ok(resp)
}

fn summarize_pitches(feed: &Value) -> HashMap<String, HashMap<String, HashMap<String, u32>>> {
    let mut result: HashMap<String, HashMap<String, HashMap<String, u32>>> = HashMap::new();

    let all_plays = feed
        .get("liveData")
        .and_then(|ld| ld.get("plays"))
        .and_then(|p| p.get("allPlays"))
        .and_then(|ap| ap.as_array())
        .unwrap();

    for play in all_plays {
        let pitcher_name = play
            .get("matchup")
            .and_then(|m| m.get("pitcher"))
            .and_then(|p| p.get("fullName"))
            .and_then(|s| s.as_str())
            .unwrap_or("Unknown pitcher")
            .to_string();

        if let Some(events) = play.get("playEvents").and_then(|e| e.as_array()) {
            for ev in events {
                if is_pitch_event(ev) {
                    let raw_type = find_pitch_type(ev);
                    let (pitch_name, pitch_category) = normalize_pitch_type(&raw_type);

                    let pitcher_entry = result.entry(pitcher_name.clone()).or_insert_with(HashMap::new);
                    let category_map = pitcher_entry
                        .entry(pitch_category)
                        .or_insert_with(HashMap::new);
                    *category_map.entry(pitch_name).or_insert(0) += 1;
                }
            }
        }
    }

    result
}

fn is_pitch_event(ev: &Value) -> bool {
    if let Some(b) = ev.get("isPitch").and_then(|v| v.as_bool()) {
        return b;
    }
    ev.get("pitchData").is_some()
}

fn find_pitch_type(ev: &Value) -> String {
    if let Some(details) = ev.get("details") {
        if let Some(t) = details.get("type").and_then(|v| v.get("description")).and_then(|v| v.as_str()) {
            return t.to_string().to_lowercase();
        }

        if let Some(desc) = details.get("description").and_then(|v| v.as_str()) {
            return desc.to_string();
        }
    }

    "unknown".to_string()
}

fn normalize_pitch_type(raw: &str) -> (String, String) {
    let code = raw.trim();
    if code.is_empty() {
        return ("unknown".to_string(),  "unknown".to_string());
    }
    // common code-to-name mapping
    let mappings: &[(&str, &str)] = &[
        ("FF", "fastball"),
        ("FA", "fastball"),
        ("FT", "fastball"),
        ("FF/FT", "fastball"),
        ("SI", "sinker"),
        ("SL", "slider"),
        ("CU", "curveball"),
        ("KC", "curveball"),
        ("CH", "changeup"),
        ("FC", "cutter"),
        ("FS", "splitter"),
        ("IN", "intentional"),
    ];

    // direct code match (uppercase)
    let up = code.to_uppercase();
    for (k, v) in mappings {
        if up == *k {
            return (v.to_string(), v.to_string());
        }
    }

    // substring matching for common names
    let low = code.to_lowercase();
    if low.contains("fast")  {
        return ("fastball".to_string(), "heater".to_string());
    }
    if low.contains("slider")  {
        return ("slider".to_string(), "breaking ball".to_string());
    }
    if low.contains("curve")  {
        return ("curveball".to_string(), "breaking ball".to_string());
    }
    if low.contains("change") {
        return ("changeup".to_string(), "offspeed".to_string());
    }
    if low.contains("sinker")  {
        return ("sinker".to_string(), "heater".to_string());
    }
    if low.contains("cutter")  {
        return ("cutter".to_string(), "heater".to_string());
    }
    if low.contains("splitter")  {
        return ("splitter".to_string(), "offspeed".to_string());
    }
    if low.contains("sweeper")  {
        return ("sweeper".to_string(), "breaking ball".to_string());
    }
    if low.contains("knuckle curve")  {
        return ("knuckle curve".to_string(), "breaking ball".to_string());
    }
    if low.contains("knuckleball")  {
        return ("knuckleball".to_string(), "other".to_string());
    }
    // fallback to returning the raw label (helpful when API gives full text)
    (code.to_string(), code.to_string())
}

fn print_summary(summary: &HashMap<String, HashMap<String, HashMap<String, u32>>>) {
    println!("");
    let mut names: Vec<_> = summary.keys().collect();
    names.sort();
    let preferred = ["heater", "breaking ball", "offspeed"];

    for name in names {
        let categories = &summary[name];

        let total: u32 = categories.values().flat_map(|m| m.values()).sum();
        // pad name first so ANSI escape sequences don't break alignment
        let name_padded = format!("{:13}", name.bright_white().bold());
        println!("{} ({})", &name_padded, total.to_string().bright_white().bold());

        // print preferred categories first in that order
        for cat in &preferred {
            if let Some(pitches) = categories.get(*cat) {
                let cat_total: u32 = pitches.values().sum();
                println!("  {} {:>2}", cat.bright_yellow().bold(), cat_total);

                let mut pairs: Vec<_> = pitches.iter().collect();
                pairs.sort_by(|a, b| b.1.cmp(a.1));
                for (ptype, count) in pairs {
                    println!("    {:12} {:>3}", ptype, count);
                }
            }
        }

        // then any other categories (sorted)
        let mut other: Vec<_> = categories
            .keys()
            .filter(|k| !preferred.contains(&k.as_str()))
            .collect();
        other.sort();
        for cat in other {
            if let Some(pitches) = categories.get(cat) {
                let cat_total: u32 = pitches.values().sum();
                println!("  {} {:>2}", cat.bright_yellow().bold(), cat_total);

                let mut pairs: Vec<_> = pitches.iter().collect();
                pairs.sort_by(|a, b| b.1.cmp(a.1));
                for (ptype, count) in pairs {
                    println!("    {:12} {:>3}", ptype, count);
                }
            }
        }

        println!();
    }
}
