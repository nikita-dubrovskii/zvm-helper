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
use anyhow::*;
use reqwest::Url;
use std::env::current_dir;
use std::fmt;

#[derive(Debug)]
pub enum CoreOsVariant {
    Fedora,
    RedHat,
}

#[derive(Debug)]
pub struct Config {
    pub zvm: ZvmConfig,
    pub network: NetworkConfig,
    pub target: DiskConfig,
    pub images: ImagesConfig,
}

#[derive(Debug)]
pub struct ZvmConfig {
    pub zvm: String,
    pub ignition: Url,
    pub dfltcc: Option<bool>,
    pub cmdline: Option<String>,
}

#[derive(Debug)]
pub enum ImagesConfig {
    Build(BuildImages),
    Live(LiveImages),
}

#[derive(Debug)]
pub struct BuildImages {
    pub url: Option<Url>,
    pub variant: CoreOsVariant,
    pub version: String,
    pub date: String,
    pub time: Option<String>,
    pub id: Option<u32>,
}

#[derive(Debug)]
pub struct LiveImages {
    pub live_kernel: Url,
    pub live_initrd: Url,
    pub live_rootfs: Url,
}

#[derive(Debug)]
pub enum DiskConfig {
    Dasd(DasdDisk),
    Fba(FbaDisk),
    Scsi(ScsiDisk),
    Multipath(MultipathDisks),
}

#[derive(Debug)]
pub struct DasdDisk {
    pub dasd: String,
}

#[derive(Debug)]
pub struct FbaDisk {
    pub fba: String,
}

#[derive(Debug)]
pub struct ScsiDisk {
    pub scsi: String,
}

#[derive(Debug)]
pub struct MultipathDisks {
    pub scsi: Vec<String>,
}

#[derive(Debug)]
pub struct NetworkConfig {
    pub ip: String,
    pub id: String,
    pub gw: String,
    pub mask: String,
    pub hostname: String,
    pub nic: String,
    pub dhcp: String,
    pub nameserver: String,
    pub znet: String,
}

pub trait InstallTarget {
    fn install_target(&self) -> Result<String>;
}

impl InstallTarget for DasdDisk {
    fn install_target(&self) -> Result<String> {
        ensure!(self.dasd.starts_with("0.0."));
        let target = "/dev/disk/by-path/ccw-".to_string();
        Ok(target + &self.dasd)
    }
}

impl InstallTarget for FbaDisk {
    fn install_target(&self) -> Result<String> {
        let target = "/dev/disk/by-path/ccw-".to_string();
        Ok(target + &self.fba)
    }
}

impl InstallTarget for ScsiDisk {
    fn install_target(&self) -> Result<String> {
        Ok("sda".to_string())
    }
}

impl InstallTarget for MultipathDisks {
    fn install_target(&self) -> Result<String> {
        ensure!(self.scsi.len() > 1);
        Ok("/dev/mapper/mpatha".to_string())
    }
}

impl From<&BuildImages> for LiveImages {
    fn from(images: &BuildImages) -> Self {
        // fedora-coreos-34.20210629.dev.0-live-initramfs.s390x.img
        // fedora-coreos-34.20210629.dev.0-live-kernel-s390x
        // fedora-coreos-34.20210629.dev.0-live-rootfs.s390x.img

        // rhcos-49.84.202106281019-0-live-initramfs.s390x.img
        // rhcos-49.84.202106281019-0-live-kernel-s390x
        // rhcos-49.84.202106281019-0-live-rootfs.s390x.img
        let generate = |image: &str| {
            let name = match images.variant {
                CoreOsVariant::Fedora => format!(
                    "fedora-coreos-{}.{}.dev.{}-live-{}",
                    images.version,
                    images.date,
                    images.id.unwrap_or_default(),
                    image
                ),
                CoreOsVariant::RedHat => format!(
                    "rhcos-{}.{}{}-0-live-{}",
                    images.version,
                    images.date,
                    images.time.as_ref().unwrap(),
                    image
                ),
            };
            match &images.url {
                Some(url) => url
                    .join(&name)
                    .with_context(|| format!("joining '{}' '{}'", url, name)),
                _ => {
                    let path = current_dir().context("getting CWD")?.join(name);
                    let url = path
                        .to_str()
                        .with_context(|| format!("converting '{}' to str", path.display()))?;
                    match Url::from_file_path(url) {
                        Ok(url) => Ok(url),
                        _ => bail!("parsing '{}'", url),
                    }
                }
            }
        };
        LiveImages {
            live_kernel: generate("kernel-s390x").unwrap(),
            live_initrd: generate("initramfs.s390x.img").unwrap(),
            live_rootfs: generate("rootfs.s390x.img").unwrap(),
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Installing CoreOS\n{}\n{}\n{}\n{}\n",
            self.zvm, self.network, self.images, self.target
        )
    }
}

impl fmt::Display for ZvmConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "zVM: {}\n\tIgnition: {}\n\tdfltcc: {:?}\n\tCmdline: {:?}",
            self.zvm, self.ignition, self.dfltcc, self.cmdline
        )
    }
}

impl fmt::Display for NetworkConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Network config:\n\tip: {}\n\tgateway: {}\n\tnetmask: {}\n\thostname: {}\n\tnic: {}\n\tnameserver: {}\n\tznet: {}",
            self.ip, self.gw, self.mask, self.hostname, self.nic, self.nameserver, self.znet
        )
    }
}

impl fmt::Display for LiveImages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Live Images:\n\tkernel: {}\n\tinitrd: {}\n\trootfs: {}",
            self.live_kernel, self.live_initrd, self.live_rootfs
        )
    }
}

impl fmt::Display for ImagesConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Live(images) => images.fmt(f),
            Self::Build(build) => {
                let images = LiveImages::from(build);
                if let Some(ref url) = build.url.as_ref() {
                    write!(
                        f,
                        "Builder: {}\nVariant: {:?}\n{}",
                        url, build.variant, images
                    )
                } else {
                    images.fmt(f)
                }
            }
        }
    }
}

impl fmt::Display for DiskConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dasd(dasd) => write!(f, "Target:\n\tECKD-DASD: {}\n", dasd.dasd),
            Self::Fba(fba) => write!(f, "Target:\n\tEDEV-DASD(FBA): {}\n", fba.fba),
            Self::Scsi(zfcp) => write!(f, "Target:\n\tzFCP: {}\n", zfcp.scsi),
            Self::Multipath(mp) => write!(f, "Target:\n\tMultipath: {:?}\n", mp.scsi),
        }
    }
}
