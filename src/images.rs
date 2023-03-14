// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::cmdline::{Images, InstallConfig, Live};
use anyhow::{bail, Context, Result};
use reqwest::Url;
use std::env::current_dir;
use std::fs::{metadata, File};
use std::io::{copy, BufReader, BufWriter, Write};
use std::path::PathBuf;

pub fn download_images(config: &InstallConfig) -> Result<()> {
    match &config.images {
        Images::Artifacts(build) => download_live_images(&Live::from(build)),
        Images::LiveImages(live) => download_live_images(live),
    }
}

fn download_live_images(live: &Live) -> Result<()> {
    download(&live.kernel)?;
    download(&live.initrd)?;
    Ok(())
}

fn download(url: &Url) -> Result<()> {
    let path = PathBuf::from(url.path());
    let path = path
        .file_name()
        .with_context(|| format!("getting filename '{}'", url.path()))?;
    let path = current_dir().context("getting CWD")?.join(path);

    if let Ok(meta) = metadata(&path) {
        println!("{} already exists, size: {}", path.display(), meta.len());
        return Ok(());
    } else if url.scheme() == "file" {
        bail!("No such file: '{}'", path.display());
    }

    println!("Downloadind {}", url);
    let client = reqwest::blocking::ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("building HTTP client")?;
    let mut resp = client
        .get(url.as_ref())
        .send()
        .with_context(|| format!("sending request for '{}'", url))?
        .error_for_status()
        .with_context(|| format!("fetching '{}'", url))?;
    let mut file = File::create(&path)?;
    let mut writer = BufWriter::with_capacity(1024, &mut file);
    copy(&mut BufReader::with_capacity(1024, &mut resp), &mut writer)
        .with_context(|| format!("couldn't copy '{}'", url))?;
    writer
        .flush()
        .with_context(|| format!("couldn't write '{}' to '{:?}'", url, path.display()))?;
    drop(writer);

    Ok(())
}
