// cargo run  -- --game-pk 813026

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use clap::Parser;
use reqwest::blocking::Client;
use serde_json::Value;

/// Summarize pitch types per pitcher for a single MLB game.
#[derive(Parser)]
struct Opts {
    /// Game primary key (gamePk) from MLB API. If provided, date/team args are ignored.
    #[arg(long)]
    game_pk: Option<u64>,

    /// Date of the game YYYY-MM-DD (used to look up the game when game-pk is not given)
    #[arg(long)]
    date: Option<String>,

    /// Home team name (substring match) for selecting the right game on the date
    #[arg(long)]
    home: Option<String>,

    /// Away team name (substring match) for selecting the right game on the date
    #[arg(long)]
    away: Option<String>,
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    let client = Client::builder()
        .user_agent("pitchers-cli/0.1")
        .build()?;

    let game_pk = if let Some(pk) = opts.game_pk {
        pk
    } else {
        let date = opts
            .date
            .as_deref()
            .context("date is required if game_pk is not supplied (format YYYY-MM-DD)")?;
        find_game_pk(&client, date, opts.home.as_deref(), opts.away.as_deref())?
    };

    let feed = fetch_game_feed(&client, game_pk)?;
    let summary = summarize_pitches(&feed);

    print_summary(&summary);

    Ok(())
}

fn find_game_pk(client: &Client, date: &str, home_filter: Option<&str>, away_filter: Option<&str>) -> Result<u64> {
    let url = format!("https://statsapi.mlb.com/api/v1/schedule?sportId=1&date={}", date);
    let resp: Value = client.get(&url).send()?.error_for_status()?.json()?;
    let dates = resp.get("dates").and_then(|v| v.as_array()).unwrap();

    for date_obj in dates {
        if let Some(games) = date_obj.get("games").and_then(|g| g.as_array()) {
            for g in games {
                let home_name = g
                    .get("teams")
                    .and_then(|t| t.get("home"))
                    .and_then(|h| h.get("team"))
                    .and_then(|tn| tn.get("name"))
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_lowercase();
                let away_name = g
                    .get("teams")
                    .and_then(|t| t.get("away"))
                    .and_then(|a| a.get("team"))
                    .and_then(|tn| tn.get("name"))
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_lowercase();

                let home_ok = home_filter.map_or(true, |f| home_name.contains(&f.to_lowercase()));
                let away_ok = away_filter.map_or(true, |f| away_name.contains(&f.to_lowercase()));

                if home_ok && away_ok {
                    if let Some(pk) = g.get("gamePk").and_then(|p| p.as_u64()) {
                        return Ok(pk);
                    }
                }
            }
        }
    }

    bail!("no matching game found for date {} and filters home={:?} away={:?}", date, home_filter, away_filter);
}

fn fetch_game_feed(client: &Client, game_pk: u64) -> Result<Value> {
    let url = format!("https://statsapi.mlb.com/api/v1.1/game/{}/feed/live", game_pk);
    let resp: Value = client.get(&url).send()?.error_for_status()?.json()?;
    Ok(resp)
}

fn summarize_pitches(feed: &Value) -> HashMap<String, (String, HashMap<String, u32>)> {
    // Map: pitcher_name -> (team_name, map<pitch_name, count>)
    let mut result: HashMap<String, (String, HashMap<String, u32>)> = HashMap::new();

    let all_plays = feed
        .get("liveData")
        .and_then(|ld| ld.get("plays"))
        .and_then(|p| p.get("allPlays"))
        .and_then(|ap| ap.as_array())
        .unwrap();

    for play in all_plays {
        // pitcher info usually on the play's "matchup"
        let pitcher_name = play
            .get("matchup")
            .and_then(|m| m.get("pitcher"))
            .and_then(|p| p.get("fullName"))
            .and_then(|s| s.as_str())
            .unwrap_or("Unknown pitcher")
            .to_string();

        let team_name = play
            .get("matchup")
            .and_then(|m| m.get("pitcher"))
            .and_then(|p| p.get("team"))
            .and_then(|t| t.get("name"))
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();

        // collect pitch events: some plays have "playEvents" (array)
        if let Some(events) = play.get("playEvents").and_then(|e| e.as_array()) {
            for ev in events {
                if is_pitch_event(ev) {
                    let raw_type = find_pitch_type(ev);
                    let pitch_name = normalize_pitch_type(&raw_type);
                    let entry = result.entry(pitcher_name.clone()).or_insert_with(|| (team_name.clone(), HashMap::new()));
                    *entry.1.entry(pitch_name).or_insert(0) += 1;
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
    // attempt several paths where pitch type might live
    // if let Some(pitch_data) = ev.get("pitchData") {

    //     println!("pitch_data: {:?}", pitch_data);
    //     println!("");

    //     if let Some(pt) = pitch_data.get("pitchType").and_then(|v| v.as_str()) {
    //         return pt.to_string();
    //     }
    //     if let Some(details) = pitch_data.get("details") {
    //         if let Some(t) = details.get("type").and_then(|v| v.as_str()) {
    //             // often this is a pitch code like "FF", or sometimes an action - we return it for mapping
    //             return t.to_string();
    //         }
    //         if let Some(desc) = details.get("description").and_then(|v| v.as_str()) {
    //             return desc.to_string();
    //         }
    //     }
    //     if let Some(t) = pitch_data.get("type").and_then(|v| v.as_str()) {
    //         return t.to_string();
    //     }
    // }

    if let Some(details) = ev.get("details") {
        // if let Some(t) = details.get("type").and_then(|v| v.as_str()) {
        //     return t.to_string();
        // }
        if let Some(t) = details.get("type").and_then(|v| v.get("description")).and_then(|v| v.as_str()) {
            return t.to_string();
        }

        if let Some(desc) = details.get("description").and_then(|v| v.as_str()) {
            return desc.to_string();
        }
    }

    "unknown".to_string()
}

fn normalize_pitch_type(raw: &str) -> String {
    let code = raw.trim();
    if code.is_empty() {
        return "unknown".to_string();
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
            return v.to_string();
        }
    }

    // substring matching for common names
    let low = code.to_lowercase();
    if low.contains("fast") || low == "fastball" || low.contains("fb") {
        return "fastball".to_string();
    }
    if low.contains("slider") || low.contains("sl") && low.len() <= 3 {
        return "slider".to_string();
    }
    if low.contains("curve") || low.contains("cu") {
        return "curveball".to_string();
    }
    if low.contains("change") || low.contains("ch") {
        return "changeup".to_string();
    }
    if low.contains("sinker") || low == "si" {
        return "sinker".to_string();
    }
    if low.contains("cutter") || low == "fc" {
        return "cutter".to_string();
    }

    // fallback to returning the raw label (helpful when API gives full text)
    code.to_string()
}

fn print_summary(summary: &HashMap<String, (String, HashMap<String, u32>)>) {
    // print pitchers sorted by name
    println!("");
    let mut names: Vec<_> = summary.keys().collect();
    names.sort();
    for name in names {
        let (_team, pitches) = &summary[name];

        let sum: u32 = pitches.values().sum();

        println!("{:13} ({:>2})", name, sum);
        // sort pitch types by count descending
        let mut pairs: Vec<_> = pitches.iter().collect();
        pairs.sort_by(|a, b| b.1.cmp(a.1));
        for (ptype, count) in pairs {
            println!("  {:12} {:>2}", ptype, count);
        }
        println!();
    }
}
