#![deny(
    clippy::all,
    clippy::correctness,
    clippy::style,
    clippy::complexity,
    clippy::perf,
    clippy::pedantic,
    absolute_paths_not_starting_with_crate,
    anonymous_parameters,
    bad_style,
    const_err,
    dead_code,
    keyword_idents,
    improper_ctypes,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_abi,
    no_mangle_generic_items,
    non_shorthand_field_patterns,
    noop_method_call,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    pointer_structural_match,
    private_in_public,
    semicolon_in_expressions_from_macros,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unaligned_references,
    unconditional_recursion,
    unreachable_pub,
    unsafe_code,
    // unused,
    // unused_allocation,
    // unused_comparisons,
    // unused_extern_crates,
    // unused_import_braces,
    // unused_lifetimes,
    // unused_parens,
    // unused_qualifications,
    while_true
)]
#![allow(
    clippy::similar_names,
    clippy::struct_excessive_bools,
    clippy::shadow_reuse,
    clippy::too_many_lines,
    clippy::doc_markdown,
    clippy::single_match_else,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::upper_case_acronyms
)]

use clap::Parser;
use config::CONFIG;
use error::{Error, Result};
use once_cell::sync::Lazy;

mod apps;
mod cli;
mod common;
mod config;
mod error;
mod utils;

fn main() -> Result<()> {
    use cli::Cmd;
    use common::Handler;
    use std::collections::HashMap;

    // create config if it doesn't exist
    Lazy::force(&CONFIG);

    let mut apps = (*apps::APPS).clone();

    let res = || -> Result<()> {
        match Cmd::parse() {
            Cmd::Ask {
                path,
                skim,
                plain,
                config,
            } => {
                let selected = apps.ask_handler(&path.get_mime()?.0, skim, plain, config)?;
                selected.exec(common::ExecMode::Open, vec![path.to_string()])?;
            },
            Cmd::Set { mime, handler } => {
                apps.set_handler(mime.0, handler);
                apps.save()?;
            },
            Cmd::Add { mime, handler } => {
                apps.add_handler(mime.0, handler);
                apps.save()?;
            },
            Cmd::Launch { mime, args } => {
                apps.get_handler(&mime.0)?
                    .launch(args.into_iter().map(|a| a.to_string()).collect())?;
            },
            Cmd::Get { mime, json } => {
                apps.show_handler(&mime.0, json)?;
            },
            Cmd::Open { paths } => {
                let mut handlers: HashMap<Handler, Vec<String>> = HashMap::new();

                for path in paths {
                    handlers
                        .entry(apps.get_handler(&path.get_mime()?.0)?)
                        .or_default()
                        .push(path.to_string());
                }

                for (handler, paths) in handlers {
                    handler.open(paths)?;
                }
            },
            Cmd::List { all } => {
                apps.print(all);
            },
            Cmd::Unset { mime } => {
                apps.remove_handler(&mime.0)?;
            },
            Cmd::Edit { handler } => {
                apps.edit_handler(&handler)?;
            },
            Cmd::Cat { handler } => {
                apps.cat_handler(&handler)?;
            },
            Cmd::Status { handler } => {
                apps.get_status(&handler)?;
            },
            Cmd::Autocomplete {
                desktop_files,
                mimes,
            } =>
                if desktop_files {
                    apps::MimeApps::list_handlers()?;
                } else if mimes {
                    common::db_autocomplete()?;
                },
        }
        Ok(())
    }();

    match (res, atty::is(atty::Stream::Stdout)) {
        (Err(e), _) if matches!(e, Error::Cancelled) => {
            std::process::exit(1);
        },
        (Err(e), true) => {
            eprintln!("{}", e);
            std::process::exit(1);
        },
        (Err(e), false) => {
            utils::notify("handlr error", &e.to_string())?;
            std::process::exit(1);
        },
        _ => Ok(()),
    }
}
