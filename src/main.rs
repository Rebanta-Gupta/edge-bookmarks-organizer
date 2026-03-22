//! Edge Bookmarks Organizer CLI
//!
//! A command-line tool for managing Microsoft Edge bookmarks:
//! - Detect and remove duplicates
//! - Find and remove dead links
//! - Organize bookmarks by domain or topic
//! - Backup and restore bookmark files

use clap::{Parser, Subcommand};
use colored::*;
use edge_bookmarks_organizer::{
    backup, deadlinks, duplicates, embeddings, error::Result, organizer, parser, rebuilder,
    Bookmark, BookmarksFile,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "edge-bookmarks")]
#[command(author = "Edge Bookmarks Organizer")]
#[command(version = "0.1.0")]
#[command(about = "Organize and clean up Microsoft Edge bookmarks", long_about = None)]
struct Cli {
    /// Path to the Edge Bookmarks file (auto-detected if not specified)
    #[arg(short, long, global = true)]
    bookmarks_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import and display bookmark statistics
    Import,

    /// List all bookmarks grouped by domain
    ListDomains {
        /// Show individual bookmark names within each domain
        #[arg(short, long)]
        verbose: bool,

        /// Minimum number of bookmarks to show a domain
        #[arg(short, long, default_value = "1")]
        min_count: usize,
    },

    /// List duplicate bookmarks (same URL)
    ListDuplicates {
        /// Show full URLs instead of just names
        #[arg(short, long)]
        verbose: bool,
    },

    /// Remove duplicate bookmarks, keeping the first occurrence
    RemoveDuplicates {
        /// Actually write changes (dry-run by default)
        #[arg(long)]
        apply: bool,

        /// Keep most recently used duplicate instead of first
        #[arg(long)]
        keep_recent: bool,
    },

    /// Check all bookmarks for dead links
    CheckDead {
        /// Request timeout in seconds
        #[arg(short, long, default_value = "5")]
        timeout: u64,

        /// Number of concurrent requests
        #[arg(short, long, default_value = "10")]
        concurrency: usize,

        /// Only show dead/unreachable links
        #[arg(long)]
        only_dead: bool,
    },

    /// Remove dead links from bookmarks
    RemoveDead {
        /// Actually write changes (dry-run by default)
        #[arg(long)]
        apply: bool,

        /// Request timeout in seconds
        #[arg(short, long, default_value = "5")]
        timeout: u64,

        /// Number of concurrent requests
        #[arg(short, long, default_value = "10")]
        concurrency: usize,
    },

    /// Rebuild bookmarks with a new organization structure
    Rebuild {
        /// Organization strategy: domain, preserve, topic
        #[arg(short, long, default_value = "domain")]
        strategy: String,

        /// Actually write changes (dry-run by default)
        #[arg(long)]
        apply: bool,
    },

    /// Save current state to the bookmarks file (creates backup first)
    Save {
        /// Skip creating a backup
        #[arg(long)]
        no_backup: bool,
    },

    /// List or restore from backups
    Backup {
        /// List all backups
        #[arg(long)]
        list: bool,

        /// Restore from a specific backup file
        #[arg(long)]
        restore: Option<PathBuf>,

        /// Delete old backups, keeping only N most recent
        #[arg(long)]
        prune: Option<usize>,
    },

    /// Assign topics to bookmarks using keyword analysis
    AssignTopics {
        /// Show topic assignments without saving
        #[arg(long)]
        dry_run: bool,
    },
}

/// Application state holding loaded bookmarks.
struct App {
    bookmarks_path: PathBuf,
    bookmarks_file: BookmarksFile,
    bookmarks: Vec<Bookmark>,
}

impl App {
    fn new(path: Option<PathBuf>) -> Result<Self> {
        let bookmarks_path = match path {
            Some(p) => p,
            None => parser::get_default_bookmarks_path()?,
        };

        println!(
            "{} {}",
            "Loading bookmarks from:".cyan(),
            bookmarks_path.display()
        );

        let bookmarks_file = parser::load_bookmarks_file(&bookmarks_path)?;
        let bookmarks = parser::parse_bookmarks(&bookmarks_file);

        println!(
            "{} {} bookmarks loaded\n",
            "✓".green(),
            bookmarks.len().to_string().yellow()
        );

        Ok(Self {
            bookmarks_path,
            bookmarks_file,
            bookmarks,
        })
    }

