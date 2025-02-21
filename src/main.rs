mod fetch;

use clap::{Arg, Command};
use fetch::Fetch;

#[tokio::main]
async fn main() {
    // Parse command-line arguments using `clap`
    let args = match parse_args() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("Error: {}", err);
            return;
        }
    };

    // Extract arguments for convenience
    let db_host = args.db_url.clone();
    let table_name = args.table_name.clone();
    let dry_run = args.dry_run;
    let limit = args.limit;

    let fetcher = Fetch::new(&db_host, &table_name, limit);

    fetcher
        .with_callback(Box::new(|doc| {
            println!("{:?}", doc);
        })) // closure to be called for each document
        .execute()
        .await;
}

pub struct Args {
    pub db_url: String,       // URL of the CouchDB database
    pub table_name: String,   // Name of the table (or document type)
    pub dry_run: bool,        // Whether to perform a dry run (preview changes without modifying the database)
    pub limit: usize,         // Maximum number of documents to fetch per iteration
}

/// Parse command-line arguments using `clap`
pub fn parse_args() -> Result<Args, String> {
    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");
    let authors = env!("CARGO_PKG_AUTHORS");
    let description = env!("CARGO_PKG_DESCRIPTION");

    let matches = Command::new(name)
        .version(version)
        .author(authors)
        .about(description)
        .arg(
            Arg::new("db_prefix")
                .short('u')
                .long("url")
                .value_name("URL")
                .help("URL of the CouchDB database (Example: http://localhost:5984)")
                .required(true),
        )
        .arg(
            Arg::new("table_name")
                .short('t')
                .long("table")
                .value_name("TABLE")
                .help("Name of the table (or document type)")
                .required(true),
        )
        .arg(
            Arg::new("dry_run")
                .long("dry-run") // Use --dry-run to enable dry-run mode
                .help("Enable dry-run mode (preview changes without modifying the database)")
                .action(clap::ArgAction::SetTrue) // Defaults to false unless --dry-run is provided
                .default_value("false"), // Default value is false (not dry-run)
        )
        .arg(
            Arg::new("limit")
                .short('l')
                .long("limit")
                .value_name("LIMIT")
                .default_value("1000")
                .value_parser(clap::value_parser!(usize))
                .help("Maximum number of documents to fetch per iteration"),
        )
        .get_matches();

    // Extract arguments from matches
    let db_url = matches.get_one::<String>("db_prefix").unwrap().clone();
    let table_name = matches.get_one::<String>("table_name").unwrap().clone();
    let dry_run = *matches.get_one::<bool>("dry_run").unwrap_or(&false);
    let limit = *matches.get_one::<usize>("limit").unwrap_or(&1000);


    Ok(Args {
        db_url,
        table_name,
        dry_run,
        limit,
    })
}
