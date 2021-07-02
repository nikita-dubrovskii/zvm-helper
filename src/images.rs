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

use crate::config::{ImagesConfig, LiveImages};
use anyhow::*;
use reqwest::Url;
use std::env::current_dir;
use std::fs::{metadata, File};
use std::io::{copy, BufReader, BufWriter, Write};
use std::path::PathBuf;

pub fn download_images(images: &ImagesConfig) -> Result<()> {
    match images {
        ImagesConfig::Build(build) => {
            let live = LiveImages::from(build);
            download_live_images(&live)
        }
        ImagesConfig::Live(live) => download_live_images(live),
    }
}

fn download_live_images(live: &LiveImages) -> Result<()> {
    download(&live.live_kernel)?;
    download(&live.live_rootfs)?;
    download(&live.live_initrd)?;
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