    fn save_bookmarks(&mut self, no_backup: bool) -> Result<()> {
        if !no_backup {
            let backup_path = backup::create_backup(&self.bookmarks_path)?;
            println!(
                "{} Backup created: {}",
                "✓".green(),
                backup_path.display()
            );
        }

        // Rebuild the file structure
        let new_file = rebuilder::rebuild_bookmarks_file(
            &self.bookmarks_file,
            &self.bookmarks,
            rebuilder::OrganizeStrategy::PreserveOriginal,
        );

        rebuilder::write_bookmarks_file(&new_file, &self.bookmarks_path)?;
        println!(
            "{} Bookmarks saved to: {}",
            "✓".green(),
            self.bookmarks_path.display()
        );

        Ok(())
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Import => {
            let app = App::new(cli.bookmarks_file)?;
            cmd_import(&app)?;
        }

        Commands::ListDomains { verbose, min_count } => {
            let app = App::new(cli.bookmarks_file)?;
            cmd_list_domains(&app, verbose, min_count)?;
        }

        Commands::ListDuplicates { verbose } => {
            let app = App::new(cli.bookmarks_file)?;
            cmd_list_duplicates(&app, verbose)?;
        }

        Commands::RemoveDuplicates { apply, keep_recent } => {
            let mut app = App::new(cli.bookmarks_file)?;
            cmd_remove_duplicates(&mut app, apply, keep_recent)?;
        }

        Commands::CheckDead {
            timeout,
            concurrency,
            only_dead,
        } => {
            let app = App::new(cli.bookmarks_file)?;
            cmd_check_dead(&app, timeout, concurrency, only_dead)?;
        }

        Commands::RemoveDead {
            apply,
            timeout,
            concurrency,
        } => {
            let mut app = App::new(cli.bookmarks_file)?;
            cmd_remove_dead(&mut app, apply, timeout, concurrency)?;
        }

        Commands::Rebuild { strategy, apply } => {
            let mut app = App::new(cli.bookmarks_file)?;
            cmd_rebuild(&mut app, &strategy, apply)?;
        }

        Commands::Save { no_backup } => {
            let mut app = App::new(cli.bookmarks_file)?;
            app.save_bookmarks(no_backup)?;
        }

        Commands::Backup {
            list,
            restore,
            prune,
        } => {
            let bookmarks_path = match cli.bookmarks_file {
                Some(p) => p,
                None => parser::get_default_bookmarks_path()?,
            };
            cmd_backup(&bookmarks_path, list, restore, prune)?;
        }

        Commands::AssignTopics { dry_run } => {
            let mut app = App::new(cli.bookmarks_file)?;
            cmd_assign_topics(&mut app, dry_run)?;
        }
    }

    Ok(())
}

// =============================================================================
// Command implementations
// =============================================================================

fn cmd_import(app: &App) -> Result<()> {
    println!("{}", "═".repeat(60).cyan());
    println!("{}", "  BOOKMARK STATISTICS".cyan().bold());
    println!("{}", "═".repeat(60).cyan());

    let total = app.bookmarks.len();
    let domain_stats = organizer::get_domain_stats(&app.bookmarks);
    let dupe_stats = duplicates::get_duplicate_stats(&app.bookmarks);

    println!("\n{}", "Summary:".yellow().bold());
    println!("  Total bookmarks:     {}", total.to_string().green());
    println!("  Unique domains:      {}", domain_stats.len().to_string().green());
    println!(
        "  Duplicate URLs:      {}",
        dupe_stats.total_duplicates.to_string().yellow()
    );
    println!(
        "  URLs with dupes:     {}",
        dupe_stats.unique_urls_with_dupes.to_string().yellow()
    );

    println!("\n{}", "Top 10 domains:".yellow().bold());
    for (i, stat) in domain_stats.iter().take(10).enumerate() {
        println!(
            "  {}. {} ({})",
            i + 1,
            stat.domain.green(),
            stat.count.to_string().cyan()
        );
    }

    // Show folder distribution
    let mut folder_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for bookmark in &app.bookmarks {
        let top_folder = bookmark
            .folder_path
            .split('/')
            .next()
            .unwrap_or("(root)")
            .to_string();
        *folder_counts.entry(top_folder).or_default() += 1;
    }

    let mut folder_stats: Vec<_> = folder_counts.into_iter().collect();
    folder_stats.sort_by(|a, b| b.1.cmp(&a.1));

    println!("\n{}", "Top-level folders:".yellow().bold());
    for (folder, count) in folder_stats.iter().take(10) {
        println!("  {} ({})", folder.green(), count.to_string().cyan());
    }

    Ok(())
}

fn cmd_list_domains(app: &App, verbose: bool, min_count: usize) -> Result<()> {
    let domain_stats = organizer::get_domain_stats(&app.bookmarks);

    println!("{}", "═".repeat(60).cyan());
    println!("{}", "  BOOKMARKS BY DOMAIN".cyan().bold());
    println!("{}", "═".repeat(60).cyan());

    let filtered: Vec<_> = domain_stats
        .iter()
        .filter(|s| s.count >= min_count)
        .collect();

    println!(
        "\nShowing {} domains with {} or more bookmarks:\n",
        filtered.len().to_string().green(),
        min_count.to_string().yellow()
    );

    for stat in filtered {
        println!(
            "{} {} bookmark{}",
            stat.domain.green().bold(),
            stat.count.to_string().cyan(),
            if stat.count == 1 { "" } else { "s" }
        );

        if verbose {
            for name in &stat.bookmarks {
                println!("    • {}", name.dimmed());
            }
        }
    }

    Ok(())
}

