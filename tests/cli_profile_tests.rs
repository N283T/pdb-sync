//! Integration tests for profile preset CLI commands.

use std::process::Command;
use std::path::PathBuf;

/// Get the path to the pdb-sync binary.
fn pdb_sync_bin() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test binary name
    if path.ends_with("deps") {
        path.pop();
    }
    path.push("pdb-sync");
    path
}

/// Run pdb-sync with arguments and return the output.
fn run_pdb_sync(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(pdb_sync_bin())
        .args(args)
        .output()
        .expect("Failed to execute pdb-sync");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let status = output.status.code().unwrap_or(-1);

    (stdout, stderr, status)
}

#[test]
fn test_profile_list() {
    let (stdout, _stderr, status) = run_pdb_sync(&["sync", "profile", "list"]);

    assert_eq!(status, 0);
    assert!(stdout.contains("Available sync profile presets"));
    assert!(stdout.contains("structures (PDB Structures)"));
    assert!(stdout.contains("assemblies (PDB Assemblies)"));
    assert!(stdout.contains("emdb (EMDB Entries)"));
    assert!(stdout.contains("sifts (SIFTS Mapping)"));
}

#[test]
fn test_profile_show_valid() {
    let (stdout, _stderr, status) = run_pdb_sync(&["sync", "profile", "show", "structures"]);

    assert_eq!(status, 0);
    assert!(stdout.contains("structures"));
    assert!(stdout.contains("PDB Structures"));
    assert!(stdout.contains("Coordinate files"));
    assert!(stdout.contains("rsync://rsync.wwpdb.org/ftp_data/structures/divided/mmCIF/"));
    assert!(stdout.contains("wwpdb/structures/mmCIF"));
}

#[test]
fn test_profile_show_invalid() {
    let (_stdout, stderr, status) = run_pdb_sync(&["sync", "profile", "show", "invalid_preset"]);

    assert_ne!(status, 0);
    assert!(stderr.contains("not found") || stderr.contains("Invalid input"));
    assert!(stderr.contains("Available presets"));
}

#[test]
fn test_profile_show_emdb() {
    let (stdout, _stderr, status) = run_pdb_sync(&["sync", "profile", "show", "emdb"]);

    assert_eq!(status, 0);
    assert!(stdout.contains("emdb"));
    assert!(stdout.contains("EMDB Entries"));
    assert!(stdout.contains("data.pdbj.org::rsync/pub/emdb/"));
}

#[test]
fn test_profile_show_sifts() {
    let (stdout, _stderr, status) = run_pdb_sync(&["sync", "profile", "show", "sifts"]);

    assert_eq!(status, 0);
    assert!(stdout.contains("sifts"));
    assert!(stdout.contains("SIFTS Mapping"));
    assert!(stdout.contains("rsync://rsync.ebi.ac.uk/pub/databases/msd/sifts/"));
}

#[test]
fn test_profile_add_invalid() {
    let (_stdout, stderr, status) = run_pdb_sync(&["sync", "profile", "add", "invalid_preset"]);

    assert_ne!(status, 0);
    assert!(stderr.contains("not found") || stderr.contains("Invalid input"));
}

#[test]
fn test_profile_add_dry_run() {
    // Use a preset that likely doesn't exist in test config
    let (stdout, _stderr, status) = run_pdb_sync(&["sync", "profile", "add", "sifts", "--dry-run"]);

    assert_eq!(status, 0);
    assert!(stdout.contains("Dry run"));
    assert!(stdout.contains("sifts"));
}

#[test]
fn test_profile_help() {
    let (stdout, _stderr, status) = run_pdb_sync(&["sync", "profile", "--help"]);

    assert_eq!(status, 0);
    assert!(stdout.contains("Commands:") || stdout.contains("Subcommands"));
    assert!(stdout.contains("list"));
    assert!(stdout.contains("show"));
    assert!(stdout.contains("add"));
}
