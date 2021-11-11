use crate::{apps::SystemApps, common::Handler, Error, Result, CONFIG};
use colored::Colorize;
use mime::Mime;
use once_cell::sync::Lazy;
use pest::Parser;
use std::{
    collections::{HashMap, VecDeque},
    ffi::OsString,
    io::Read,
    path::PathBuf,
    process::Command,
    str::FromStr,
};

pub static APPS: Lazy<MimeApps> = Lazy::new(|| MimeApps::read().unwrap());
pub static DEFAULT_EDITOR: Lazy<String> =
    Lazy::new(|| std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string()));
pub static CAT_PROG: Lazy<String> = Lazy::new(|| {
    if which::which("bat").is_ok() {
        String::from("bat")
    } else {
        String::from("cat")
    }
});

#[derive(Debug, Default, Clone, pest_derive::Parser)]
#[grammar = "common/ini.pest"]
pub struct MimeApps {
    added_associations: HashMap<Mime, VecDeque<Handler>>,
    default_apps:       HashMap<Mime, VecDeque<Handler>>,
    system_apps:        SystemApps,
}

impl MimeApps {
    pub fn add_handler(&mut self, mime: Mime, handler: Handler) {
        self.default_apps
            .entry(mime)
            .or_default()
            .push_back(handler);
    }

    pub fn set_handler(&mut self, mime: Mime, handler: Handler) {
        self.default_apps.insert(mime, vec![handler].into());
    }

    pub fn remove_handler(&mut self, mime: &Mime) -> Result<()> {
        if let Some(_removed) = self.default_apps.remove(mime) {
            self.save()?;
        }

        Ok(())
    }

    pub fn cat_handler(&self, handler: Handler) -> Result<()> {
        let path = handler.get_entry()?.file_name;
        let to_cat = xdg::BaseDirectories::new()?
            .list_data_files_once("applications")
            .into_iter()
            .find(|f| f.file_name() == Some(&path));

        if let Some(file) = to_cat {
            Command::new(CAT_PROG.to_string())
                .arg(file.display().to_string())
                .status()?;
        } else {
            println!("File does not exist");
        }

        Ok(())
    }

    pub fn edit_handler(&self, handler: Handler) -> Result<()> {
        let path = handler.get_entry()?.file_name;
        let to_edit = xdg::BaseDirectories::new()?
            .list_data_files_once("applications")
            .into_iter()
            .find(|f| f.file_name() == Some(&path));

        if let Some(file) = to_edit {
            Command::new(DEFAULT_EDITOR.to_string())
                .arg(file.display().to_string())
                .status()?;
        } else {
            println!("File does not exist");
        }

        Ok(())
    }

    // TODO: allow multiple
    pub fn get_status(&self, handler: Handler) -> Result<()> {
        use itertools::Itertools;

        let desktop = handler.get_entry()?.file_name;
        let mut enabled = HashMap::<String, OsString>::new();

        let to_rows = |map: &HashMap<String, OsString>| {
            map.iter()
                .sorted()
                .map(|(k, v)| vec![k.to_string(), v.to_str().unwrap().to_string()])
                .collect::<Vec<_>>()
        };

        let table = ascii_table::AsciiTable::default();

        for harr in self.default_apps.clone() {
            let mime = harr.0;
            for h in harr.1 {
                if desktop == h.0 {
                    enabled.insert(
                        // Ord is not satisfied for `ColoredString`
                        mime.clone().to_string().green().bold().to_string(),
                        desktop.clone(),
                    );
                }
            }
        }

        if enabled.is_empty() {
            println!("{}: {}", "disabled".red().bold(), desktop.to_str().unwrap());
        } else {
            table.print(to_rows(&enabled));
        }

        Ok(())
    }

    pub fn get_handler(&self, mime: &Mime) -> Result<Handler> {
        self.get_handler_from_user(mime)
            .or_else(|_| {
                let wildcard = Mime::from_str(&format!("{}/*", mime.type_())).unwrap();
                self.get_handler_from_user(&wildcard)
            })
            .or_else(|_| self.get_handler_from_added_associations(mime))
    }

    fn get_handler_from_user(&self, mime: &Mime) -> Result<Handler> {
        match self.default_apps.get(mime) {
            Some(handlers) if CONFIG.enable_selector && handlers.len() > 1 => {
                let handlers = handlers
                    .into_iter()
                    .map(|h| (h, h.get_entry().unwrap().name))
                    .collect::<Vec<_>>();

                let handler = {
                    let name = CONFIG.select(handlers.iter().map(|h| h.1.clone()))?;

                    handlers
                        .into_iter()
                        .find(|h| h.1 == name)
                        .unwrap()
                        .0
                        .clone()
                };

                Ok(handler)
            },
            Some(handlers) => Ok(handlers.get(0).unwrap().clone()),
            None => Err(Error::NotFound(mime.to_string())),
        }
    }

    fn get_handler_from_added_associations(&self, mime: &Mime) -> Result<Handler> {
        self.added_associations
            .get(mime)
            .map(|h| h.get(0).unwrap().clone())
            .or_else(|| self.system_apps.get_handler(mime))
            .ok_or(Error::NotFound(mime.to_string()))
    }

