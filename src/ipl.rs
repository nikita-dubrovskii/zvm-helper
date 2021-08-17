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

use crate::config::{Config, DiskConfig, ImagesConfig, InstallTarget, LiveImages};
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

pub fn ipl_zvm_guest(cfg: &Config) -> Result<()> {
    enable_vmur_dev()?;
    clear(&cfg.zvm.zvm)?;
    clear(&cfg.zvm.zvm)?; // twice!
    send(cfg)?;
    if let Err(e) = runcmd!("vmcp", "xautolog", &cfg.zvm.zvm, "ipl", "000c") {
        println!("Starting installation failed: {}", e);
        println!("Please login to zVM and IPL and manually: '#cp ipl c'");
    }
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

fn send(cfg: &Config) -> Result<()> {
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
        ImagesConfig::Build(build) => {
            let images = LiveImages::from(build);
            (
                url_to_path(&images.live_kernel),
                url_to_path(&images.live_initrd),
            )
        }
        ImagesConfig::Live(images) => (
            url_to_path(&images.live_kernel),
            url_to_path(&images.live_initrd),
        ),
    };

    let cmdline = parm(cfg)?;
    let parmfile = "cmdline";
    std::fs::write(parmfile, &cmdline)
        .with_context(|| format!("writing '{}' to '{}'", cmdline, parmfile))?;

    punch(&cfg.zvm.zvm, "coreos.kernel", &kernel?)?;
    punch(&cfg.zvm.zvm, "coreos.parm", parmfile)?;
    punch(&cfg.zvm.zvm, "coreos.initrd", &initrd?)
}

fn punch(zvm: &str, target: &str, file: &str) -> Result<()> {
    println!("Copying '{}' to '{}': '{}'", file, zvm, target);
    runcmd!("vmur", "punch", "-r", "-u", zvm, "-N", target, file)
}

fn parm(cfg: &Config) -> Result<String> {
    let mut s = String::new();

    //network
    s.push_str(
        format!(
            "rd.neednet=1 rd.znet={} ip={}:{}:{}:{}:{}:{}:{} nameserver={} ",
            cfg.network.znet,
            cfg.network.ip,
            cfg.network.id,
            cfg.network.gw,
            cfg.network.mask,
            cfg.network.hostname,
            cfg.network.nic,
            cfg.network.dhcp,
            cfg.network.nameserver
        )
        .as_str(),
    );

    //installer
    let rootfs = match &cfg.images {
        ImagesConfig::Build(build) => {
            let images = LiveImages::from(build);
            images.live_rootfs.as_str().to_string()
        }
        ImagesConfig::Live(images) => images.live_rootfs.as_str().to_string(),
    };
    s.push_str(format!("coreos.inst=yes coreos.inst.insecure=yes coreos.inst.ignition_url={} coreos.live.rootfs_url={} ", cfg.zvm.ignition, rootfs).as_str());
    match &cfg.target {
        DiskConfig::Dasd(dasd) => s.push_str(
            format!(
                "rd.dasd={} coreos.inst.install_dev={}",
                dasd.dasd,
                dasd.install_target()?
            )
            .as_str(),
        ),
        DiskConfig::Fba(fba) => s.push_str(
            format!(
                "rd.dasd={} coreos.inst.install_dev={}",
                fba.fba,
                fba.install_target()?
            )
            .as_str(),
        ),
        DiskConfig::Scsi(scsi) => s.push_str(
            format!(
                "rd.zfcp={} coreos.inst.install_dev={}",
                scsi.scsi,
                scsi.install_target()?
            )
            .as_str(),
        ),
        DiskConfig::Multipath(mp) => s.push_str(
            format!(
                "rd.multipath=default {} coreos.inst.install_dev={}",
                mp.scsi
                    .iter()
                    .map(|s| format!("rd.zfcp={}", s))
                    .collect::<Vec<String>>()
                    .join(" "),
                mp.install_target()?
            )
            .as_str(),
        ),
    };

    if let Some(dfltcc) = cfg.zvm.dfltcc {
        s.push_str(format!(" dfltcc={}", dfltcc).as_str());
    }

    if let Some(cmdline) = &cfg.zvm.cmdline {
        s.push_str(format!(" {}", cmdline).as_str());
    }

    Ok(s)
}
