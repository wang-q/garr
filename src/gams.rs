extern crate clap;
use clap::*;

pub mod cmd_gams;

fn main() -> anyhow::Result<()> {
    let app = Command::new("gams")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Genome Analyst with Rust and rediS")
        .propagate_version(true)
        .arg_required_else_help(true)
        .color(ColorChoice::Auto)
        .subcommand(cmd_gams::env::make_subcommand())
        .subcommand(cmd_gams::status::make_subcommand())
        .subcommand(cmd_gams::gen::make_subcommand())
        .subcommand(cmd_gams::locate::make_subcommand())
        .subcommand(cmd_gams::range::make_subcommand())
        .subcommand(cmd_gams::clear::make_subcommand())
        .subcommand(cmd_gams::feature::make_subcommand())
        .subcommand(cmd_gams::fsw::make_subcommand())
        .subcommand(cmd_gams::anno::make_subcommand())
        .subcommand(cmd_gams::sliding::make_subcommand())
        .subcommand(cmd_gams::peak::make_subcommand())
        .subcommand(cmd_gams::tsv::make_subcommand());

    // Check which subcomamnd the user ran...
    match app.get_matches().subcommand() {
        Some(("env", sub_matches)) => cmd_gams::env::execute(sub_matches),
        Some(("status", sub_matches)) => cmd_gams::status::execute(sub_matches),
        Some(("gen", sub_matches)) => cmd_gams::gen::execute(sub_matches),
        Some(("locate", sub_matches)) => cmd_gams::locate::execute(sub_matches),
        Some(("range", sub_matches)) => cmd_gams::range::execute(sub_matches),
        Some(("clear", sub_matches)) => cmd_gams::clear::execute(sub_matches),
        Some(("feature", sub_matches)) => cmd_gams::feature::execute(sub_matches),
        Some(("fsw", sub_matches)) => cmd_gams::fsw::execute(sub_matches),
        Some(("anno", sub_matches)) => cmd_gams::anno::execute(sub_matches),
        Some(("sliding", sub_matches)) => cmd_gams::sliding::execute(sub_matches),
        Some(("peak", sub_matches)) => cmd_gams::peak::execute(sub_matches),
        Some(("tsv", sub_matches)) => cmd_gams::tsv::execute(sub_matches),
        _ => unreachable!(),
    }
    .unwrap();

    Ok(())
}

// TODO: sliding windows of waves
// TODO: `gams count`