    pub fn show_handler(&self, mime: &Mime, output_json: bool) -> Result<()> {
        let handler = self.get_handler(mime)?;
        let output = if output_json {
            let entry = handler.get_entry()?;
            let cmd = entry.get_cmd(vec![])?;

            (json::object! {
                handler: handler.to_string(),
                name: entry.name.as_str(),
                cmd: cmd.0 + " " + &cmd.1.join(" "),
            })
            .to_string()
        } else {
            handler.to_string()
        };
        println!("{}", output);
        Ok(())
    }

    pub fn path() -> Result<PathBuf> {
        let mut config = xdg::BaseDirectories::new()?.get_config_home();
        config.push("mimeapps.list");
        Ok(config)
    }

    pub fn read() -> Result<Self> {
        let raw_conf = {
            let mut buf = String::new();
            let exists = std::path::Path::new(&Self::path()?).exists();
            std::fs::OpenOptions::new()
                .write(!exists)
                .create(!exists)
                .read(true)
                .open(Self::path()?)?
                .read_to_string(&mut buf)?;
            buf
        };
        let file = Self::parse(Rule::file, &raw_conf)?.next().unwrap();

        let mut current_section_name = "".to_string();
        let mut conf = Self {
            added_associations: HashMap::default(),
            default_apps:       HashMap::default(),
            system_apps:        SystemApps::populate()?,
        };

        file.into_inner().for_each(|line| {
            match line.as_rule() {
                Rule::section => {
                    current_section_name = line.into_inner().concat();
                },
                Rule::property => {
                    let mut inner_rules = line.into_inner(); // { name ~ "=" ~ value }

                    let name = inner_rules.next().unwrap().as_str();
                    let handlers = {
                        use itertools::Itertools;

                        inner_rules
                            .next()
                            .unwrap()
                            .as_str()
                            .split(";")
                            .filter(|s| !s.is_empty())
                            .unique()
                            .filter_map(|s| Handler::from_str(s).ok())
                            .collect::<VecDeque<_>>()
                    };

                    if !handlers.is_empty() {
                        match (Mime::from_str(name), current_section_name.as_str()) {
                            (Ok(mime), "Added Associations") =>
                                conf.added_associations.insert(mime, handlers),

                            (Ok(mime), "Default Applications") =>
                                conf.default_apps.insert(mime, handlers),
                            _ => None,
                        };
                    }
                },
                _ => {},
            }
        });

        Ok(conf)
    }

    pub fn save(&self) -> Result<()> {
        use itertools::Itertools;
        use std::io::{prelude::*, BufWriter};

        let f = std::fs::OpenOptions::new()
            .read(true)
            .create(true)
            .write(true)
            .truncate(true)
            .open(Self::path()?)?;
        let mut writer = BufWriter::new(f);

        writer.write_all(b"[Added Associations]\n")?;
        for (k, v) in self.added_associations.iter().sorted() {
            writer.write_all(k.essence_str().as_ref())?;
            writer.write_all(b"=")?;
            writer.write_all(v.iter().join(";").as_ref())?;
            writer.write_all(b";\n")?;
        }

        writer.write_all(b"\n[Default Applications]\n")?;
        for (k, v) in self.default_apps.iter().sorted() {
            writer.write_all(k.essence_str().as_ref())?;
            writer.write_all(b"=")?;
            writer.write_all(v.iter().join(";").as_ref())?;
            writer.write_all(b";\n")?;
        }

        writer.flush()?;
        Ok(())
    }

    pub fn print(&self, detailed: bool) -> Result<()> {
        use itertools::Itertools;

        let to_rows = |map: &HashMap<Mime, VecDeque<Handler>>| {
            map.iter()
                .sorted()
                .map(|(k, v)| vec![k.to_string(), v.iter().join(", ")])
                .collect::<Vec<_>>()
        };

        let table = ascii_table::AsciiTable::default();

        if detailed {
            println!("Default Apps");
            table.print(to_rows(&self.default_apps));
            if !self.added_associations.is_empty() {
                println!("Added Associations");
                table.print(to_rows(&self.added_associations));
            }
            println!("System Apps");
            table.print(to_rows(&self.system_apps.0));
        } else {
            table.print(to_rows(&self.default_apps));
        }

        Ok(())
    }

    pub fn list_handlers() -> Result<()> {
        use std::{io::Write, os::unix::ffi::OsStrExt};

        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();

        SystemApps::get_entries()?.for_each(|(_, e)| {
            stdout.write_all(e.file_name.as_bytes()).unwrap();
            stdout.write_all(b"\t").unwrap();
            stdout.write_all(e.name.as_bytes()).unwrap();
            stdout.write_all(b"\n").unwrap();
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_mimes() -> Result<()> {
        let mut user_apps = MimeApps::default();
        user_apps.add_handler(
            Mime::from_str("video/*").unwrap(),
            Handler::assume_valid("mpv.desktop".into()),
        );
        user_apps.add_handler(
            Mime::from_str("video/webm").unwrap(),
            Handler::assume_valid("brave.desktop".into()),
        );

        assert_eq!(
            user_apps
                .get_handler(&Mime::from_str("video/mp4")?)?
                .to_string(),
            "mpv.desktop"
        );
        assert_eq!(
            user_apps
                .get_handler(&Mime::from_str("video/asdf")?)?
                .to_string(),
            "mpv.desktop"
        );

        assert_eq!(
            user_apps
                .get_handler(&Mime::from_str("video/webm")?)?
                .to_string(),
            "brave.desktop"
        );

        Ok(())
    }
}