fn cmd_list_duplicates(app: &App, verbose: bool) -> Result<()> {
    let dupe_stats = duplicates::get_duplicate_stats(&app.bookmarks);

    println!("{}", "═".repeat(60).cyan());
    println!("{}", "  DUPLICATE BOOKMARKS".cyan().bold());
    println!("{}", "═".repeat(60).cyan());

    if dupe_stats.groups.is_empty() {
        println!("\n{}", "No duplicates found!".green().bold());
        return Ok(());
    }

    println!(
        "\nFound {} duplicate groups ({} extra bookmarks):\n",
        dupe_stats.unique_urls_with_dupes.to_string().yellow(),
        dupe_stats.total_duplicates.to_string().red()
    );

    for (i, group) in dupe_stats.groups.iter().enumerate() {
        println!(
            "{}. {} copies",
            (i + 1).to_string().cyan(),
            group.bookmarks.len().to_string().yellow()
        );

        if verbose {
            println!("   URL: {}", group.normalized_url.dimmed());
        }

        for bookmark in &group.bookmarks {
            println!(
                "    • {} {}",
                bookmark.name.green(),
                format!("({})", bookmark.folder_path).dimmed()
            );
        }
        println!();
    }

    println!(
        "{}",
        "Run with --apply to remove duplicates (keeps first occurrence)".dimmed()
    );

    Ok(())
}

fn cmd_remove_duplicates(app: &mut App, apply: bool, keep_recent: bool) -> Result<()> {
    let original_count = app.bookmarks.len();
    
    let deduped = if keep_recent {
        duplicates::remove_duplicates_keep_recent(app.bookmarks.clone())
    } else {
        duplicates::remove_duplicates(app.bookmarks.clone())
    };

    let removed = original_count - deduped.len();

    if removed == 0 {
        println!("{}", "No duplicates found!".green().bold());
        return Ok(());
    }

    println!(
        "Found {} duplicates to remove",
        removed.to_string().yellow()
    );

    if apply {
        app.bookmarks = deduped;
        app.save_bookmarks(false)?;
        println!(
            "{} Removed {} duplicates",
            "✓".green(),
            removed.to_string().red()
        );
    } else {
        println!("\n{}", "Dry run - no changes made".yellow().bold());
        println!("{}", "Run with --apply to save changes".dimmed());
    }

    Ok(())
}

fn cmd_check_dead(app: &App, timeout: u64, concurrency: usize, only_dead: bool) -> Result<()> {
    println!("{}", "═".repeat(60).cyan());
    println!("{}", "  DEAD LINK CHECK".cyan().bold());
    println!("{}", "═".repeat(60).cyan());
    println!(
        "\nChecking {} bookmarks (timeout: {}s, concurrency: {})...\n",
        app.bookmarks.len().to_string().yellow(),
        timeout,
        concurrency
    );

    let config = deadlinks::CheckConfig {
        timeout_secs: timeout,
        concurrency,
        ..Default::default()
    };

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let checked = runtime.block_on(deadlinks::check_bookmarks(
        app.bookmarks.clone(),
        &config,
        true,
    ));

    let stats = deadlinks::LinkCheckStats::from_checked(&checked);

    println!("\n{}", "Results:".yellow().bold());
    println!("  Alive:       {}", stats.alive.to_string().green());
    println!("  Dead:        {}", stats.dead.to_string().red());
    println!("  Unreachable: {}", stats.unreachable.to_string().yellow());

    let dead_links = deadlinks::filter_dead_links(&checked);

    if !dead_links.is_empty() && (only_dead || dead_links.len() <= 50) {
        println!("\n{}", "Dead/Unreachable links:".red().bold());
        for cb in dead_links {
            let status_str = match &cb.status {
                edge_bookmarks_organizer::LinkStatus::Dead { status_code } => {
                    format!("HTTP {}", status_code).red().to_string()
                }
                edge_bookmarks_organizer::LinkStatus::Unreachable { reason } => {
                    format!("Unreachable: {}", reason).yellow().to_string()
                }
                _ => "Unknown".to_string(),
            };
            println!(
                "  • {} - {}",
                cb.bookmark.name.green(),
                status_str
            );
            println!("    {}", cb.bookmark.url.dimmed());
        }
    }

    Ok(())
}

