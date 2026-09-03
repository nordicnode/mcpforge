use anyhow::Result;
use chrono::{NaiveDate, Utc};
use mcpforge_registry::Registry;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\nMCPFORGE CATALOG PROVENANCE & LINK INTEGRITY AUDIT");
    println!("{}", "=".repeat(95));
    println!(
        "{:<20} {:<12} {:<12} {:<12} {:<35}",
        "SERVER ID", "STATUS", "HTTP CODE", "AGE (DAYS)", "SOURCE URL"
    );
    println!("{}", "-".repeat(95));

    let registry = Registry::default();
    let entries = registry.entries();
    let total = entries.len();

    let client = reqwest::Client::builder()
        .user_agent("mcpforge-catalog-audit/0.1.0 (+https://github.com/nordicnode/mcpforge)")
        .timeout(Duration::from_secs(10))
        .build()?;

    let semaphore = Arc::new(Semaphore::new(10));
    let mut tasks = Vec::new();

    let today = Utc::now().date_naive();

    for entry in entries {
        let client = client.clone();
        let sem = semaphore.clone();
        let id = entry.id.clone();
        let url = entry.source_url.clone();
        let last_verified_str = entry.last_verified.clone();

        tasks.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            let age_days = if let Some(ref d) = last_verified_str {
                if let Ok(parsed) = NaiveDate::parse_from_str(d, "%Y-%m-%d") {
                    Some((today - parsed).num_days())
                } else {
                    None
                }
            } else {
                None
            };

            let (status_str, http_code) = if let Some(ref u) = url {
                match client.head(u).send().await {
                    Ok(resp) => {
                        let code = resp.status().as_u16();
                        if resp.status().is_success() || resp.status().is_redirection() {
                            ("OK", format!("{}", code))
                        } else {
                            // Some sites reject HEAD, retry with GET
                            match client.get(u).send().await {
                                Ok(get_resp) => {
                                    let get_code = get_resp.status().as_u16();
                                    if get_resp.status().is_success()
                                        || get_resp.status().is_redirection()
                                    {
                                        ("OK", format!("{}", get_code))
                                    } else {
                                        ("FAILED", format!("{}", get_code))
                                    }
                                }
                                Err(e) => ("ERR", e.to_string()),
                            }
                        }
                    }
                    Err(_) => match client.get(u).send().await {
                        Ok(get_resp) => {
                            let get_code = get_resp.status().as_u16();
                            if get_resp.status().is_success() || get_resp.status().is_redirection()
                            {
                                ("OK", format!("{}", get_code))
                            } else {
                                ("FAILED", format!("{}", get_code))
                            }
                        }
                        Err(e) => ("ERR", e.to_string()),
                    },
                }
            } else {
                ("MISSING", "-".to_string())
            };

            (id, status_str, http_code, age_days, url)
        }));
    }

    let mut ok_count = 0;
    let mut fail_count = 0;
    let mut missing_count = 0;
    let mut stale_count = 0;

    for task in tasks {
        let (id, status, http_code, age_days, url) = task.await?;
        let age_display = age_days
            .map(|a| {
                if a > 180 {
                    stale_count += 1;
                    format!("{}d (STALE)", a)
                } else {
                    format!("{}d", a)
                }
            })
            .unwrap_or_else(|| "N/A".to_string());

        let url_display = url.unwrap_or_else(|| "-".to_string());

        println!(
            "{:<20} {:<12} {:<12} {:<12} {:<35}",
            id, status, http_code, age_display, url_display
        );

        match status {
            "OK" => ok_count += 1,
            "MISSING" => missing_count += 1,
            _ => fail_count += 1,
        }
    }

    println!("{}", "-".repeat(95));
    println!(
        "Audit Complete: {} total, {} verified reachable, {} failed/broken, {} missing URL, {} stale dates.\n",
        total, ok_count, fail_count, missing_count, stale_count
    );

    if fail_count > 0 {
        eprintln!(
            "WARNING: {} catalog source URLs failed verification.",
            fail_count
        );
    }

    Ok(())
}
