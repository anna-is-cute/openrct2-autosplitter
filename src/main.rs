#![feature(try_blocks)]

use std::{
    collections::BTreeMap,
    path::Path,
};
use std::io::{Cursor, Read};

use anyhow::{Context, Result};
use git2::build::RepoBuilder;
use git2::Repository;
use once_cell::unsync::Lazy;
use pdb::{FallibleIterator, PDB as Pdb, SymbolData};
use reqwest::Client;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use zip::ZipArchive;

const TARGETS: &[&str] = &[
    "gScreenFlags",
    "gScenarioCompletedCompanyValue",
];

const TYPES: &[&str] = &[
    "byte",
    "ulong",
];

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new();

    println!("Fetching releases...");
    let releases: Vec<Release> = client.get("https://api.github.com/repos/OpenRCT2/OpenRCT2/releases")
        .header("User-Agent", "openrct2-autosplitter generator")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .context("could not fetch OpenRCT2 releases")?
        .json()
        .await?;

    let repo_info = Lazy::new(|| {
        println!("Cloning repository...");
        checkout_repo()
    });

    println!("Creating output file...");
    let mut output = File::create("OpenRCT2_header.asl").await?;
    output.write_all(b"// OpenRCT2 Autosplitter by anna\n").await?;
    output.write_all(b"// https://git.anna.lgbt/anna/openrct2-autosplitter\n\n").await?;

    println!("Getting cache...");
    let mut cache = Cache::get().await?;

    println!("Processing releases...");
    let mut hashes = Vec::with_capacity(releases.len());
    let mut offsets = Vec::with_capacity(releases.len());
    for release in releases {
        let res: Result<()> = try {
            let symbols_asset = match release.assets
                .iter()
                .find(|asset| asset.name.contains("symbols-x64"))
            {
                Some(asset) => asset,
                None => Err(anyhow::anyhow!("Release {} is missing debug symbols", release.name))?,
            };

            let binary_asset = match release.assets
                .iter()
                .find(|asset| asset.name.contains("windows-portable-x64"))
            {
                Some(asset) => asset,
                None => Err(anyhow::anyhow!("Release {} is missing binaries", release.name))?,
            };

            let release_ref = match cache.release_refs.get(&release.id) {
                Some(cached) => cached.clone(),
                None => {
                    let info = match &*repo_info {
                        Ok(info) => info,
                        Err(e) => Err(anyhow::anyhow!("could not clone repository: {}", e))?,
                    };
                    let reference = info.repo.find_reference(&format!("refs/tags/{}", release.tag_name))?;
                    let commit = reference.peel_to_commit()?;
                    let sha = commit.id().to_string();
                    format!("{} ({})", release.tag_name, &sha[..7])
                }
            };

            println!("  Downloading binaries for {}", release_ref);
            let (hash, has_dll) = match cache.assets.get(&(binary_asset.id, binary_asset.size)).and_then(|cached| cached.hash.as_ref().map(|hash| (hash, cached.has_dll))) {
                Some((hash, has_dll)) => (hash.clone(), has_dll),
                None => process_binary(&client, binary_asset).await?,
            };
            hashes.push((release_ref.clone(), hash.clone()));

            println!("  Downloading symbols for {}", release_ref);
            let release_offsets = match cache.assets.get(&(symbols_asset.id, symbols_asset.size)).and_then(|cached| cached.offsets.as_ref()) {
                Some(cached) if TARGETS.iter().all(|target| cached.contains_key(&**target)) => cached.clone(),
                _ => process_symbols(&client, symbols_asset).await?,
            };
            offsets.push((release_ref.clone(), has_dll, release_offsets.clone()));

            cache.release_refs.insert(release.id, release_ref);
            cache.assets.insert((binary_asset.id, binary_asset.size), CacheAsset {
                id: symbols_asset.id,
                size: symbols_asset.size,
                hash: Some(hash),
                offsets: None,
                has_dll,
            });
            cache.assets.insert((symbols_asset.id, symbols_asset.size), CacheAsset {
                id: symbols_asset.id,
                size: symbols_asset.size,
                hash: None,
                offsets: Some(release_offsets),
                has_dll,
            });
        };
        if let Err(e) = res {
            eprintln!("could not process release {}: {}", release.name, e);
        }
    }

    println!("  Saving cache...");
    cache.save().await?;

    for (release_ref, has_dll, offsets) in offsets {
        output.write_all(format!("state(\"openrct2\", \"{}\") {{\n", release_ref).as_bytes()).await?;
        for i in 0..TARGETS.len() {
            let name = TARGETS[i];
            let kind = TYPES[i];
            if let Some(offset) = offsets.get(name) {
                if has_dll {
                    output.write_all(format!("    {} {} : \"openrct2.dll\", 0x{:x};\n", kind, name, offset).as_bytes()).await?;
                } else {
                    output.write_all(format!("    {} {} : 0x{:x};\n", kind, name, offset).as_bytes()).await?;
                }
            }
        }
        output.write_all(b"}\n\n").await?;
    }

    output.write_all(b"init {\n    var module = modules.First();\n    string hash = vars.CalcModuleHash(module);\n    switch (hash) {\n").await?;
    for (release_ref, hash) in hashes {
        output.write_all(format!("        case \"{}\":\n            version = \"{}\";\n            break;\n", hash, release_ref).as_bytes()).await?;
    }
    // double newline for concatenation purposes
    output.write_all(b"    }\n}\n\n").await?;

    Ok(())
}

