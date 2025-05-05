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
use anyhow::{anyhow, Context, Result};
use reqwest::Url;
use std::process::Command;

macro_rules! runcmd {
    ($cmd:expr) => (runcmd!($cmd,));
    ($cmd:expr, $($args:expr),*) => {{
        let mut cmd = Command::new($cmd);
        $( cmd.arg($args); )*
        let status = cmd.status().with_context(|| format!("running {:#?}", cmd))?;
        if !status.success() {
            Result::Err(anyhow!("{:#?} failed with {}", cmd, status))
        } else {
            Result::Ok(())
        }
    }}
}

pub fn ipl_zvm_guest(cfg: &InstallConfig) -> Result<()> {
    enable_vmur_dev()?;
    clear(&cfg.zvm)?;
    send(cfg)?;
    println!("Please login to zVM and IPL and manually: '#cp ipl c'");
    Ok(())
}

fn enable_vmur_dev() -> Result<()> {
    runcmd!("modprobe", "vmur")?;
    for id in ["c", "d", "e"] {
        let output = Command::new("cio_ignore")
            .arg("--is-ignored")
            .arg(id)
            .output()
            .with_context(|| format!("running 'cio_ignore --is-ignored {}'", id))?;
        let output = String::from_utf8(output.stdout)?;
        if output.contains("is ignored") {
            runcmd!("cio_ignore", "--remove", id)?;
        }
        runcmd!("chccwdev", "--online", id)?;
    }
    Ok(())
}

fn clear(zvm: &str) -> Result<()> {
    runcmd!("vmcp", "sp", "pun", zvm, "rdr")?;
    runcmd!("vmcp", "pur", zvm, "rdr", "all")
}

fn punch(zvm: &str, target: &str, file: &str) -> Result<()> {
    println!("Copying '{}' to '{}': '{}'", file, zvm, target);
    runcmd!("vmur", "punch", "-r", "-u", zvm, "-N", target, file)
}

fn send(cfg: &InstallConfig) -> Result<()> {
    let url_to_path = |url: &Url| {
        if url.scheme() == "file" {
            match url.to_file_path() {
                Ok(path) => path
                    .to_str()
                    .map(<_>::to_string)
                    .ok_or(anyhow!("getting filename from '{}'", path.display())),
                _ => Err(anyhow!("converting '{}'", url)),
            }
        } else {
            url.path()
                .split('/')
                .last()
                .map(<_>::to_string)
                .ok_or(anyhow!("getting filename from '{}'", url))
        }
    };

    let (kernel, initrd) = match &cfg.images {
        Images::Artifacts(build) => {
            let images = Live::from(build);
            (url_to_path(&images.kernel), url_to_path(&images.initrd))
        }
        Images::LiveImages(images) => (url_to_path(&images.kernel), url_to_path(&images.initrd)),
    };

    let cmdline = parm(cfg);
    let parmfile = "cmdline";
    std::fs::write(parmfile, &cmdline)
        .with_context(|| format!("writing '{}' to '{}'", cmdline, parmfile))?;

    punch(&cfg.zvm, "coreos.kernel", &kernel?)?;
    punch(&cfg.zvm, "coreos.parm", parmfile)?;
    punch(&cfg.zvm, "coreos.initrd", &initrd?)
}

fn parm(cfg: &InstallConfig) -> String {
    let mut s = String::new();
    // network
    s.push_str(&format!(
        "rd.neednet=1 rd.znet={} ip={} {}",
        cfg.znet,
        cfg.ip,
        cfg.dns
            .iter()
            .map(|ns| format!("nameserver={} ", ns))
            .collect::<Vec<String>>()
            .join(" ")
    ));

    // target
    if let Some(dasd) = &cfg.dasd {
        s.push_str(&format!(
            " rd.dasd={} coreos.inst.install_dev=/dev/disk/by-path/ccw-{}",
            dasd, dasd
        ));
    } else if let Some(edev) = &cfg.edev {
        s.push_str(&format!(
            " rd.dasd={} coreos.inst.install_dev=/dev/disk/by-path/ccw-{}",
            edev, edev
        ));
    } else if let Some(scsi) = &cfg.scsi {
        s.push_str(&format!("rd.zfcp={} coreos.inst.install_dev=sda", scsi));
    } else if let Some(mp) = &cfg.mp {
        s.push_str(&format!(
            " rd.multipath=default {} coreos.inst.install_dev=/dev/mapper/mpatha",
            mp.iter()
                .map(|s| format!("rd.zfcp={}", s))
                .collect::<Vec<String>>()
                .join(" "),
        ));
    }

    let rootfs = match &cfg.images {
        Images::Artifacts(b) => Live::from(b).rootfs.to_string(),
        Images::LiveImages(i) => i.rootfs.to_string(),
    };
    s.push_str(&format!(" coreos.inst=yes coreos.inst.insecure=yes coreos.inst.ignition_url={} coreos.live.rootfs_url={}",  
        cfg.ignition, rootfs));

    // dfltcc
    if let Some(dfltcc) = cfg.dfltcc {
        s.push_str(&format!(" dfltcc={}", dfltcc));
    }

    // cmdline
    if let Some(cmdline) = &cfg.cmdline {
        s.push_str(&format!(" {}", cmdline));
    }

    s
}
