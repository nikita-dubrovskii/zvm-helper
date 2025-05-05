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

use anyhow::{anyhow, Context};
use chrono::prelude::*;
use clap::{Parser, ValueEnum};
use reqwest::Url;
use std::borrow::Cow;
use std::env::current_dir;
use std::fmt;

#[derive(Debug, Parser)]
#[clap(name = "zvmhelper", version)]
#[clap(args_conflicts_with_subcommands = true)]
#[clap(disable_help_subcommand = true)]
#[clap(help_expected = true)]
pub enum Cmd {
    /// Install zVM using given arguments
    Install(InstallConfig),
}

#[derive(Debug, Parser)]
pub struct InstallConfig {
    /// zVM target
    #[clap(long, short, value_name = "zVM", default_value = "a3e29008")]
    pub zvm: String,

    /// zVM target
    #[clap(long, short, value_name = "IGNITION_CONFIG")]
    pub ignition: String,

    /// dfltcc option
    #[clap(long, value_name = "DFLTCC")]
    pub dfltcc: Option<bool>,

    /// extra kargs
    #[clap(long, short, value_name = "CMDLINE")]
    pub cmdline: Option<String>,

    /// Dasd
    #[clap(long, value_name = "DASD")]
    pub dasd: Option<String>,

    /// Edev
    #[clap(
        long,
        value_name = "EDEV",
        conflicts_with = "dasd",
        conflicts_with = "scsi",
        conflicts_with = "mp"
    )]
    pub edev: Option<String>,

    /// Scsi
    #[clap(
        long,
        value_name = "SCSI",
        conflicts_with = "dasd",
        conflicts_with = "edev",
        conflicts_with = "mp"
    )]
    pub scsi: Option<String>,

    /// Multipath
    #[clap(
        long,
        value_name = "MULTIPATH",
        conflicts_with = "dasd",
        conflicts_with = "scsi",
        conflicts_with = "edev"
    )]
    pub mp: Option<Vec<String>>,

    /// zVM network device (rd.znet)
    #[clap(
        long,
        value_name = "ZNET",
        default_value = "qeth,0.0.bdf0,0.0.bdf1,0.0.bdf2,layer2=1,portno=0"
    )]
    pub znet: String,

    /// Guest ip= karg
    #[clap(
        long,
        value_name = "IP",
        default_value = "172.23.237.227::172.23.0.1:255.255.0.0:coreos:encbdf0:none"
    )]
    pub ip: String,

    /// Guest nameserver= karg
    #[clap(long, value_name = "NAMESERVER", default_value = "172.23.0.1")]
    pub dns: Vec<String>,

    ///Images
    #[clap(subcommand)]
    pub images: Images,
}

#[derive(Debug, Parser)]
pub enum Images {
    /// Set live images
    LiveImages(Live),

    /// Set build artifacts
    Artifacts(Build),
}

#[derive(Debug, Parser)]
pub struct Live {
    /// Base URL for kernel
    #[clap(long, value_name = "VMLINUZ")]
    pub kernel: Url,
    /// Base URL for initrd
    #[clap(long, value_name = "INITRD")]
    pub initrd: Url,
    /// Base URL for rootfs
    #[clap(long, value_name = "ROOTFS")]
    pub rootfs: Url,
}

#[derive(Debug, Clone, ValueEnum)]
#[allow(clippy::upper_case_acronyms)]
pub enum CoreOS {
    FCOS,
    RHCOS,
}

#[derive(Debug, Parser)]
pub struct Build {
    /// Base URL for builder
    #[clap(long, value_name = "URL", default_value = "http://172.23.236.43")]
    pub url: Url,
    /// CoreOS variant
    #[clap(value_enum)]
    #[clap(long, value_name = "VARIANT", default_value = "fcos")]
    pub variant: CoreOS,
    /// CoreOS version
    #[clap(long, value_name = "VERSION", default_value = "37")]
    pub version: String,
    /// Build date
    #[clap(long, value_name = "DATE")]
    pub date: Option<String>,
    /// Build time
    #[clap(long, value_name = "TIME")]
    pub time: Option<String>,
    /// Build id
    #[clap(long, value_name = "ID", default_value = "0")]
    pub id: u32,
}

#[cfg(test)]
mod test {
    use super::*;
    use clap::IntoApp;

    #[test]
    fn clap_app() {
        Cmd::command().debug_assert()
    }
}

impl From<&Build> for Live {
    fn from(images: &Build) -> Self {
        let generate = |image: &str| {
            let date = match images.date.as_ref() {
                Some(v) => Cow::from(v),
                _ => {
                    let now = chrono::Local::now();
                    Cow::from(format!("{}{:02}{:02}", now.year(), now.month(), now.day()))
                }
            };
            let name = match images.variant {
                // fedora-coreos-37.20230314.dev.0-live-
                CoreOS::FCOS => {
                    format!(
                        "fedora-coreos-{}.{}.dev.{}-live-{}",
                        images.version, date, images.id, image
                    )
                }
                // rhcos-413.92.202303141019-0-live-
                CoreOS::RHCOS => {
                    format!(
                        "rhcos-{}.{}{}-0-live-{}",
                        images.version,
                        date,
                        images
                            .time
                            .as_ref()
                            .context("RHCOS artifacts require build time")?,
                        image
                    )
                }
            };
            if images.url.scheme() == "http" {
                images
                    .url
                    .join(&name)
                    .with_context(|| format!("joining '{}' '{}'", images.url, name))
            } else {
                let path = current_dir().context("CWD")?.join(name);
                match Url::from_file_path(&path) {
                    Ok(url) => Ok(url),
                    _ => Err(anyhow!("Building URL from {:?}", path)),
                }
            }
        };
        Live {
            kernel: generate("kernel-s390x").unwrap(),
            initrd: generate("initramfs.s390x.img").unwrap(),
            rootfs: generate("rootfs.s390x.img").unwrap(),
        }
    }
}

impl fmt::Display for InstallConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Installing CoreOS:\nzVM:\t{}\nIP:\t{}\n{}\n",
            self.zvm, self.ip, self.images
        )?;
        write!(
            f,
            "Ignition:\t{}\ndfltcc:\t{:?}\nCmdline:\t{:?}",
            self.ignition, self.dfltcc, self.cmdline
        )?;
        if let Some(dasd) = self.dasd.as_ref() {
            write!(f, "Target:\n\tECKD-DASD: {}\n", dasd)?;
        }
        if let Some(edev) = self.edev.as_ref() {
            write!(f, "Target:\n\tEDEV-DASD(FBA): {}\n", edev)?;
        }
        if let Some(scsi) = self.scsi.as_ref() {
            write!(f, "Target:\n\tzFCP: {}\n", scsi)?;
        }
        if let Some(mp) = self.mp.as_ref() {
            write!(f, "Target:\n\tMultipath: {:?}\n", mp)?;
        }
        Ok(())
    }
}

impl fmt::Display for Live {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Live:\n\tkernel: {}\n\tinitrd: {}\n\trootfs: {}",
            self.kernel, self.initrd, self.rootfs
        )
    }
}

impl fmt::Display for Images {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LiveImages(images) => images.fmt(f),
            Self::Artifacts(build) => Live::from(build).fmt(f),
        }
    }
}
