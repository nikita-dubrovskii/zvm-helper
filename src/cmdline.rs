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

use std::str::FromStr;

use crate::config::*;
use anyhow::*;
use chrono::prelude::*;
use clap::{crate_version, App, AppSettings, Arg, ArgMatches, SubCommand};
use reqwest::Url;

pub fn parse_args() -> Result<Config> {
    let matches = App::new("zvm-helper")
        .version(crate_version!())
        .global_setting(AppSettings::ColorAuto)
        .global_setting(AppSettings::DeriveDisplayOrder)
        .global_setting(AppSettings::UnifiedHelpMessage)
        .subcommand(
            SubCommand::with_name("install")
                .arg(
                    Arg::with_name("zvm")
                        .long("zvm")
                        .help("Set tartget zVM")
                        .required(true)
                        .takes_value(true)
                        .default_value("t8360003"),
                )
                .arg(
                    Arg::with_name("ignition")
                        .long("ignition")
                        .help("Set ignition URL")
                        .required(true)
                        .default_value("http://172.18.10.243/configs/ignition.ign")
                        .takes_value(true),
                )
                .arg(Arg::with_name("dfltcc")
                         .long("dfltcc")
                         .help("Disable dfltcc")
                         .default_value("0")
                         .takes_value(false),
                )
                .arg(Arg::with_name("cmdline")
                         .long("kargs")
                         .help("Add extra kargs")
                         .multiple(true)
                         .takes_value(true)
                         .default_value("random.trust_cpu=on zfcp.allow_lun_scan=0 cio_ignore=all,!condev"),
                )
                .arg(
                    Arg::with_name("dasd")
                        .long("dasd")
                        .help("Set CoreOS installation target to DASD disk")
                        .takes_value(true)
                        .conflicts_with_all(&["fba", "scsi", "multipath"]),
                )
                .arg(
                    Arg::with_name("fba")
                        .long("fba")
                        .help("Set CoreOS installation target to EDEV(FBA) disk")
                        .takes_value(true)
                        .conflicts_with_all(&["dasd", "scsi", "multipath"]),
                )
                .arg(
                    Arg::with_name("scsi")
                        .long("scsi")
                        .help("Set CoreOS installation target to zFCP disk")
                        .takes_value(true)
                        .conflicts_with_all(&["fba", "dasd", "multipath"]),
                )
                .arg(
                    Arg::with_name("multipath")
                        .long("mp")
                        .help("Set CoreOS installation target to Multipath disk")
                        .multiple(true)
                        .min_values(2)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("znet")
                        .long("znet")
                        .help("Set zVM network device (rd.znet)")
                        .takes_value(true)
                        .default_value("qeth,0.0.bdf0,0.0.bdf1,0.0.bdf2,layer2=1,portno=0"),
                )
                .arg(
                    Arg::with_name("ip")
                        .long("ip")
                        .help("Set CoreOs IP address")
                        .takes_value(true)
                        .default_value("172.18.142.3"),
                )
                .arg(
                    Arg::with_name("gateway")
                        .long("gw")
                        .help("Set CoreOs gateway")
                        .takes_value(true)
                        .default_value("172.18.0.1"),
                )
                .arg(
                    Arg::with_name("netmask")
                        .long("nm")
                        .help("Set CoreOs IP netmask")
                        .takes_value(true)
                        .default_value("255.254.0.0"),
                )
                .arg(
                    Arg::with_name("hostname")
                        .long("hostname")
                        .help("Set CoreOs hostname")
                        .takes_value(true)
                        .default_value("coreos"),
                )
                .arg(
                    Arg::with_name("nic")
                        .long("nic")
                        .help("Set CoreOs nic name")
                        .takes_value(true)
                        .default_value("encbdf0"),
                )
                .arg(
                    Arg::with_name("dhcp")
                        .long("dchp")
                        .help("Set CoreOs dhcp mode")
                        .takes_value(true)
                        .default_value("off"),
                )
                .arg(
                    Arg::with_name("nameserver")
                        .long("ns")
                        .help("Set CoreOs nameserver")
                        .takes_value(true)
                        .default_value("172.18.0.1"),
                )
                .subcommand(
                    SubCommand::with_name("set-build-images")
                        .about("Set the CoreOS builder host and image-name's template")
                        .arg(
                            Arg::with_name("url")
                                .long("url")
                                .help("Specify the CoreOS builder URL")
                                .default_value("http://172.18.10.243")
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("variant")
                                .long("variant")
                                .help("Set variant of CoreOS: Fedora or RedHat")
                                .possible_values(&["fcos", "rhcos"])
                                .default_value("fcos")
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("version")
                                .long("version")
                                .help("Set version of CoreOS")
                                .default_value("34")
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("date")
                                .long("date")
                                .help("Set build date of CoreOS")
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("time")
                                .long("time")
                                .help("Set build time of CoreOS")
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("id")
                                .long("id")
                                .help("Set build id of CoreOS")
                                .default_value("0")
                                .takes_value(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("set-live-images")
                        .about("Set the CoreOS live images")
                        .arg(
                            Arg::with_name("kernel")
                                .long("kernel")
                                .help("Set CoreOS live-kernel URL")
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("initrd")
                                .long("initrd")
                                .help("Set CoreOS live-initrd image URL")
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("rootfs")
                                .long("rootfs")
                                .help("Set CoreOS live-rootfs image URL")
                                .takes_value(true),
                        ),
                ),
        )
        .get_matches();

    let matches = matches
        .subcommand_matches("install")
        .context("Missing install")?;

    let images = if let Some(build) = matches.subcommand_matches("set-build-images") {
        parse_build_config(build)?
    } else if let Some(live) = matches.subcommand_matches("set-live-images") {
        parse_live_config(live)?
    } else {
        bail!("Missing CoreOs images options")
    };

    Ok(Config {
        zvm: parse_zvm_config(&matches)?,
        network: parse_network_config(&matches)?,
        target: parse_target_config(&matches)?,
        images,
    })
}

fn parse_zvm_config(am: &ArgMatches) -> Result<ZvmConfig> {
    Ok(ZvmConfig {
        zvm: am
            .value_of("zvm")
            .map(String::from)
            .expect("zvm is missing"),
        ignition: am
            .value_of("ignition")
            .map(Url::parse)
            .expect("ignition is missing")
            .context("Parsing ignition URL")?,
        dfltcc: am
            .value_of("dfltcc")
            .map(|s| s.parse::<bool>().expect("parsing dfltcc")),
        cmdline: am.value_of("cmdline").map(String::from),
    })
}

fn parse_target_config(am: &ArgMatches) -> Result<DiskConfig> {
    if am.is_present("dasd") {
        Ok(DiskConfig::Dasd(DasdDisk {
            dasd: am.value_of("dasd").map(String::from).unwrap(),
        }))
    } else if am.is_present("fba") {
        Ok(DiskConfig::Fba(FbaDisk {
            fba: am.value_of("fba").map(String::from).unwrap(),
        }))
    } else if am.is_present("scsi") {
        Ok(DiskConfig::Scsi(ScsiDisk {
            scsi: am.value_of("scsi").map(String::from).unwrap(),
        }))
    } else if am.is_present("multipath") {
        Ok(DiskConfig::Multipath(MultipathDisks {
            scsi: am
                .values_of("multipath")
                .unwrap()
                .map(String::from)
                .collect(),
        }))
    } else {
        bail!("Installation target is missing")
    }
}

fn parse_network_config(am: &ArgMatches) -> Result<NetworkConfig> {
    Ok(NetworkConfig {
        ip: am.value_of("ip").map(String::from).expect("IP is missing"),
        id: "".to_string(),
        gw: am
            .value_of("gateway")
            .map(String::from)
            .expect("Missing `gateway`"),
        mask: am
            .value_of("netmask")
            .map(String::from)
            .expect("Missing `netmask`"),
        hostname: am
            .value_of("hostname")
            .map(String::from)
            .expect("Missing `hostname`"),
        nic: am.value_of("nic").map(String::from).expect("Missing `nic`"),
        dhcp: am
            .value_of("dhcp")
            .map(String::from)
            .expect("Missing `dhcp`"),
        nameserver: am
            .value_of("nameserver")
            .map(String::from)
            .expect("Missing `nameserver`"),
        znet: am
            .value_of("znet")
            .map(String::from)
            .expect("Missing `znet`"),
    })
}

fn parse_live_config(am: &ArgMatches) -> Result<ImagesConfig> {
    let parse = |s: &str| {
        let url = match std::fs::canonicalize(s).with_context(|| format!("canonicalizing: '{}'", s))
        {
            Ok(path) => match Url::from_file_path(path.as_path()) {
                Ok(url) => url,
                _ => bail!("parsing URL from '{}'", path.display()),
            },
            _ => Url::from_str(s).with_context(|| format!("parsing '{}'", s))?,
        };
        Ok(url)
    };

    Ok(ImagesConfig::Live(LiveImages {
        live_kernel: parse(am.value_of("kernel").expect("Missing `kernel`"))?,
        live_initrd: parse(am.value_of("initrd").expect("Missing `initrd`"))?,
        live_rootfs: parse(am.value_of("rootfs").expect("Missing `rootfs`"))?,
    }))
}

fn parse_build_config(am: &ArgMatches) -> Result<ImagesConfig> {
    let date = {
        let now = chrono::Local::now();
        format!("{}{:02}{:02}", now.year(), now.month(), now.day())
    };
    Ok(ImagesConfig::Build(BuildImages {
        url: am
            .value_of("url")
            .map(Url::parse)
            .transpose()
            .context("Parsing builder url")?,
        variant: match am.value_of("variant").expect("CoreOS variant is missing") {
            "fcos" => CoreOsVariant::Fedora,
            _ => CoreOsVariant::RedHat,
        },
        version: am
            .value_of("version")
            .map(String::from)
            .expect("CoreOS version is missing"),
        date: am.value_of("date").map(String::from).unwrap_or(date),
        time: am.value_of("time").map(String::from),
        id: am
            .value_of("id")
            .map(|s| s.parse::<u32>().expect("converting id")),
    }))
}
