use crate::Result;

/// Send notifications
pub(crate) fn notify(title: &str, msg: &str) -> Result<()> {
    std::process::Command::new("notify-send")
        .args(&["-t", "10000", title, msg])
        .spawn()?;
    Ok(())
}

/// Interactively select one of the given items within the TUI
#[allow(unused)]
pub(crate) fn select_item<'a, S: AsRef<str>>(prompt: &'a str, items: &'a [S]) -> Option<usize> {
    // Build sorted list of string references as items
    let items = items.iter().map(AsRef::as_ref).collect::<Vec<_>>();
    // items.sort_unstable();

    loop {
        // Print options and prompt
        items
            .iter()
            .enumerate()
            .for_each(|(i, item)| eprintln!("{}: {}", i + 1, item));
        eprint!("{} (number/empty): ", prompt);

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("failed to read user input from stdin");

        // If empty, we selected none
        if input.trim().is_empty() {
            return None;
        }

        // Try to parse number, select item, or show error and retry
        match input.trim().parse::<usize>().ok() {
            Some(n) if n > 0 && n <= items.len() =>
            // return Some(items[n - 1].to_string().split(" -- ").collect::<Vec<_>>()[1].into()),
                return Some(n),
            _ => {
                eprintln!("invalid selection input");
                eprintln!();
            },
        }
    }
}

#[cfg(all(feature = "skim-select", unix))]
pub(crate) mod skim {

    use crate::common::DesktopEntry;
    use skim::{
        prelude::{SkimItemReceiver, SkimItemSender, SkimOptionsBuilder},
        AnsiString, DisplayContext, Skim, SkimItem,
    };

    use std::{borrow::Cow, sync::Arc};

    /// Display selection with the `skim` library
    pub(crate) fn skim_select(items: SkimItemReceiver, prompt: &str) -> Option<String> {
        let mut skim_args = Vec::new();
        let default_height = String::from("50%");
        let default_margin = String::from("0%");
        let default_layout = String::from("default");
        // This is the default settings within the skim 'src/' folder
        let default_theme = String::from(
            "matched:108,matched_bg:0,current:254,current_bg:236,current_match:151,\
             current_match_bg:236,spinner:148,info:144,prompt:110,cursor:161,selected:168,header:\
             109,border:59",
        );

        skim_args.extend(
            std::env::var("SKIM_DEFAULT_OPTIONS")
                .ok()
                .and_then(|val| shlex::split(&val))
                .unwrap_or_default(),
        );

        let prompt = format!("{}: ", prompt);
        let options = SkimOptionsBuilder::default()
            .prompt(Some(&prompt))
            .margin(Some(
                skim_args
                    .iter()
                    .find(|arg| arg.contains("--margin") && *arg != &"--margin".to_string())
                    .unwrap_or_else(|| {
                        skim_args
                            .iter()
                            .position(|arg| arg.contains("--margin"))
                            .map_or(&default_margin, |pos| &skim_args[pos + 1])
                    }),
            ))
            .height(Some(
                skim_args
                    .iter()
                    .find(|arg| arg.contains("--height") && *arg != &"--height".to_string())
                    .unwrap_or_else(|| {
                        skim_args
                            .iter()
                            .position(|arg| arg.contains("--height"))
                            .map_or(&default_height, |pos| &skim_args[pos + 1])
                    }),
            ))
            .layout(
                skim_args
                    .iter()
                    .find(|arg| arg.contains("--layout") && *arg != &"--layout".to_string())
                    .unwrap_or_else(|| {
                        skim_args
                            .iter()
                            .position(|arg| arg.contains("--layout"))
                            .map_or(&default_layout, |pos| &skim_args[pos + 1])
                    }),
            )
            .color(Some(
                skim_args
                    .iter()
                    .find(|arg| {
                        arg.contains("--color")
                            && *arg != &"--color".to_string()
                            && !arg.contains("{}")
                    })
                    .unwrap_or_else(|| {
                        skim_args
                            .iter()
                            .position(|arg| arg.contains("--color"))
                            .map_or(&default_theme, |pos| &skim_args[pos + 1])
                    }),
            ))
            .bind(
                skim_args
                    .iter()
                    .filter(|arg| arg.contains("--bind"))
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )
            .reverse(skim_args.iter().any(|arg| arg.contains("--reverse")))
            .tac(skim_args.iter().any(|arg| arg.contains("--tac")))
            .nosort(skim_args.iter().any(|arg| arg.contains("--no-sort")))
            .inline_info(skim_args.iter().any(|arg| arg.contains("--inline-info")))
            .multi(false)
            .build()
            .unwrap();

        // Run skim, get output, abort on close
        let output = Skim::run_with(&options, Some(items))?;
        if output.is_abort {
            return None;
        }

        // Get the first selected, and return
        output.selected_items.get(0).map(|i| i.output().to_string())
    }

    /// Wrapper for selecting [`DesktopEntry`](crate::common::DesktopEntry) with
    /// [`Skim`](skim::Skim)
    pub(crate) struct SkimDesktop(DesktopEntry);

    impl From<DesktopEntry> for SkimDesktop {
        fn from(desktop: DesktopEntry) -> Self {
            Self(desktop)
        }
    }

    impl SkimItem for SkimDesktop {
        fn display(&self, _: DisplayContext) -> AnsiString {
            format!(
                "{} -- {}",
                self.0.name,
                self.0.file_name.to_str().unwrap().replace("\"", "")
            )
            .into()
        }

        fn text(&self) -> Cow<str> {
            self.0.file_name.to_str().unwrap().replace("\"", "").into()
        }

        fn output(&self) -> Cow<str> {
            self.0.file_name.to_str().unwrap().replace("\"", "").into()
        }
    }

    /// Generate `SkimDesktop` items from given slice of `DesktopEntry`
    fn skim_desktop_items(desktops: &[DesktopEntry]) -> SkimItemReceiver {
        skim_items(
            desktops
                .iter()
                .cloned()
                .map(Into::into)
                .collect::<Vec<SkimDesktop>>(),
        )
    }

    /// Create `SkimItemReceiver` from given array
    fn skim_items<I: SkimItem>(items: Vec<I>) -> SkimItemReceiver {
        let (tx, rx): (SkimItemSender, SkimItemReceiver) = skim::prelude::bounded(items.len());

        for g in items {
            let _drop = tx.send(Arc::new(g));
        }

        rx
    }

    /// Select a [`DesktopEntry`](crate::common::DesktopEntry) (public
    /// interface)
    pub(crate) fn skim_select_item(desktops: &[DesktopEntry]) -> Option<&DesktopEntry> {
        // Return if theres just one to choose
        if desktops.len() == 1 {
            return Some(&desktops[0]);
        }

        let items = skim_desktop_items(desktops);
        let selected = skim_select(items, "Select desktop: ")?;

        Some(
            desktops
                .iter()
                .find(|e| e.file_name.to_str().unwrap().replace("\"", "") == selected)
                .unwrap(),
        )
    }
}
