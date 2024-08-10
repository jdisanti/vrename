// This file is part of vrename.
// Copyright (C) 2024 John DiSanti.
//
// vrename is free software: you can redistribute it and/or modify it under the terms of
// the GNU General Public License as published by the Free Software Foundation, either version 3 of
// the License, or (at your option) any later version.
//
// vrename is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See
// the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with vrename.
// If not, see <https://www.gnu.org/licenses/>.

use crate::name_file::NameFile;
use anyhow::{anyhow, bail, Result};
use std::{
    fs,
    process::{self, Stdio},
};

mod name_file;

struct Inputs {
    preferred_editor: String,
    file_names: Vec<String>,
}

impl Inputs {
    fn from_env() -> Result<Option<Self>> {
        let preferred_editor = std::env::var("EDITOR")
            .map_err(|_| anyhow!("missing preferred editor (EDITOR) environment variable"))?;
        let file_names = std::env::args().skip(1).collect::<Vec<_>>();

        Ok(if file_names.is_empty() {
            None
        } else {
            Some(Self {
                preferred_editor,
                file_names,
            })
        })
    }
}

fn vrename(inputs: &Inputs) -> Result<()> {
    // Create the temp name file with the names from args
    let name_file = NameFile::new(&inputs.file_names)?;

    // Open that temp file in the preferred editor
    let output = process::Command::new(&inputs.preferred_editor)
        .arg(name_file.path())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|err| anyhow!("failed to run preferred text editor: {err}"))?;
    if !output.status.success() {
        bail!("preferred text editor exited with failure status");
    }

    // Read the names back from the temp file after editing
    let name_map = name_file.read_back()?;

    // Perform the renames
    for (old_name, new_name) in &name_map {
        fs::rename(old_name, new_name)
            .map_err(|err| anyhow!("failed to rename {old_name} to {new_name}: {err}"))?;
        eprintln!("renamed \"{old_name}\" to \"{new_name}\"");
    }

    Ok(())
}

fn do_main() -> Result<()> {
    match Inputs::from_env()? {
        Some(inputs) => vrename(&inputs),
        None => {
            eprintln!("vrename - batch rename files with your preferred text editor");
            eprintln!();
            eprintln!("usage: vrename <file names...>");
            process::exit(0);
        }
    }
}

fn main() {
    if let Err(err) = do_main() {
        eprintln!("failed: {err}");
        process::exit(1);
    }
}
