use crate::common::{Handler, MimeOrExtension, UserPath};

#[derive(clap::Parser)]
#[clap(
    version = clap::crate_version!(),
    global_setting = clap::AppSettings::DeriveDisplayOrder,
    global_setting = clap::AppSettings::DisableHelpSubcommand,
    color = clap::ColorChoice::Auto,
)]
pub(crate) enum Cmd {
    /// List default apps and the associated handlers
    #[clap(aliases = &["ls", "l", "list"])]
    List {
        #[clap(long, short)]
        all: bool,
    },

    /// Open a path/URL with its default handler
    Open {
        #[clap(required = true)]
        paths: Vec<UserPath>,
    },

    /// Set the default handler for mime/extension
    Set {
        mime:    MimeOrExtension,
        handler: Handler,
    },

    /// Unset the default handler for mime/extension
    Unset { mime: MimeOrExtension },

    /// Launch the handler for specified extension/mime with optional arguments
    Launch {
        mime: MimeOrExtension,
        args: Vec<UserPath>,
    },

    /// Get handler for this mime/extension
    Get {
        #[clap(long)]
        json: bool,
        mime: MimeOrExtension,
    },

    /// Add a handler for given mime/extension
    /// Note that the first handler is the default
    Add {
        mime:    MimeOrExtension,
        handler: Handler,
    },

    /// Edit a desktop file in $EDITOR
    Edit { handler: Handler },

    /// Display a desktop file in the terminal
    Cat { handler: Handler },

    /// Get the status of whether or not the desktop file is in use
    Status { handler: Handler },

    /// Ask which application should open the file
    Ask {
        #[clap(required = true)]
        path: UserPath,
    },

    #[clap(setting = clap::AppSettings::Hidden)]
    Autocomplete {
        #[clap(short)]
        desktop_files: bool,
        #[clap(short)]
        mimes:         bool,
    },
}