async fn get_asset_zip(client: &Client, asset: &Asset) -> Result<ZipArchive<Cursor<Vec<u8>>>> {
    let zip_bytes = client.get(&asset.browser_download_url).send().await?.bytes().await?;
    let zip = ZipArchive::new(Cursor::new(zip_bytes.to_vec()))?;
    Ok(zip)
}

async fn process_binary(client: &Client, asset: &Asset) -> Result<(String, bool)> {
    let mut zip = get_asset_zip(client, asset).await?;
    let has_dll = zip.file_names().any(|name| name == "openrct2.dll");

    let mut zip_exe = zip.by_name("openrct2.exe")?;
    let mut raw_exe = vec![0; zip_exe.size() as usize];
    zip_exe.read_exact(&mut raw_exe)?;

    Ok((hex::encode(Sha256::digest(&*raw_exe)), has_dll))
}

async fn process_symbols(client: &Client, asset: &Asset) -> Result<BTreeMap<String, u32>> {
    let zip_bytes = client.get(&asset.browser_download_url).send().await?.bytes().await?;
    let mut zip = ZipArchive::new(Cursor::new(&*zip_bytes))?;
    let names: Vec<String> = zip.file_names().map(ToString::to_string).collect();
    let mut offsets = BTreeMap::new();
    for name in names {
        if name.ends_with(".pdb") {
            let mut zip_pdb = zip.by_name(&name)?;
            let mut raw_pdb = vec![0; zip_pdb.size() as usize];
            zip_pdb.read_exact(&mut raw_pdb)?;
            let mut pdb = Pdb::open(Cursor::new(&*raw_pdb))?;
            let addr_map = pdb.address_map()?;
            let globals = pdb.global_symbols()?;
            let mut iter = globals.iter();
            while let Ok(Some(symbol)) = iter.next() {
                match symbol.parse() {
                    Ok(SymbolData::Data(d)) if TARGETS.contains(&&*d.name.to_string()) => {
                        if let Some(offset) = d.offset.to_rva(&addr_map) {
                            offsets.insert(d.name.to_string().to_string(), offset.0);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(offsets)
}

fn checkout_repo() -> Result<RepoInfo> {
    let path = tempfile::tempdir()?;
    let repo = RepoBuilder::new()
        .bare(true)
        .clone("https://github.com/OpenRCT2/OpenRCT2", path.as_ref())?;
    Ok(RepoInfo {
        repo,
        _path: path,
    })
}

#[derive(Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
struct Release {
    id: u64,
    name: String,
    assets: Vec<Asset>,
    tag_name: String,
}

#[derive(Deserialize, Serialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
struct Asset {
    id: u64,
    size: u64,
    browser_download_url: String,
    name: String,
}

#[derive(Default, Deserialize, Serialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
struct Cache {
    release_refs: BTreeMap<u64, String>,
    assets: BTreeMap<(u64, u64), CacheAsset>,
}

impl Cache {
    const FILE_NAME: &'static str = "cache.ron";

    async fn get() -> Result<Self> {
        let path = Path::new(Self::FILE_NAME);
        if path.exists() {
            let file = File::open(Self::FILE_NAME).await?;
            let cache: Self = ron::de::from_reader(file.into_std().await)?;
            Ok(cache)
        } else {
            File::create(path).await?;
            Ok(Self::default())
        }
    }

    async fn save(&self) -> Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(Self::FILE_NAME)
            .await?;
        ron::ser::to_writer_pretty(file.into_std().await, self, PrettyConfig::default())?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
struct CacheAsset {
    id: u64,
    size: u64,
    offsets: Option<BTreeMap<String, u32>>,
    hash: Option<String>,
    has_dll: bool,
}

struct RepoInfo {
    repo: Repository,
    _path: TempDir,
}
