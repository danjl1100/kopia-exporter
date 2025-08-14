use kopia_exporter::kopia;
use std::fs;

fn main() {
    // Test parsing the sample JSON
    match fs::read_to_string("src/sample_kopia-snapshot-list.json") {
        Ok(content) => {
            match kopia::parse_snapshots(&content) {
                Ok(snapshots) => {
                    println!("Successfully parsed {} snapshots", snapshots.len());
                    
                    if let Some(latest) = snapshots.last() {
                        println!("Latest snapshot: {} ({})", latest.id, latest.start_time);
                        println!("  Total size: {} bytes", latest.stats.total_size);
                        println!("  Errors: {}", latest.stats.error_count);
                        println!("  Failed files: {}", latest.root_entry.summ.num_failed);
                    }
                    
                    let retention_counts = kopia::get_retention_counts(&snapshots);
                    println!("Retention counts: {:?}", retention_counts);
                }
                Err(e) => {
                    eprintln!("Failed to parse JSON: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to read sample file: {}", e);
        }
    }
}
