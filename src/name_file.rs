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

use anyhow::{anyhow, bail, Result};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write},
    path::Path,
};

/// Represents the temp file where name edits occur
pub struct NameFile<'a, S> {
    temp_file: tempfile::NamedTempFile,
    file_names: &'a [S],
}

impl<'a, S: AsRef<str> + std::fmt::Debug> NameFile<'a, S> {
    /// Creates the temp file with the given file names in it
    pub fn new(file_names: &'a [S]) -> Result<Self> {
        let mut temp_file = tempfile::NamedTempFile::new()
            .map_err(|err| anyhow!("failed to open temp file: {err}"))?;
        {
            let mut temp_writer = BufWriter::new(&mut temp_file);
            for file_name in file_names {
                writeln!(temp_writer, "{}", file_name.as_ref())
                    .map_err(|err| anyhow!("failed to write to temp file: {err}"))?;
            }
        }
        Ok(Self {
            temp_file,
            file_names,
        })
    }

    /// Path to the temp file
    pub fn path(&self) -> &Path {
        self.temp_file.path()
    }

    /// Reads the temp file back and maps the old names to the new names
    pub fn read_back(mut self) -> Result<HashMap<String, String>> {
        // Since we never close the file handle, we have to rewind it
        self.temp_file
            .as_file_mut()
            .seek(SeekFrom::Start(0))
            .map_err(|err| anyhow!("failed to rewind temp file: {err}"))?;

        let reader = BufReader::new(&mut self.temp_file);
        let mut new_names = Vec::with_capacity(self.file_names.len());
        for line in reader.lines() {
            // Intended to trim the new line off the end of the line string, but trimming in general seems good
            let line = line
                .map_err(|err| anyhow!("failed to read line back from temp file: {err}"))?
                .trim_ascii()
                .to_string();
            if !line.is_empty() {
                new_names.push(line);
            }
        }
        if new_names.len() != self.file_names.len() {
            eprintln!("old names: {:?}", self.file_names);
            eprintln!("new names: {:?}", new_names);
            bail!("cannot map old names to new names (number of lines in name file changed)");
        }
        Ok(self
            .file_names
            .iter()
            .map(|s| s.as_ref().to_string())
            .zip(new_names)
            .collect::<HashMap<_, _>>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn name_file_mapping() {
        let old_names = vec!["Test 1", "Test 2", "Test 3"];
        let name_file = NameFile::new(&old_names).unwrap();

        let mut file = fs::File::create(name_file.path()).unwrap();
        writeln!(file, "test1").unwrap();
        writeln!(file, "test2").unwrap();
        writeln!(file, "test3").unwrap();
        drop(file);

        let mapping = name_file.read_back().unwrap();
        assert_eq!("test1", mapping.get("Test 1").unwrap());
        assert_eq!("test2", mapping.get("Test 2").unwrap());
        assert_eq!("test3", mapping.get("Test 3").unwrap());
    }

    #[test]
    fn name_file_mapping_trailing_line() {
        let old_names = vec!["Test 1", "Test 2", "Test 3"];
        let name_file = NameFile::new(&old_names).unwrap();

        let mut file = fs::File::create(name_file.path()).unwrap();
        writeln!(file, "test1").unwrap();
        writeln!(file, "test2").unwrap();
        writeln!(file, "test3").unwrap();
        writeln!(file).unwrap(); // trailing line
        drop(file);

        let mapping = name_file.read_back().unwrap();
        assert_eq!("test1", mapping.get("Test 1").unwrap());
        assert_eq!("test2", mapping.get("Test 2").unwrap());
        assert_eq!("test3", mapping.get("Test 3").unwrap());
    }

    #[test]
    fn name_file_mapping_missing_line() {
        let old_names = vec!["Test 1", "Test 2", "Test 3"];
        let name_file = NameFile::new(&old_names).unwrap();

        let mut file = fs::File::create(name_file.path()).unwrap();
        writeln!(file, "test1").unwrap();
        writeln!(file, "test2").unwrap();
        writeln!(file).unwrap(); // trailing line
        drop(file);

        let err = name_file.read_back().err().unwrap();
        assert!(format!("{err}").contains("cannot map old names to new names"));
    }
}