fn cmd_remove_dead(app: &mut App, apply: bool, timeout: u64, concurrency: usize) -> Result<()> {
    println!("Checking all bookmarks for dead links...\n");

    let config = deadlinks::CheckConfig {
        timeout_secs: timeout,
        concurrency,
        ..Default::default()
    };

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let checked = runtime.block_on(deadlinks::check_bookmarks(
        app.bookmarks.clone(),
        &config,
        true,
    ));

    let stats = deadlinks::LinkCheckStats::from_checked(&checked);
    let dead_count = stats.dead + stats.unreachable;

    if dead_count == 0 {
        println!("{}", "No dead links found!".green().bold());
        return Ok(());
    }

    println!(
        "Found {} dead/unreachable links",
        dead_count.to_string().red()
    );

    if apply {
        let alive_bookmarks = deadlinks::remove_dead_bookmarks(checked);
        app.bookmarks = alive_bookmarks;
        app.save_bookmarks(false)?;
        println!(
            "{} Removed {} dead links",
            "✓".green(),
            dead_count.to_string().red()
        );
    } else {
        println!("\n{}", "Dry run - no changes made".yellow().bold());
        println!("{}", "Run with --apply to save changes".dimmed());
    }

    Ok(())
}

fn cmd_rebuild(app: &mut App, strategy: &str, apply: bool) -> Result<()> {
    let organize_strategy = match strategy.to_lowercase().as_str() {
        "domain" => rebuilder::OrganizeStrategy::ByDomain,
        "preserve" | "original" => rebuilder::OrganizeStrategy::PreserveOriginal,
        "topic" => rebuilder::OrganizeStrategy::ByTopic,
        _ => {
            eprintln!(
                "{} Unknown strategy '{}'. Use: domain, preserve, or topic",
                "Error:".red(),
                strategy
            );
            return Ok(());
        }
    };

    println!(
        "Rebuilding bookmarks with strategy: {}",
        strategy.yellow().bold()
    );

    if apply {
        let new_file = rebuilder::rebuild_bookmarks_file(
            &app.bookmarks_file,
            &app.bookmarks,
            organize_strategy,
        );

        backup::create_backup(&app.bookmarks_path)?;
        rebuilder::write_bookmarks_file(&new_file, &app.bookmarks_path)?;

        println!(
            "{} Bookmarks reorganized and saved!",
            "✓".green()
        );
    } else {
        println!("\n{}", "Dry run - no changes made".yellow().bold());
        println!("{}", "Run with --apply to save changes".dimmed());
    }

    Ok(())
}

fn cmd_backup(
    bookmarks_path: &PathBuf,
    list: bool,
    restore: Option<PathBuf>,
    prune: Option<usize>,
) -> Result<()> {
    if list {
        let backups = backup::list_backups(bookmarks_path)?;
        if backups.is_empty() {
            println!("{}", "No backups found".yellow());
        } else {
            println!("{}", "Available backups:".cyan().bold());
            for (i, path) in backups.iter().rev().enumerate() {
                println!(
                    "  {}. {}",
                    (i + 1),
                    path.file_name().unwrap().to_string_lossy().green()
                );
            }
        }
        return Ok(());
    }

    if let Some(restore_path) = restore {
        backup::restore_backup(&restore_path, bookmarks_path)?;
        println!(
            "{} Restored from: {}",
            "✓".green(),
            restore_path.display()
        );
        return Ok(());
    }

    if let Some(keep) = prune {
        let deleted = backup::prune_backups(bookmarks_path, keep)?;
        println!(
            "{} Deleted {} old backups, keeping {} most recent",
            "✓".green(),
            deleted,
            keep
        );
        return Ok(());
    }

    // Default: create a new backup
    let backup_path = backup::create_backup(bookmarks_path)?;
    println!(
        "{} Backup created: {}",
        "✓".green(),
        backup_path.display()
    );

    Ok(())
}

fn cmd_assign_topics(app: &mut App, dry_run: bool) -> Result<()> {
    println!("Analyzing bookmarks for topic assignment...\n");

    let extractor = embeddings::TopicExtractor::new();
    extractor.assign_topics(&mut app.bookmarks);

    // Count by topic
    let groups = embeddings::group_by_topic(&app.bookmarks);
    let mut topic_counts: Vec<_> = groups
        .iter()
        .map(|(topic, bookmarks)| (topic.clone(), bookmarks.len()))
        .collect();
    topic_counts.sort_by(|a, b| b.1.cmp(&a.1));

    println!("{}", "Topic Distribution:".cyan().bold());
    for (topic, count) in &topic_counts {
        println!(
            "  {} ({} bookmarks)",
            topic.green(),
            count.to_string().cyan()
        );
    }

    if !dry_run {
        app.save_bookmarks(false)?;
        println!("\n{} Topics saved to bookmarks", "✓".green());
    } else {
        println!("\n{}", "Dry run - no changes made".yellow().bold());
    }

    Ok(())
}
